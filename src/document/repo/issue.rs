//! `draft → issued`: validation, exchange rate, numbering, snapshots, totals.

use chrono::{DateTime, Datelike, FixedOffset, NaiveDate};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, Set, TransactionTrait,
};
use uuid::Uuid;

use anyhow::Context as _;

use super::{advance_sources, context, credit, lines, query, view, write};
use crate::cnb::{self, CnbClient};
use crate::contact::entity::contact;
use crate::document::advance::{self, AdvanceCtx};
use crate::document::compute::{Evaluated, Params};
use crate::document::entity::document;
use crate::document::handlers::dto::{BankSnapshot, PartySnapshot};
use crate::document::handlers::input;
use crate::document::line::{LineData, PaymentMethod, Status, VatMode};
use crate::error::{AppError, FieldErrors};
use crate::pdf::{PdfService, archive};
use crate::settings::doc_type::DocType;
use crate::settings::entity::{bank_account, company};
use crate::settings::repo::{company as company_repo, number_series};

/// The exchange rate an issued document uses.
pub struct Rate {
    pub rate: Option<Decimal>,
    pub date: Option<NaiveDate>,
    pub source: Option<&'static str>,
}

/// CZK: no rate. Otherwise the manual rate, else ČNB for `date`; ČNB failing
/// or not listing the currency means the user has to enter the rate
/// (`exchangeRate: required`).
pub async fn fetch_rate(
    db: &DatabaseConnection,
    cnb: &CnbClient,
    currency: &str,
    manual: Option<Decimal>,
    date: NaiveDate,
    today: NaiveDate,
) -> Result<Rate, AppError> {
    if currency == "CZK" {
        return Ok(Rate {
            rate: None,
            date: None,
            source: None,
        });
    }
    if let Some(r) = manual {
        return Ok(Rate {
            rate: Some(r),
            date: None,
            source: Some("manual"),
        });
    }
    match cnb::repo::rate(db, cnb, currency, date, today).await {
        Ok(r) => Ok(Rate {
            rate: Some(r.rate),
            date: Some(r.date),
            source: Some("cnb"),
        }),
        Err(AppError::NotFound | AppError::CnbUnavailable(_)) => {
            Err(AppError::field("exchangeRate", "required"))
        }
        Err(other) => Err(other),
    }
}

/// Everything resolved before the transaction (incl. the ČNB call, which must
/// not run while the row is locked).
struct Prepared {
    doc_type: DocType,
    seen_updated_at: DateTime<FixedOffset>,
    lines: Vec<LineData>,
    customer: contact::Model,
    bank: Option<bank_account::Model>,
    company: company::Model,
    rate: Rate,
    evaluated: Evaluated,
}

/// Issue the draft and archive its PDF in the same transaction: a render
/// failure leaves the draft (and the number counter) untouched.
pub async fn issue(
    db: &DatabaseConnection,
    cnb: &CnbClient,
    pdf: &PdfService,
    id: Uuid,
    today: NaiveDate,
) -> Result<(), AppError> {
    let doc = query::find(db, id).await?;
    if view::status(&doc)? != Status::Draft {
        return Err(AppError::InvalidState);
    }
    let doc_type = context::existing(&doc)?.doc_type;
    let mut lines = query::load_lines(db, id).await?;
    let (customer, bank) = check(db, &doc, doc_type, &lines).await?;
    let rate = if doc_type == DocType::CreditNote {
        Rate {
            rate: doc.exchange_rate,
            date: None,
            source: doc.exchange_rate.map(|_| "original"),
        }
    } else {
        let date = doc.tax_point_date.unwrap_or(doc.issue_date);
        fetch_rate(db, cnb, &doc.currency, doc.exchange_rate, date, today).await?
    };
    let vat_mode = VatMode::parse(&doc.vat_mode).ok_or(AppError::field("vatMode", "invalid"))?;
    let params = Params {
        vat_mode,
        is_czk: doc.currency == "CZK",
        exchange_rate: rate.rate,
        round_total: doc.round_total,
    };
    // Re-resolve deductions: the stored amounts are a draft-time snapshot.
    let sources = advance_sources::load(db, &advance::referenced_ids(&lines)).await?;
    let adv = AdvanceCtx {
        doc_type,
        vat_mode,
        document_id: Some(id),
        related_document_id: doc.related_document_id,
        contact_id: doc.contact_id,
        currency: &doc.currency,
        locale: &doc.locale,
        sources: &sources,
    };
    let evaluated = input::evaluate(&mut lines, params, &adv)?;
    let prepared = Prepared {
        doc_type,
        lines,
        seen_updated_at: doc.updated_at,
        customer,
        bank,
        company: company_repo::get(db).await?,
        rate,
        evaluated,
    };
    let txn = db.begin().await?;
    let stored = match issue_in(&txn, id, prepared, pdf).await {
        Ok(stored) => stored,
        Err(e) => {
            if let Err(r) = txn.rollback().await {
                tracing::warn!(error = %r, "rollback issue");
            }
            return Err(e);
        }
    };
    if let Err(e) = txn.commit().await {
        if let Some(rel) = stored {
            pdf.remove(&rel).await;
        }
        return Err(e.into());
    }
    Ok(())
}

/// The issue validations (422 with every failing field).
async fn check(
    db: &DatabaseConnection,
    doc: &document::Model,
    doc_type: DocType,
    lines: &[LineData],
) -> Result<(contact::Model, Option<bank_account::Model>), AppError> {
    let mut e = FieldErrors::new();
    let customer = match doc.contact_id {
        Some(cid) => contact::Entity::find_by_id(cid).one(db).await?,
        None => None,
    };
    if customer.is_none() {
        e.add("contactId", "required");
    }
    if !lines.iter().any(|l| matches!(l, LineData::Item(_))) {
        e.add("lines", "required");
    }
    if doc.due_date < doc.issue_date {
        e.add("dueDate", "invalid");
    }
    match (doc_type, doc.tax_point_date) {
        (DocType::Proforma, Some(_)) => e.add("taxPointDate", "invalid"),
        (DocType::Proforma, None) | (_, Some(_)) => {}
        (_, None) => e.add("taxPointDate", "required"),
    }
    if doc_type == DocType::CreditNote && doc.correction_reason.is_none() {
        e.add("correctionReason", "required");
    }
    let bank = match doc.bank_account_id {
        Some(bid) => context::bank_account(db, bid).await?,
        None => None,
    };
    match &bank {
        Some(b) if b.currency != doc.currency => e.add("bankAccountId", "invalid"),
        None if doc.payment_method == PaymentMethod::BankTransfer.as_str() => {
            e.add("bankAccountId", "required")
        }
        _ => {}
    }
    e.into_result()?;
    let customer = customer.ok_or(AppError::field("contactId", "required"))?;
    Ok((customer, bank))
}

/// The last 10 digits of the number.
pub fn variable_symbol(number: &str) -> String {
    let digits: Vec<char> = number.chars().filter(char::is_ascii_digit).collect();
    digits[digits.len().saturating_sub(10)..].iter().collect()
}

pub(crate) fn supplier(c: &company::Model) -> PartySnapshot {
    PartySnapshot {
        name: c.name.clone(),
        ico: c.ico.clone(),
        dic: c.dic.clone(),
        street: c.street.clone(),
        city: c.city.clone(),
        zip: c.zip.clone(),
        country: c.country.clone(),
        registration: c.registration.clone(),
        vat_payer: Some(c.vat_payer),
        email: c.email.clone(),
        phone: c.phone.clone(),
        web: c.web.clone(),
    }
}

pub(crate) fn customer(c: &contact::Model) -> PartySnapshot {
    PartySnapshot {
        name: c.name.clone(),
        ico: c.ico.clone(),
        dic: c.dic.clone(),
        street: c.street.clone(),
        city: c.city.clone(),
        zip: c.zip.clone(),
        country: c.country.clone(),
        registration: None,
        vat_payer: None,
        email: c.email.clone(),
        phone: c.phone.clone(),
        web: None,
    }
}

fn json<T: serde::Serialize>(v: &T) -> Result<serde_json::Value, AppError> {
    Ok(serde_json::to_value(v).map_err(anyhow::Error::from)?)
}

async fn issue_in(
    txn: &DatabaseTransaction,
    id: Uuid,
    p: Prepared,
    pdf: &PdfService,
) -> Result<Option<String>, AppError> {
    let doc = query::lock(txn, id).await?;
    if view::status(&doc)? != Status::Draft {
        return Err(AppError::InvalidState);
    }
    // Edited between validation and the lock: what we validated is stale.
    if doc.updated_at != p.seen_updated_at {
        return Err(AppError::Conflict("document changed while issuing".into()));
    }
    let totals = p.evaluated.totals;
    advance_sources::lock_and_recheck(txn, id, &p.lines).await?;
    if p.doc_type == DocType::CreditNote {
        let invoice = doc
            .related_document_id
            .context("credit note without an invoice")?;
        credit::check_cap(txn, id, invoice, &totals).await?;
    }
    let (number, seq) =
        number_series::allocate_number(txn, p.doc_type, doc.issue_date.year()).await?;
    let mut row: document::ActiveModel = doc.clone().into();
    row.status = Set(Status::Issued.as_str().into());
    row.variable_symbol = Set(Some(
        doc.variable_symbol
            .clone()
            .unwrap_or_else(|| variable_symbol(&number)),
    ));
    row.number = Set(Some(number));
    row.number_year = Set(Some(doc.issue_date.year()));
    row.number_seq = Set(Some(seq));
    row.exchange_rate = Set(p.rate.rate);
    row.exchange_rate_date = Set(p.rate.date);
    row.exchange_rate_source = Set(p.rate.source.map(str::to_string));
    row.supplier_snapshot = Set(Some(json(&supplier(&p.company))?));
    row.customer_snapshot = Set(Some(json(&customer(&p.customer))?));
    row.bank_snapshot = Set(p
        .bank
        .as_ref()
        .map(|b| {
            json(&BankSnapshot {
                account_number: b.account_number.clone(),
                iban: b.iban.clone(),
                bic: b.bic.clone(),
            })
        })
        .transpose()?);
    write::apply_totals(&mut row, &totals);
    row.updated_at = Set(chrono::Utc::now().into());
    row.update(txn).await?;
    if !advance::referenced_ids(&p.lines).is_empty() {
        lines::replace(txn, id, &p.lines).await?;
    }
    lines::replace_recap(txn, id, &totals).await?;
    // The render runs while the document row (and counter) stay locked, up to
    // the 60 s mdcast timeout: accepted for a single-user app, because it is
    // what makes "mdcast down → nothing issued, no number used" atomic.
    archive::archive_in(txn, pdf, id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_symbol_keeps_last_ten_digits() {
        assert_eq!(variable_symbol("20260001"), "20260001");
        assert_eq!(variable_symbol("FV-2026/0042"), "20260042");
        assert_eq!(variable_symbol("123456789012"), "3456789012");
    }
}

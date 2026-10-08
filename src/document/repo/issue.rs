//! `draft → issued`: validation, exchange rate, numbering, snapshots, totals.

use chrono::{DateTime, Datelike, FixedOffset, NaiveDate};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, Set, TransactionTrait,
};
use uuid::Uuid;

use super::{context, lines, query, view, write};
use crate::cnb::{self, CnbClient};
use crate::contact::entity::contact;
use crate::document::compute::{self, Evaluated, Params};
use crate::document::entity::document;
use crate::document::handlers::dto::{BankSnapshot, PartySnapshot};
use crate::document::line::{LineData, PaymentMethod, Status, VatMode};
use crate::error::{AppError, FieldErrors};
use crate::settings::doc_type::DocType;
use crate::settings::entity::{bank_account, company};
use crate::settings::repo::{company as company_repo, number_series};

/// The exchange rate the issued document uses.
struct Rate {
    rate: Option<Decimal>,
    date: Option<NaiveDate>,
    source: Option<&'static str>,
}

/// Everything resolved before the transaction (incl. the ČNB call, which must
/// not run while the row is locked).
struct Prepared {
    seen_updated_at: DateTime<FixedOffset>,
    customer: contact::Model,
    bank: Option<bank_account::Model>,
    company: company::Model,
    rate: Rate,
    evaluated: Evaluated,
}

pub async fn issue(
    db: &DatabaseConnection,
    cnb: &CnbClient,
    id: Uuid,
    today: NaiveDate,
) -> Result<(), AppError> {
    let doc = query::find(db, id).await?;
    if view::status(&doc)? != Status::Draft {
        return Err(AppError::InvalidState);
    }
    let lines = query::load_lines(db, id).await?;
    let (customer, bank) = check(db, &doc, &lines).await?;
    let rate = exchange_rate(db, cnb, &doc, today).await?;
    let vat_mode = VatMode::parse(&doc.vat_mode).ok_or(AppError::field("vatMode", "invalid"))?;
    let params = Params {
        vat_mode,
        is_czk: doc.currency == "CZK",
        exchange_rate: rate.rate,
        round_total: doc.round_total,
    };
    let evaluated = compute::evaluate(&lines, params).map_err(AppError::Validation)?;
    let prepared = Prepared {
        seen_updated_at: doc.updated_at,
        customer,
        bank,
        company: company_repo::get(db).await?,
        rate,
        evaluated,
    };
    Ok(db
        .transaction(|txn| Box::pin(issue_in(txn, id, prepared)))
        .await?)
}

/// The issue validations (422 with every failing field).
async fn check(
    db: &DatabaseConnection,
    doc: &document::Model,
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
    if doc.tax_point_date.is_none() {
        e.add("taxPointDate", "required");
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

/// Manual rate wins; otherwise ČNB for the tax point date. ČNB failing or not
/// listing the currency means the user has to enter the rate.
async fn exchange_rate(
    db: &DatabaseConnection,
    cnb: &CnbClient,
    doc: &document::Model,
    today: NaiveDate,
) -> Result<Rate, AppError> {
    if doc.currency == "CZK" {
        return Ok(Rate {
            rate: None,
            date: None,
            source: None,
        });
    }
    if let Some(r) = doc.exchange_rate {
        return Ok(Rate {
            rate: Some(r),
            date: None,
            source: Some("manual"),
        });
    }
    let date = doc.tax_point_date.unwrap_or(doc.issue_date);
    match cnb::repo::rate(db, cnb, &doc.currency, date, today).await {
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

/// The last 10 digits of the number.
pub fn variable_symbol(number: &str) -> String {
    let digits: Vec<char> = number.chars().filter(char::is_ascii_digit).collect();
    digits[digits.len().saturating_sub(10)..].iter().collect()
}

fn supplier(c: &company::Model) -> PartySnapshot {
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
    }
}

fn customer(c: &contact::Model) -> PartySnapshot {
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
    }
}

fn json<T: serde::Serialize>(v: &T) -> Result<serde_json::Value, AppError> {
    Ok(serde_json::to_value(v).map_err(anyhow::Error::from)?)
}

async fn issue_in(txn: &DatabaseTransaction, id: Uuid, p: Prepared) -> Result<(), AppError> {
    let doc = query::lock(txn, id).await?;
    if view::status(&doc)? != Status::Draft {
        return Err(AppError::InvalidState);
    }
    // Edited between validation and the lock: what we validated is stale.
    if doc.updated_at != p.seen_updated_at {
        return Err(AppError::Conflict("document changed while issuing".into()));
    }
    let (number, seq) =
        number_series::allocate_number(txn, DocType::Invoice, doc.issue_date.year()).await?;
    let totals = p.evaluated.totals;
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
    lines::replace_recap(txn, id, &totals).await?;
    Ok(())
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

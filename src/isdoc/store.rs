//! Confirm: one analysed entry → one stored document, in its own transaction
//! (contact, document, lines, recap, original PDF, payment).

use chrono::Datelike;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, Set,
    Statement, TransactionTrait,
};
use uuid::Uuid;

use super::lookup;
use super::model::Party;
use super::plan::{Plan, snapshot};
use crate::contact::entity::contact;
use crate::document::entity::{document, payment};
use crate::document::line::Status;
use crate::document::repo::issue::Rate;
use crate::document::repo::{lines, original, payments, write};
use crate::error::{AppError, number_violation};
use crate::pdf::{PdfService, storage};
use crate::settings::doc_type::{DocType, ISSUED, RECEIVED};
use crate::settings::entity::bank_account;
use crate::settings::repo::number_series;
use crate::validation::normalize_iban;

/// Batch options of `confirm`.
#[derive(Debug, Clone)]
pub struct Options {
    pub mark_paid: bool,
    pub category_id: Option<Uuid>,
    pub vat_deductible: bool,
}

/// Entry failure codes of `confirm`.
pub const DUPLICATE: &str = "duplicate";
pub const NUMBER_TAKEN: &str = "number_taken";

fn json<T: serde::Serialize>(v: &T) -> Result<serde_json::Value, AppError> {
    Ok(serde_json::to_value(v).map_err(anyhow::Error::from)?)
}

async fn contact_for<C: ConnectionTrait>(db: &C, p: &Party) -> Result<Uuid, AppError> {
    if let Some(c) = lookup::contact(db, p).await? {
        return Ok(c.id);
    }
    let now = chrono::Utc::now().into();
    let id = Uuid::new_v4();
    contact::ActiveModel {
        id: Set(id),
        name: Set(p.name.clone()),
        ico: Set(p.ico.clone()),
        dic: Set(p.dic.clone()),
        street: Set(p.street.clone()),
        city: Set(p.city.clone()),
        zip: Set(p.zip.clone()),
        country: Set(p.country.clone()),
        email: Set(p.email.clone()),
        phone: Set(p.phone.clone()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(id)
}

/// Our bank account named by the ISDOC payment details.
async fn bank_account<C: ConnectionTrait>(db: &C, plan: &Plan) -> Result<Option<Uuid>, AppError> {
    let Some(bank) = &plan.bank else {
        return Ok(None);
    };
    let wanted = [bank.iban.as_deref(), bank.account_number.as_deref()];
    let wanted: Vec<String> = wanted.into_iter().flatten().map(normalize_iban).collect();
    Ok(bank_account::Entity::find()
        .all(db)
        .await?
        .into_iter()
        .find(|b| {
            [b.iban.as_deref(), b.account_number.as_deref()]
                .into_iter()
                .flatten()
                .any(|x| wanted.contains(&normalize_iban(x)))
        })
        .map(|b| b.id))
}

/// Payable types; a DDPP takes no payment.
fn payable(t: DocType) -> bool {
    t != DocType::AdvanceTaxDoc
}

struct Stored {
    id: Uuid,
    file: Option<String>,
}

async fn insert(
    txn: &DatabaseTransaction,
    pdf: &PdfService,
    plan: &Plan,
    file: Option<&[u8]>,
    rate: &Rate,
    opts: &Options,
) -> Result<Stored, AppError> {
    // Serialise imports of the same document and re-check under the lock: no
    // unique index covers received documents (supplier + supplier number).
    txn.query_one(Statement::from_sql_and_values(
        txn.get_database_backend(),
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
        [lookup::identity(plan).into()],
    ))
    .await?;
    if lookup::duplicate(txn, plan).await? {
        return Err(AppError::Conflict(DUPLICATE.into()));
    }
    let id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let issued = plan.direction == ISSUED;
    let contact_id = match plan.counterparty() {
        Some(p) => Some(contact_for(txn, p).await?),
        None => None,
    };
    let related = lookup::related(txn, plan).await?;
    let received_date = plan.tax_point_date.unwrap_or(plan.issue_date);
    let (number, year, seq) = if issued {
        (plan.number.clone(), plan.issue_date.year(), None)
    } else {
        let year = received_date.year();
        let (n, seq) =
            number_series::allocate_number(txn, plan.doc_type.series(RECEIVED), year).await?;
        (n, year, Some(seq))
    };
    let mut row = document::ActiveModel {
        id: Set(id),
        direction: Set(plan.direction.into()),
        doc_type: Set(plan.doc_type.as_str().into()),
        status: Set(Status::Issued.as_str().into()),
        number: Set(Some(number)),
        number_year: Set(Some(year)),
        number_seq: Set(seq),
        imported: Set(issued),
        contact_id: Set(contact_id),
        issue_date: Set(plan.issue_date),
        tax_point_date: Set(plan.tax_point_date),
        due_date: Set(plan.due_date),
        currency: Set(plan.currency.clone()),
        exchange_rate: Set(rate.rate),
        exchange_rate_date: Set(rate.date),
        exchange_rate_source: Set(rate.source.map(str::to_string)),
        locale: Set("cs".into()),
        vat_mode: Set(plan.vat_mode.as_str().into()),
        bank_account_id: Set(if issued {
            bank_account(txn, plan).await?
        } else {
            None
        }),
        payment_method: Set(plan.payment_method.as_str().into()),
        variable_symbol: Set(plan.variable_symbol.clone()),
        constant_symbol: Set(plan.constant_symbol.clone()),
        header_note: Set(plan.note.clone().filter(|_| issued)),
        internal_note: Set(plan.note.clone().filter(|_| !issued)),
        // As received create: no rounding flag; it is CZK-only for issued ones.
        round_total: Set(issued && plan.currency == "CZK" && !plan.totals.rounding.is_zero()),
        supplier_snapshot: Set(Some(json(&snapshot(&plan.supplier, Some(plan.vat_payer)))?)),
        customer_snapshot: Set(match (&plan.customer, issued) {
            (Some(c), true) => Some(json(&snapshot(c, None))?),
            _ => None,
        }),
        bank_snapshot: Set(match (&plan.bank, issued) {
            (Some(b), true) => Some(json(b)?),
            _ => None,
        }),
        paid: Set(Decimal::ZERO),
        related_document_id: Set(related),
        supplier_number: Set((!issued).then(|| plan.number.clone())),
        received_date: Set((!issued).then_some(received_date)),
        vat_deductible: Set(!issued && opts.vat_deductible),
        supplier_account: Set(match (&plan.bank, issued) {
            (Some(b), false) => b
                .iban
                .clone()
                .or_else(|| b.account_number.clone())
                .map(|a| a.chars().take(60).collect()),
            _ => None,
        }),
        category_id: Set(opts.category_id.filter(|_| !issued)),
        custom_fields: Set(serde_json::json!({})),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
        ..Default::default()
    };
    write::apply_totals(&mut row, &plan.totals);
    let stored_file = match file {
        Some(bytes) => {
            let sha = storage::sha256_hex(bytes);
            let rel = original::relative_path(year, id, &sha);
            row.original_path = Set(Some(rel.clone()));
            row.original_sha256 = Set(Some(sha));
            row.original_size = Set(Some(
                i64::try_from(bytes.len()).map_err(|_| AppError::TooLarge)?,
            ));
            row.original_uploaded_at = Set(Some(now.into()));
            Some(rel)
        }
        None => None,
    };
    row.insert(txn).await.map_err(|e| {
        number_violation(e, || {
            AppError::Conflict(if issued { DUPLICATE } else { NUMBER_TAKEN }.into())
        })
    })?;
    // Display-only for both directions: the recap and totals above come from
    // the ISDOC and are never recomputed from these lines.
    lines::replace(txn, id, &plan.lines).await?;
    lines::replace_recap(txn, id, &plan.totals).await?;
    let paid = plan.totals.payable;
    if opts.mark_paid && payable(plan.doc_type) && paid > Decimal::ZERO {
        payment::ActiveModel {
            id: Set(Uuid::new_v4()),
            document_id: Set(id),
            date: Set(plan.due_date.unwrap_or(plan.issue_date)),
            amount: Set(paid),
            note: Set(None),
            created_at: Set(now.into()),
        }
        .insert(txn)
        .await?;
        payments::resum(txn, id).await?;
    }
    if let (Some(rel), Some(bytes)) = (&stored_file, file) {
        pdf.write(rel, bytes.to_vec()).await?;
    }
    Ok(Stored {
        id,
        file: stored_file,
    })
}

/// Store one document; the PDF file is removed again if the commit fails.
pub async fn import(
    db: &DatabaseConnection,
    pdf: &PdfService,
    plan: &Plan,
    file: Option<&[u8]>,
    rate: &Rate,
    opts: &Options,
) -> Result<Uuid, AppError> {
    let txn = db.begin().await?;
    let stored = match insert(&txn, pdf, plan, file, rate, opts).await {
        Ok(s) => s,
        Err(e) => {
            if let Err(r) = txn.rollback().await {
                tracing::warn!(error = %r, "rollback ISDOC import");
            }
            return Err(e);
        }
    };
    if let Err(e) = txn.commit().await {
        if let Some(rel) = &stored.file {
            pdf.remove(rel).await;
        }
        return Err(e.into());
    }
    Ok(stored.id)
}

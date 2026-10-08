//! Credit notes: the draft created from an issued invoice, and the per-rate
//! cap checked when it is issued.

use anyhow::Context as _;
use chrono::NaiveDate;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, EntityTrait,
    QueryFilter, TransactionTrait,
};
use uuid::Uuid;

use super::{context, query, view, write};
use crate::document::compute::{self, Params, Totals};
use crate::document::credit::{copy_lines, exceeds_original};
use crate::document::entity::{document, vat_recap};
use crate::document::handlers::input::DocumentData;
use crate::document::line::{LineData, PaymentMethod, Status, VatMode};
use crate::error::AppError;
use crate::settings::doc_type::DocType;

/// Lock `id` and require an issued invoice (else `invalid_state`).
async fn locked_invoice<C: ConnectionTrait>(
    txn: &C,
    id: Uuid,
) -> Result<document::Model, AppError> {
    let doc = query::lock(txn, id).await?;
    if doc.doc_type != DocType::Invoice.as_str() || view::status(&doc)? != Status::Issued {
        return Err(AppError::InvalidState);
    }
    Ok(doc)
}

/// A draft credit note copying the invoice (advance lines dropped).
pub async fn create(
    db: &DatabaseConnection,
    invoice_id: Uuid,
    correction_reason: Option<String>,
    today: NaiveDate,
) -> Result<Uuid, AppError> {
    let inv = query::find(db, invoice_id).await?;
    if inv.doc_type != DocType::Invoice.as_str() || view::status(&inv)? != Status::Issued {
        return Err(AppError::InvalidState);
    }
    let due_date = context::due_date(db, inv.contact_id, today).await?;
    let vat_mode = VatMode::parse(&inv.vat_mode)
        .with_context(|| format!("document {invoice_id} has unknown vat mode"))?;
    let data = DocumentData {
        doc_type: DocType::CreditNote,
        related_document_id: Some(inv.id),
        correction_reason,
        contact_id: inv.contact_id,
        issue_date: today,
        tax_point_date: Some(today),
        due_date,
        exchange_rate: inv.exchange_rate.filter(|_| inv.currency != "CZK"),
        currency: inv.currency,
        locale: inv.locale,
        vat_mode,
        bank_account_id: inv.bank_account_id,
        payment_method: PaymentMethod::parse(&inv.payment_method)
            .unwrap_or(PaymentMethod::BankTransfer),
        variable_symbol: None,
        constant_symbol: None,
        order_ref: None,
        header_note: None,
        footer_note: None,
        internal_note: None,
        round_total: false,
        lines: copy_lines(&query::load_lines(db, invoice_id).await?),
    };
    let totals = compute::evaluate(&data.lines, data.params())
        .map_err(AppError::Validation)?
        .totals;
    Ok(db
        .transaction(|txn| {
            Box::pin(async move {
                locked_invoice(txn, invoice_id).await?;
                write::create_in(txn, data, totals).await
            })
        })
        .await?)
}

/// Every non-cancelled credit note of the invoice — the issued ones plus the
/// one being issued (`own`) — may not exceed the invoice's item base per rate.
/// Locks the invoice row, so concurrent credit-note issues queue.
pub async fn check_cap(
    txn: &DatabaseTransaction,
    credit_note_id: Uuid,
    invoice_id: Uuid,
    own: &Totals,
) -> Result<(), AppError> {
    locked_invoice(txn, invoice_id).await?;
    let original = item_bases(txn, invoice_id).await?;
    let others: Vec<Uuid> = document::Entity::find()
        .filter(document::Column::RelatedDocumentId.eq(invoice_id))
        .filter(document::Column::DocType.eq(DocType::CreditNote.as_str()))
        .filter(document::Column::Status.eq(Status::Issued.as_str()))
        .filter(document::Column::Id.ne(credit_note_id))
        .all(txn)
        .await?
        .into_iter()
        .map(|d| d.id)
        .collect();
    let issued = vat_recap::Entity::find()
        .filter(vat_recap::Column::DocumentId.is_in(others))
        .all(txn)
        .await?;
    let credited = issued
        .iter()
        .map(|r| (r.vat_rate.normalize(), r.base))
        .chain(own.recap.iter().map(|r| (r.vat_rate.normalize(), r.base)));
    if exceeds_original(&original, credited) {
        return Err(AppError::field("lines", "exceeds_original"));
    }
    Ok(())
}

/// The invoice's item base per rate (before any advance deduction).
async fn item_bases<C: ConnectionTrait>(
    db: &C,
    invoice_id: Uuid,
) -> Result<Vec<(rust_decimal::Decimal, rust_decimal::Decimal)>, AppError> {
    let lines = query::load_lines(db, invoice_id).await?;
    let items = lines.iter().filter_map(|l| match l {
        LineData::Item(i) => {
            compute::item_base(i.quantity, i.unit_price, i.discount_pct).map(|b| (i.vat_rate, b))
        }
        _ => None,
    });
    // Recap rows with vat 0 — only the per-rate base sums matter here.
    let params = Params {
        vat_mode: VatMode::Exempt,
        is_czk: true,
        exchange_rate: None,
        round_total: false,
    };
    let totals = compute::totals(items, params)
        .map_err(|_| anyhow::anyhow!("stored invoice {invoice_id} totals overflow"))?;
    Ok(totals
        .recap
        .into_iter()
        .map(|r| (r.vat_rate.normalize(), r.base))
        .collect())
}

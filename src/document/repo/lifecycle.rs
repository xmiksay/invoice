//! Changes allowed after issue: cancel, mark-sent, internal note.

use chrono::{DateTime, FixedOffset};
use sea_orm::{ActiveModelTrait, DatabaseConnection, DatabaseTransaction, Set, TransactionTrait};
use uuid::Uuid;

use super::{credit, ddpp_correction, query, view};
use crate::document::entity::document;
use crate::document::line::Status;
use crate::error::AppError;
use crate::settings::doc_type::ISSUED;

/// Lock the row, require `issued` and `allowed`, apply `f`. Anything else is
/// `invalid_state`.
async fn on_issued(
    db: &DatabaseConnection,
    id: Uuid,
    allowed: fn(&document::Model) -> bool,
    f: impl FnOnce(&mut document::ActiveModel) + Send + 'static,
) -> Result<(), AppError> {
    Ok(db
        .transaction(|txn| Box::pin(on_issued_in(txn, id, allowed, f)))
        .await?)
}

async fn on_issued_in(
    txn: &DatabaseTransaction,
    id: Uuid,
    allowed: fn(&document::Model) -> bool,
    f: impl FnOnce(&mut document::ActiveModel),
) -> Result<(), AppError> {
    let doc = locked_issued(txn, id, allowed).await?;
    apply(txn, doc, f).await
}

/// Lock the row; received documents are only recorded (no cancel / mark-sent).
async fn locked_issued(
    txn: &DatabaseTransaction,
    id: Uuid,
    allowed: fn(&document::Model) -> bool,
) -> Result<document::Model, AppError> {
    let doc = query::lock(txn, id).await?;
    if view::status(&doc)? != Status::Issued || doc.direction != ISSUED || !allowed(&doc) {
        return Err(AppError::InvalidState);
    }
    Ok(doc)
}

async fn apply(
    txn: &DatabaseTransaction,
    doc: document::Model,
    f: impl FnOnce(&mut document::ActiveModel),
) -> Result<(), AppError> {
    let mut row: document::ActiveModel = doc.into();
    f(&mut row);
    row.updated_at = Set(chrono::Utc::now().into());
    row.update(txn).await?;
    Ok(())
}

/// A native DDPP is only ever cancelled by deleting its payment (an imported
/// one has no payment link); a proforma only while nothing is paid on it.
fn cancellable(doc: &document::Model) -> bool {
    match doc.doc_type.as_str() {
        "advance_tax_doc" => doc.imported,
        "proforma" => doc.paid.is_zero(),
        _ => true,
    }
}

/// Payments stay; the number stays used. Corrections keep their originals
/// consistent: a debit note only while the credit notes stay within the
/// lowered cap, a DDPP correction only while its DDPP is not deducted, an
/// (imported) DDPP only without live corrections.
pub async fn cancel(
    db: &DatabaseConnection,
    id: Uuid,
    reason: Option<String>,
) -> Result<(), AppError> {
    Ok(db
        .transaction(|txn| {
            Box::pin(async move {
                let doc = locked_issued(txn, id, cancellable).await?;
                match doc.doc_type.as_str() {
                    "debit_note" => credit::check_debit_cancel(txn, &doc).await?,
                    "advance_credit_note" => ddpp_correction::check_linked(txn, &doc).await?,
                    "advance_tax_doc" => ddpp_correction::ensure_uncorrected(txn, doc.id).await?,
                    _ => {}
                }
                apply(txn, doc, move |row| {
                    row.status = Set(Status::Cancelled.as_str().into());
                    row.cancelled_at = Set(Some(chrono::Utc::now().into()));
                    row.cancel_reason = Set(reason);
                })
                .await
            })
        })
        .await?)
}

/// Idempotent: a repeated call overwrites `sent_at`.
pub async fn mark_sent(
    db: &DatabaseConnection,
    id: Uuid,
    sent_at: DateTime<FixedOffset>,
) -> Result<(), AppError> {
    on_issued(
        db,
        id,
        |_| true,
        move |row| row.sent_at = Set(Some(sent_at)),
    )
    .await
}

/// Allowed in every status.
pub async fn set_internal_note(
    db: &DatabaseConnection,
    id: Uuid,
    note: Option<String>,
) -> Result<(), AppError> {
    let doc = query::find(db, id).await?;
    let mut row: document::ActiveModel = doc.into();
    row.internal_note = Set(note);
    row.updated_at = Set(chrono::Utc::now().into());
    row.update(db).await?;
    Ok(())
}

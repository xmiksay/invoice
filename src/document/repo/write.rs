//! Draft writes: create, replace, delete. Totals are recomputed and stored on
//! every save.

use sea_orm::{
    ActiveModelTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, Set, TransactionTrait,
};
use uuid::Uuid;

use super::{advance_sources, lines, query, view};
use crate::document::compute::Totals;
use crate::document::entity::document::{ActiveModel, Entity};
use crate::document::handlers::input::DocumentData;
use crate::document::line::Status;
use crate::error::AppError;
use crate::settings::doc_type::DocType;

/// Header fields an edit sets, shared by create and update.
fn apply(row: &mut ActiveModel, d: &DocumentData) {
    row.contact_id = Set(d.contact_id);
    row.issue_date = Set(d.issue_date);
    row.tax_point_date = Set(d.tax_point_date);
    row.due_date = Set(d.due_date);
    row.currency = Set(d.currency.clone());
    row.exchange_rate = Set(d.exchange_rate);
    row.exchange_rate_date = Set(None);
    // A credit note's rate is copied from its invoice, never entered.
    let source = match d.doc_type {
        DocType::CreditNote => "original",
        _ => "manual",
    };
    row.exchange_rate_source = Set(d.exchange_rate.map(|_| source.to_string()));
    row.correction_reason = Set(d.correction_reason.clone());
    row.locale = Set(d.locale.clone());
    row.vat_mode = Set(d.vat_mode.as_str().to_string());
    row.bank_account_id = Set(d.bank_account_id);
    row.payment_method = Set(d.payment_method.as_str().to_string());
    row.variable_symbol = Set(d.variable_symbol.clone());
    row.constant_symbol = Set(d.constant_symbol.clone());
    row.order_ref = Set(d.order_ref.clone());
    row.header_note = Set(d.header_note.clone());
    row.footer_note = Set(d.footer_note.clone());
    row.internal_note = Set(d.internal_note.clone());
    row.round_total = Set(d.round_total);
}

pub fn apply_totals(row: &mut ActiveModel, t: &Totals) {
    row.total_base = Set(t.base);
    row.total_vat = Set(t.vat);
    row.total = Set(t.total);
    row.rounding = Set(t.rounding);
    row.payable = Set(t.payable);
    row.total_czk = Set(t.total_czk);
}

/// `data` and `totals` must already be validated / computed.
pub async fn create(
    db: &DatabaseConnection,
    data: DocumentData,
    totals: Totals,
) -> Result<Uuid, AppError> {
    Ok(db
        .transaction(|txn| Box::pin(create_in(txn, data, totals)))
        .await?)
}

pub async fn create_in(
    txn: &DatabaseTransaction,
    data: DocumentData,
    totals: Totals,
) -> Result<Uuid, AppError> {
    let id = Uuid::new_v4();
    let now = chrono::Utc::now().into();
    let mut row = ActiveModel {
        id: Set(id),
        direction: Set("issued".into()),
        doc_type: Set(data.doc_type.as_str().into()),
        status: Set(Status::Draft.as_str().into()),
        number: Set(None),
        number_year: Set(None),
        number_seq: Set(None),
        imported: Set(false),
        supplier_snapshot: Set(None),
        customer_snapshot: Set(None),
        bank_snapshot: Set(None),
        paid: Set(Default::default()),
        sent_at: Set(None),
        cancelled_at: Set(None),
        cancel_reason: Set(None),
        related_document_id: Set(data.related_document_id),
        payment_id: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    apply(&mut row, &data);
    apply_totals(&mut row, &totals);
    advance_sources::lock_and_recheck(txn, id, &data.lines).await?;
    row.insert(txn).await?;
    lines::replace(txn, id, &data.lines).await?;
    lines::replace_recap(txn, id, &totals).await?;
    Ok(id)
}

/// Replace a draft; anything else is [`AppError::DocumentLocked`].
pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    data: DocumentData,
    totals: Totals,
) -> Result<(), AppError> {
    Ok(db
        .transaction(|txn| Box::pin(update_in(txn, id, data, totals)))
        .await?)
}

async fn update_in(
    txn: &DatabaseTransaction,
    id: Uuid,
    data: DocumentData,
    totals: Totals,
) -> Result<(), AppError> {
    let doc = locked_draft(txn, id).await?;
    advance_sources::lock_and_recheck(txn, id, &data.lines).await?;
    let mut row: ActiveModel = doc.into();
    apply(&mut row, &data);
    apply_totals(&mut row, &totals);
    row.updated_at = Set(chrono::Utc::now().into());
    row.update(txn).await?;
    lines::replace(txn, id, &data.lines).await?;
    lines::replace_recap(txn, id, &totals).await?;
    Ok(())
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<(), AppError> {
    Ok(db
        .transaction(|txn| {
            Box::pin(async move {
                locked_draft(txn, id).await?;
                Entity::delete_by_id(id).exec(txn).await?;
                Ok(())
            })
        })
        .await?)
}

async fn locked_draft(
    txn: &DatabaseTransaction,
    id: Uuid,
) -> Result<crate::document::entity::document::Model, AppError> {
    let doc = query::lock(txn, id).await?;
    if view::status(&doc)? != Status::Draft {
        return Err(AppError::DocumentLocked);
    }
    Ok(doc)
}

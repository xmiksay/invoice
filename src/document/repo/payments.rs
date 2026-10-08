//! Payments of an issued document; `documents.paid` is re-summed in the same
//! transaction so list filters on payment state stay exact.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction,
    EntityTrait, QueryFilter, QueryOrder, Set, Statement, TransactionTrait,
};
use uuid::Uuid;

use super::{query, view};
use crate::document::entity::document;
use crate::document::entity::payment::{self, ActiveModel, Column, Entity};
use crate::document::line::{MAX_AMOUNT, Status};
use crate::error::AppError;

pub struct NewPayment {
    pub date: NaiveDate,
    pub amount: Decimal,
    pub note: Option<String>,
}

pub async fn list(db: &DatabaseConnection, id: Uuid) -> Result<Vec<payment::Model>, AppError> {
    query::find(db, id).await?;
    Ok(Entity::find()
        .filter(Column::DocumentId.eq(id))
        .order_by_asc(Column::Date)
        .order_by_asc(Column::CreatedAt)
        .all(db)
        .await?)
}

pub async fn create(
    db: &DatabaseConnection,
    id: Uuid,
    p: NewPayment,
) -> Result<payment::Model, AppError> {
    Ok(db
        .transaction(|txn| {
            Box::pin(async move {
                let doc = locked_issued(txn, id).await?;
                // `documents.paid` is numeric(18,2) too: the new sum must fit.
                if doc
                    .paid
                    .checked_add(p.amount)
                    .is_none_or(|sum| sum >= MAX_AMOUNT)
                {
                    return Err(AppError::field("amount", "invalid"));
                }
                let row = ActiveModel {
                    id: Set(Uuid::new_v4()),
                    document_id: Set(id),
                    date: Set(p.date),
                    amount: Set(p.amount),
                    note: Set(p.note),
                    created_at: Set(chrono::Utc::now().into()),
                }
                .insert(txn)
                .await?;
                resum(txn, id).await?;
                Ok(row)
            })
        })
        .await?)
}

pub async fn delete(db: &DatabaseConnection, id: Uuid, payment_id: Uuid) -> Result<(), AppError> {
    Ok(db
        .transaction(|txn| {
            Box::pin(async move {
                locked_issued(txn, id).await?;
                let res = Entity::delete_many()
                    .filter(Column::Id.eq(payment_id))
                    .filter(Column::DocumentId.eq(id))
                    .exec(txn)
                    .await?;
                if res.rows_affected == 0 {
                    return Err(AppError::NotFound);
                }
                resum(txn, id).await
            })
        })
        .await?)
}

async fn locked_issued(txn: &DatabaseTransaction, id: Uuid) -> Result<document::Model, AppError> {
    let doc = query::lock(txn, id).await?;
    if view::status(&doc)? != Status::Issued {
        return Err(AppError::InvalidState);
    }
    Ok(doc)
}

async fn resum(txn: &DatabaseTransaction, id: Uuid) -> Result<(), AppError> {
    txn.execute(Statement::from_sql_and_values(
        txn.get_database_backend(),
        "UPDATE documents SET paid = (SELECT COALESCE(SUM(amount), 0) FROM payments \
         WHERE document_id = $1), updated_at = now() WHERE id = $1",
        [id.into()],
    ))
    .await?;
    Ok(())
}

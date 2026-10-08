use sea_orm::{
    ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, QueryOrder, QuerySelect, Set,
    TransactionTrait,
};
use uuid::Uuid;

use crate::error::{AppError, FieldErrors, unique_violation};
use crate::settings::entity::custom_field::{self, ActiveModel, Column, Entity};
use crate::settings::handlers::custom_fields::CustomFieldData;

pub async fn list(db: &DatabaseConnection) -> Result<Vec<custom_field::Model>, AppError> {
    Ok(Entity::find()
        .order_by_asc(Column::Position)
        .order_by_asc(Column::Key)
        .all(db)
        .await?)
}

fn key_taken(e: DbErr) -> AppError {
    match unique_violation(&e).as_deref() {
        Some("custom_fields_key_key") => AppError::field("key", "duplicate"),
        _ => e.into(),
    }
}

fn apply(row: &mut ActiveModel, d: CustomFieldData) {
    row.label = Set(d.label);
    row.options = Set(d.options);
    row.applies_to = Set(d.applies_to);
    row.required = Set(d.required);
    row.active = Set(d.active);
    row.position = Set(d.position);
    row.updated_at = Set(chrono::Utc::now().into());
}

pub async fn create(
    db: &DatabaseConnection,
    d: CustomFieldData,
) -> Result<custom_field::Model, AppError> {
    let mut row = ActiveModel {
        id: Set(Uuid::new_v4()),
        key: Set(d.key.clone()),
        field_type: Set(d.field_type.as_str().to_string()),
        created_at: Set(chrono::Utc::now().into()),
        ..Default::default()
    };
    apply(&mut row, d);
    row.insert(db).await.map_err(key_taken)
}

/// `key` and `type` are immutable (stored values depend on them) → `invalid`.
pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    d: CustomFieldData,
) -> Result<custom_field::Model, AppError> {
    Ok(db
        .transaction(|txn| {
            Box::pin(async move {
                let existing = Entity::find_by_id(id)
                    .lock_exclusive()
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                let mut e = FieldErrors::new();
                if existing.key != d.key {
                    e.add("key", "invalid");
                }
                if existing.field_type != d.field_type.as_str() {
                    e.add("type", "invalid");
                }
                e.into_result()?;
                let mut row: ActiveModel = existing.into();
                apply(&mut row, d);
                Ok(row.update(txn).await?)
            })
        })
        .await?)
}

/// Stored values stay in the documents' JSON (ignored from now on).
pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<(), AppError> {
    let res = Entity::delete_by_id(id).exec(db).await?;
    if res.rows_affected == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

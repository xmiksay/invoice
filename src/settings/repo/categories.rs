use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use uuid::Uuid;

use crate::document::entity::document;
use crate::error::{AppError, unique_violation};
use crate::settings::entity::category::{self, ActiveModel, Column, Entity};
use crate::settings::handlers::categories::CategoryData;

pub async fn list<C: ConnectionTrait>(db: &C) -> Result<Vec<category::Model>, AppError> {
    Ok(Entity::find()
        .order_by_asc(Column::Kind)
        .order_by_asc(Column::Position)
        .order_by_asc(Column::Name)
        .order_by_asc(Column::Id)
        .all(db)
        .await?)
}

fn name_taken(e: DbErr) -> AppError {
    match unique_violation(&e).as_deref() {
        Some("categories_kind_name_key") => AppError::field("name", "duplicate"),
        _ => e.into(),
    }
}

fn apply(row: &mut ActiveModel, d: CategoryData) {
    row.name = Set(d.name);
    row.kind = Set(d.kind);
    row.active = Set(d.active);
    row.position = Set(d.position);
    row.updated_at = Set(chrono::Utc::now().into());
}

pub async fn create(db: &DatabaseConnection, d: CategoryData) -> Result<category::Model, AppError> {
    let mut row = ActiveModel {
        id: Set(Uuid::new_v4()),
        created_at: Set(chrono::Utc::now().into()),
        ..Default::default()
    };
    apply(&mut row, d);
    row.insert(db).await.map_err(name_taken)
}

async fn used<C: ConnectionTrait>(db: &C, id: Uuid) -> Result<bool, AppError> {
    Ok(document::Entity::find()
        .filter(document::Column::CategoryId.eq(id))
        .count(db)
        .await?
        > 0)
}

/// The row lock makes a concurrent document write (its FK check share-locks
/// the category) wait, so the in-use checks below cannot go stale.
async fn locked<C: ConnectionTrait>(db: &C, id: Uuid) -> Result<category::Model, AppError> {
    Entity::find_by_id(id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or(AppError::NotFound)
}

/// The kind of a category in use is fixed (its documents' direction
/// depends on it) → `kind: invalid`.
pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    d: CategoryData,
) -> Result<category::Model, AppError> {
    Ok(db
        .transaction(|txn| {
            Box::pin(async move {
                let existing = locked(txn, id).await?;
                if existing.kind != d.kind && used(txn, id).await? {
                    return Err(AppError::field("kind", "invalid"));
                }
                let mut row: ActiveModel = existing.into();
                apply(&mut row, d);
                row.update(txn).await.map_err(name_taken)
            })
        })
        .await?)
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<(), AppError> {
    Ok(db
        .transaction(|txn| {
            Box::pin(async move {
                locked(txn, id).await?;
                if used(txn, id).await? {
                    return Err(AppError::CategoryInUse);
                }
                Entity::delete_by_id(id).exec(txn).await?;
                Ok(())
            })
        })
        .await?)
}

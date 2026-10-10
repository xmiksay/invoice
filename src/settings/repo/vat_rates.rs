use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbErr,
    EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use uuid::Uuid;

use crate::error::{AppError, unique_violation};
use crate::settings::entity::vat_rate::{self, ActiveModel, Column, Entity};
use crate::settings::handlers::vat_rates::VatRateData;
use crate::space::SpaceId;

pub async fn list<C: ConnectionTrait>(
    db: &C,
    space: SpaceId,
) -> Result<Vec<vat_rate::Model>, AppError> {
    Ok(Entity::find()
        .filter(Column::SpaceId.eq(space))
        .order_by_asc(Column::Position)
        .order_by_desc(Column::Rate)
        .all(db)
        .await?)
}

pub async fn create(
    db: &DatabaseConnection,
    space: SpaceId,
    input: VatRateData,
) -> Result<vat_rate::Model, AppError> {
    Ok(db
        .transaction(|txn| Box::pin(create_in(txn, space, input)))
        .await?)
}

pub async fn update(
    db: &DatabaseConnection,
    space: SpaceId,
    id: Uuid,
    input: VatRateData,
) -> Result<vat_rate::Model, AppError> {
    Ok(db
        .transaction(|txn| Box::pin(update_in(txn, space, id, input)))
        .await?)
}

pub async fn delete(db: &DatabaseConnection, space: SpaceId, id: Uuid) -> Result<(), AppError> {
    Ok(db
        .transaction(|txn| Box::pin(delete_in(txn, space, id)))
        .await?)
}

async fn create_in(
    txn: &DatabaseTransaction,
    space: SpaceId,
    input: VatRateData,
) -> Result<vat_rate::Model, AppError> {
    if input.is_default {
        unset_default(txn, space, None).await?;
    }
    let position = match input.position {
        Some(p) => p,
        None => next_position(txn, space).await?,
    };
    let id = Uuid::new_v4();
    ActiveModel {
        id: Set(id),
        space_id: Set(space.uuid()),
        rate: Set(input.rate),
        label: Set(input.label),
        is_default: Set(input.is_default),
        active: Set(input.active),
        position: Set(position),
    }
    .insert(txn)
    .await
    .map_err(map_duplicate)?;
    ensure_default(txn, space, None).await?;
    find(txn, space, id).await
}

async fn update_in(
    txn: &DatabaseTransaction,
    space: SpaceId,
    id: Uuid,
    input: VatRateData,
) -> Result<vat_rate::Model, AppError> {
    let old = find(txn, space, id).await?;
    if old.is_default && !input.active && !other_active_exists(txn, space, id).await? {
        return Err(AppError::field("isDefault", "invalid"));
    }
    if input.is_default {
        unset_default(txn, space, Some(id)).await?;
    }
    let mut row: ActiveModel = old.into();
    row.rate = Set(input.rate);
    row.label = Set(input.label);
    row.is_default = Set(input.is_default);
    row.active = Set(input.active);
    if let Some(p) = input.position {
        row.position = Set(p);
    }
    row.update(txn).await.map_err(map_duplicate)?;
    ensure_default(txn, space, Some(id)).await?;
    find(txn, space, id).await
}

/// Always allowed: invoice lines store the rate value, not a reference.
async fn delete_in(txn: &DatabaseTransaction, space: SpaceId, id: Uuid) -> Result<(), AppError> {
    find(txn, space, id).await?;
    Entity::delete_by_id(id).exec(txn).await?;
    ensure_default(txn, space, None).await?;
    Ok(())
}

async fn other_active_exists(
    txn: &DatabaseTransaction,
    space: SpaceId,
    id: Uuid,
) -> Result<bool, AppError> {
    let other = Entity::find()
        .filter(Column::SpaceId.eq(space))
        .filter(Column::Active.eq(true))
        .filter(Column::Id.ne(id))
        .one(txn)
        .await?;
    Ok(other.is_some())
}

fn map_duplicate(err: DbErr) -> AppError {
    match unique_violation(&err) {
        Some(constraint) if constraint == "vat_rates_rate_key" => {
            AppError::field("rate", "duplicate")
        }
        _ => err.into(),
    }
}

async fn find(
    txn: &DatabaseTransaction,
    space: SpaceId,
    id: Uuid,
) -> Result<vat_rate::Model, AppError> {
    Entity::find_by_id(id)
        .filter(Column::SpaceId.eq(space))
        .one(txn)
        .await?
        .ok_or(AppError::NotFound)
}

async fn next_position(txn: &DatabaseTransaction, space: SpaceId) -> Result<i32, AppError> {
    let max: Option<Option<i32>> = Entity::find()
        .filter(Column::SpaceId.eq(space))
        .select_only()
        .column_as(Column::Position.max(), "max")
        .into_tuple()
        .one(txn)
        .await?;
    Ok(max.flatten().map_or(1, |m| m.saturating_add(1)))
}

async fn unset_default(
    txn: &DatabaseTransaction,
    space: SpaceId,
    except: Option<Uuid>,
) -> Result<(), AppError> {
    let mut q = Entity::update_many()
        .filter(Column::SpaceId.eq(space))
        .col_expr(Column::IsDefault, Expr::value(false))
        .filter(Column::IsDefault.eq(true));
    if let Some(id) = except {
        q = q.filter(Column::Id.ne(id));
    }
    q.exec(txn).await?;
    Ok(())
}

/// Exactly one default while any active rate exists (the default is always
/// active): if none is flagged, promote the first active rate by position,
/// preferring any other than `avoid` (just un-flagged or deactivated).
async fn ensure_default(
    txn: &DatabaseTransaction,
    space: SpaceId,
    avoid: Option<Uuid>,
) -> Result<(), AppError> {
    let rows = Entity::find()
        .filter(Column::SpaceId.eq(space))
        .filter(Column::Active.eq(true))
        .order_by_asc(Column::Position)
        .order_by_desc(Column::Rate)
        .all(txn)
        .await?;
    let flags: Vec<(Uuid, bool)> = rows.iter().map(|r| (r.id, r.is_default)).collect();
    if let Some(id) = super::default_to_promote(&flags, avoid) {
        Entity::update_many()
            .col_expr(Column::IsDefault, Expr::value(true))
            .filter(Column::Id.eq(id))
            .exec(txn)
            .await?;
    }
    Ok(())
}

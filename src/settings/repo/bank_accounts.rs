use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use uuid::Uuid;

use crate::error::AppError;
use crate::settings::entity::bank_account::{self, ActiveModel, Column, Entity};
use crate::settings::handlers::bank_accounts::BankAccountInput;
use crate::space::SpaceId;

pub async fn list(
    db: &DatabaseConnection,
    space: SpaceId,
) -> Result<Vec<bank_account::Model>, AppError> {
    Ok(Entity::find()
        .filter(Column::SpaceId.eq(space))
        .order_by_asc(Column::Currency)
        .order_by_desc(Column::IsDefault)
        .order_by_asc(Column::Label)
        .order_by_asc(Column::CreatedAt)
        .all(db)
        .await?)
}

pub async fn create(
    db: &DatabaseConnection,
    space: SpaceId,
    input: BankAccountInput,
) -> Result<bank_account::Model, AppError> {
    Ok(db
        .transaction(|txn| Box::pin(create_in(txn, space, input)))
        .await?)
}

pub async fn update(
    db: &DatabaseConnection,
    space: SpaceId,
    id: Uuid,
    input: BankAccountInput,
) -> Result<bank_account::Model, AppError> {
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
    input: BankAccountInput,
) -> Result<bank_account::Model, AppError> {
    if input.is_default {
        unset_default(txn, space, &input.currency, None).await?;
    }
    let id = Uuid::new_v4();
    ActiveModel {
        id: Set(id),
        space_id: Set(space.uuid()),
        label: Set(input.label),
        currency: Set(input.currency.clone()),
        account_number: Set(input.account_number),
        iban: Set(input.iban),
        bic: Set(input.bic),
        is_default: Set(input.is_default),
        created_at: Set(chrono::Utc::now().into()),
    }
    .insert(txn)
    .await?;
    ensure_default(txn, space, &input.currency, None).await?;
    find(txn, space, id).await
}

async fn update_in(
    txn: &DatabaseTransaction,
    space: SpaceId,
    id: Uuid,
    input: BankAccountInput,
) -> Result<bank_account::Model, AppError> {
    let old = find(txn, space, id).await?;
    if input.is_default {
        unset_default(txn, space, &input.currency, Some(id)).await?;
    }
    let mut row: ActiveModel = old.clone().into();
    row.label = Set(input.label);
    row.currency = Set(input.currency.clone());
    row.account_number = Set(input.account_number);
    row.iban = Set(input.iban);
    row.bic = Set(input.bic);
    row.is_default = Set(input.is_default);
    row.update(txn).await?;
    ensure_default(txn, space, &input.currency, Some(id)).await?;
    if old.currency != input.currency {
        ensure_default(txn, space, &old.currency, None).await?;
    }
    find(txn, space, id).await
}

async fn delete_in(txn: &DatabaseTransaction, space: SpaceId, id: Uuid) -> Result<(), AppError> {
    let old = find(txn, space, id).await?;
    Entity::delete_by_id(id).exec(txn).await?;
    ensure_default(txn, space, &old.currency, None).await?;
    Ok(())
}

async fn find(
    txn: &DatabaseTransaction,
    space: SpaceId,
    id: Uuid,
) -> Result<bank_account::Model, AppError> {
    Entity::find_by_id(id)
        .filter(Column::SpaceId.eq(space))
        .one(txn)
        .await?
        .ok_or(AppError::NotFound)
}

async fn unset_default(
    txn: &DatabaseTransaction,
    space: SpaceId,
    currency: &str,
    except: Option<Uuid>,
) -> Result<(), AppError> {
    let mut q = Entity::update_many()
        .filter(Column::SpaceId.eq(space))
        .col_expr(Column::IsDefault, Expr::value(false))
        .filter(Column::Currency.eq(currency))
        .filter(Column::IsDefault.eq(true));
    if let Some(id) = except {
        q = q.filter(Column::Id.ne(id));
    }
    q.exec(txn).await?;
    Ok(())
}

/// A currency with accounts always has a default: if none is flagged, promote
/// the oldest account, preferring any other than `avoid` (the one the caller
/// just un-flagged).
async fn ensure_default(
    txn: &DatabaseTransaction,
    space: SpaceId,
    currency: &str,
    avoid: Option<Uuid>,
) -> Result<(), AppError> {
    let rows = Entity::find()
        .filter(Column::SpaceId.eq(space))
        .filter(Column::Currency.eq(currency))
        .order_by_asc(Column::CreatedAt)
        .order_by_asc(Column::Id)
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

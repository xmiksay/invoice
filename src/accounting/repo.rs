//! The stored accounting settings (one JSON row).

use anyhow::Context as _;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, QuerySelect, Set, TransactionTrait};

use super::entity;
use super::settings::{AccountingSettings, AccountingUpdate};
use crate::error::AppError;
use crate::space::SpaceId;

async fn read<C: ConnectionTrait>(
    db: &C,
    space: SpaceId,
    lock: bool,
) -> Result<AccountingSettings, AppError> {
    let mut q = entity::Entity::find_by_id(space.uuid());
    if lock {
        q = q.lock_exclusive();
    }
    let row = q
        .one(db)
        .await?
        .context("accounting settings row of the space missing (created with the space)")?;
    let s: AccountingSettings =
        serde_json::from_value(row.data).context("decode stored accounting settings")?;
    Ok(s.full())
}

/// The settings with every section and row ([`AccountingSettings::full`]).
pub async fn get<C: ConnectionTrait>(
    db: &C,
    space: SpaceId,
) -> Result<AccountingSettings, AppError> {
    read(db, space, false).await
}

/// Apply `update` (already validated) to the stored settings: the present
/// sections are replaced, the others kept. Row-locked, so two saves of
/// different sections never lose one.
pub async fn update<C: TransactionTrait>(
    db: &C,
    space: SpaceId,
    update: AccountingUpdate,
) -> Result<AccountingSettings, AppError> {
    let txn = db.begin().await?;
    let s = update.apply(read(&txn, space, true).await?);
    entity::ActiveModel {
        space_id: Set(space.uuid()),
        data: Set(serde_json::to_value(&s).context("encode accounting settings")?),
        updated_at: Set(chrono::Utc::now().into()),
    }
    .update(&txn)
    .await?;
    txn.commit().await?;
    Ok(s)
}

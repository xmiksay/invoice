//! The stored accounting settings (one JSON row).

use anyhow::Context as _;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, QuerySelect, Set, TransactionTrait};

use super::entity::{self, SINGLETON_ID};
use super::settings::{AccountingSettings, AccountingUpdate};
use crate::error::AppError;

async fn read<C: ConnectionTrait>(db: &C, lock: bool) -> Result<AccountingSettings, AppError> {
    let mut q = entity::Entity::find_by_id(SINGLETON_ID);
    if lock {
        q = q.lock_exclusive();
    }
    let row = q
        .one(db)
        .await?
        .context("accounting settings row missing (migration seeds it)")?;
    let s: AccountingSettings =
        serde_json::from_value(row.data).context("decode stored accounting settings")?;
    Ok(s.full())
}

/// The settings with every section and row ([`AccountingSettings::full`]).
pub async fn get<C: ConnectionTrait>(db: &C) -> Result<AccountingSettings, AppError> {
    read(db, false).await
}

/// Apply `update` (already validated) to the stored settings: the present
/// sections are replaced, the others kept. Row-locked, so two saves of
/// different sections never lose one.
pub async fn update<C: TransactionTrait>(
    db: &C,
    update: AccountingUpdate,
) -> Result<AccountingSettings, AppError> {
    let txn = db.begin().await?;
    let s = update.apply(read(&txn, true).await?);
    entity::ActiveModel {
        id: Set(SINGLETON_ID),
        data: Set(serde_json::to_value(&s).context("encode accounting settings")?),
        updated_at: Set(chrono::Utc::now().into()),
    }
    .update(&txn)
    .await?;
    txn.commit().await?;
    Ok(s)
}

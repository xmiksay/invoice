//! The stored accounting settings (one JSON row).

use anyhow::Context as _;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};

use super::entity::{self, SINGLETON_ID};
use super::settings::AccountingSettings;
use crate::error::AppError;

/// The settings with every direction × type row ([`AccountingSettings::full`]).
pub async fn get<C: ConnectionTrait>(db: &C) -> Result<AccountingSettings, AppError> {
    let row = entity::Entity::find_by_id(SINGLETON_ID)
        .one(db)
        .await?
        .context("accounting settings row missing (migration seeds it)")?;
    let s: AccountingSettings =
        serde_json::from_value(row.data).context("decode stored accounting settings")?;
    Ok(s.full())
}

/// Replace the settings with `s` (already validated).
pub async fn put<C: ConnectionTrait>(
    db: &C,
    s: &AccountingSettings,
) -> Result<AccountingSettings, AppError> {
    entity::ActiveModel {
        id: Set(SINGLETON_ID),
        data: Set(serde_json::to_value(s).context("encode accounting settings")?),
        updated_at: Set(chrono::Utc::now().into()),
    }
    .update(db)
    .await?;
    Ok(s.clone())
}

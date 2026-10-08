//! Database lookups that validation and defaults need.

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::contact::entity::contact;
use crate::document::handlers::input::{Context, DocumentData};
use crate::error::AppError;
use crate::settings::entity::{bank_account, vat_rate};
use crate::settings::repo::company;

pub async fn load(
    db: &DatabaseConnection,
    contact_id: Option<Uuid>,
    apply_defaults: bool,
    today: chrono::NaiveDate,
) -> Result<Context, AppError> {
    let contact = match contact_id {
        Some(id) => contact::Entity::find_by_id(id).one(db).await?,
        None => None,
    };
    Ok(Context {
        today,
        apply_defaults,
        company: company::get(db).await?,
        contact,
        default_vat_rate: default_vat_rate(db).await?,
    })
}

pub async fn default_vat_rate<C: ConnectionTrait>(db: &C) -> Result<Option<Decimal>, AppError> {
    Ok(vat_rate::Entity::find()
        .filter(vat_rate::Column::IsDefault.eq(true))
        .one(db)
        .await?
        .map(|r| r.rate.normalize()))
}

pub async fn bank_account<C: ConnectionTrait>(
    db: &C,
    id: Uuid,
) -> Result<Option<bank_account::Model>, AppError> {
    Ok(bank_account::Entity::find_by_id(id).one(db).await?)
}

/// The named bank account must exist and match the currency; with defaults
/// on, a missing one becomes the currency's default account (if any).
pub async fn resolve_bank(
    db: &DatabaseConnection,
    data: &mut DocumentData,
    apply_defaults: bool,
) -> Result<(), AppError> {
    match data.bank_account_id {
        Some(id) => match bank_account(db, id).await? {
            Some(acc) if acc.currency == data.currency => Ok(()),
            _ => Err(AppError::field("bankAccountId", "invalid")),
        },
        None if apply_defaults => {
            data.bank_account_id = bank_account::Entity::find()
                .filter(bank_account::Column::Currency.eq(data.currency.as_str()))
                .filter(bank_account::Column::IsDefault.eq(true))
                .one(db)
                .await?
                .map(|a| a.id);
            Ok(())
        }
        None => Ok(()),
    }
}

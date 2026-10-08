use anyhow::Context;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

use crate::error::AppError;
use crate::settings::entity::company::{self, SINGLETON_ID};
use crate::settings::handlers::company::Company;

pub async fn get(db: &DatabaseConnection) -> Result<company::Model, AppError> {
    let row = company::Entity::find_by_id(SINGLETON_ID)
        .one(db)
        .await?
        .context("company singleton row missing (migration seeds it)")?;
    Ok(row)
}

/// Replace every field of the singleton with the (validated) input.
pub async fn update(db: &DatabaseConnection, c: Company) -> Result<company::Model, AppError> {
    let row = company::ActiveModel {
        id: Set(SINGLETON_ID),
        name: Set(c.name),
        ico: Set(c.ico),
        dic: Set(c.dic),
        vat_payer: Set(c.vat_payer),
        street: Set(c.street),
        city: Set(c.city),
        zip: Set(c.zip),
        country: Set(c.country),
        email: Set(c.email),
        phone: Set(c.phone),
        web: Set(c.web),
        registration: Set(c.registration),
        default_due_days: Set(c.default_due_days),
        default_locale: Set(c.default_locale),
        updated_at: Set(chrono::Utc::now().into()),
    }
    .update(db)
    .await?;
    Ok(row)
}

use sea_orm::entity::prelude::*;

/// Own bank account; at most one `is_default` per currency (partial unique index).
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "bank_accounts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub space_id: Uuid,
    pub label: Option<String>,
    pub currency: String,
    pub account_number: Option<String>,
    pub iban: Option<String>,
    pub bic: Option<String>,
    pub is_default: bool,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

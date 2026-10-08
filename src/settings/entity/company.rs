use sea_orm::entity::prelude::*;

/// The own company profile — a singleton row with `id = 1`, seeded by the migration.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "company")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i16,
    pub name: String,
    pub ico: Option<String>,
    pub dic: Option<String>,
    pub vat_payer: bool,
    pub street: String,
    pub city: String,
    pub zip: String,
    pub country: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub web: Option<String>,
    pub registration: Option<String>,
    pub default_due_days: i32,
    pub default_locale: String,
    pub updated_at: DateTimeWithTimeZone,
}

pub const SINGLETON_ID: i16 = 1;

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

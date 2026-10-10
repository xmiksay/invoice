use sea_orm::entity::prelude::*;

/// Settings → Accounting: a singleton (`id = 1`, seeded by the migration)
/// holding [`super::settings::AccountingSettings`] as JSON.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "accounting_settings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i16,
    pub data: Json,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub const SINGLETON_ID: i16 = 1;

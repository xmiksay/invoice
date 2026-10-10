use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "catalog_items")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub space_id: Uuid,
    pub name: String,
    pub unit: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub unit_price: Decimal,
    pub currency: String,
    #[sea_orm(column_type = "Decimal(Some((5, 2)))")]
    pub vat_rate: Decimal,
    pub active: bool,
    pub note: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

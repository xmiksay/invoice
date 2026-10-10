use sea_orm::entity::prelude::*;

/// Selectable VAT rate; `rate` is unique, at most one `is_default` (partial unique index).
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "vat_rates")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub space_id: Uuid,
    #[sea_orm(column_type = "Decimal(Some((5, 2)))")]
    pub rate: Decimal,
    pub label: String,
    pub is_default: bool,
    pub active: bool,
    pub position: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

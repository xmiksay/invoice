use sea_orm::entity::prelude::*;

/// One line; only raw inputs are stored — bases are recomputed on read.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "document_lines")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub document_id: Uuid,
    pub position: i32,
    pub kind: String,
    pub description: String,
    #[sea_orm(column_type = "Decimal(Some((18, 4)))", nullable)]
    pub quantity: Option<Decimal>,
    pub unit: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((18, 4)))", nullable)]
    pub unit_price: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((5, 2)))", nullable)]
    pub discount_pct: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((5, 2)))", nullable)]
    pub vat_rate: Option<Decimal>,
    pub refs: Option<Vec<i32>>,
    pub collapse: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

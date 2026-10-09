use sea_orm::entity::prelude::*;

/// Stored VAT recap row of a document (one per rate).
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "document_vat_recap")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub document_id: Uuid,
    #[sea_orm(
        primary_key,
        auto_increment = false,
        column_type = "Decimal(Some((5, 2)))"
    )]
    pub vat_rate: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub base: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub vat: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))", nullable)]
    pub base_czk: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))", nullable)]
    pub vat_czk: Option<Decimal>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for crate::document::line::AdvanceRow {
    fn from(r: Model) -> Self {
        Self {
            vat_rate: r.vat_rate.normalize(),
            base: r.base,
            vat: r.vat,
            base_czk: r.base_czk,
            vat_czk: r.vat_czk,
        }
    }
}

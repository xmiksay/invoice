use sea_orm::entity::prelude::*;

/// Cached ČNB rate: what ČNB answered for `requested_date` (the list published
/// on or before it). `rate` is `None` when that list lacks the currency.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "exchange_rates")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub currency: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub requested_date: Date,
    pub published_date: Date,
    #[sea_orm(column_type = "Decimal(Some((18, 6)))", nullable)]
    pub rate: Option<Decimal>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

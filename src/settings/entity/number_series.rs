use sea_orm::entity::prelude::*;

/// Number pattern per document type (`doc_type` is a `DocType` wire value).
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "number_series")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub doc_type: String,
    pub pattern: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

use sea_orm::entity::prelude::*;

/// A custom field definition; values live in `documents.custom_fields`.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "custom_fields")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub key: String,
    pub label: String,
    pub field_type: String,
    pub options: Vec<String>,
    pub applies_to: String,
    pub required: bool,
    pub active: bool,
    pub position: i32,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

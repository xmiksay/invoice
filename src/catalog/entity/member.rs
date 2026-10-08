use sea_orm::entity::prelude::*;

/// A group member; removed with its group or its item (FK cascade).
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "catalog_group_members")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub group_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub item_id: Uuid,
    pub position: i32,
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub quantity: Decimal,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "pool_members")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub pool_id: i32,
    pub user_id: i32,
    pub role: String,
    pub invited_at: DateTimeUtc,
    pub accepted_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

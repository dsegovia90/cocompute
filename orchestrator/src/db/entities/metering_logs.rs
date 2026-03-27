use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "metering_logs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub host_endpoint_id: String,
    pub model: String,
    pub request_type: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub compute_ms: i64,
    pub created_at: DateTimeUtc,
    pub api_key_id: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

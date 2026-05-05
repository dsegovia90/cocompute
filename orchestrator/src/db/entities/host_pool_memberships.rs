// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "host_pool_memberships")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub host_endpoint_id: String,
    pub pool_id: i32,
    pub created_at: DateTimeUtc,
    pub is_active: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

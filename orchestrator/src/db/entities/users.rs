// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: String,
    #[sea_orm(unique)]
    pub email: String,
    pub password_hash: String,
    pub name: String,
    pub email_verification_token: Option<String>,
    pub email_verification_sent_at: Option<DateTimeUtc>,
    pub email_verified_at: Option<DateTimeUtc>,
    pub reset_token: Option<String>,
    pub reset_sent_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm_migration::prelude::*;

pub struct M20260412CreateHostTokens;

impl MigrationName for M20260412CreateHostTokens {
    fn name(&self) -> &str {
        "m20260412_000003_create_host_tokens"
    }
}

#[derive(DeriveIden)]
enum HostTokens {
    Table,
    Id,
    TokenHash,
    UserId,
    UsedAt,
    HostId,
    CreatedAt,
    ExpiresAt,
}

#[async_trait::async_trait]
impl MigrationTrait for M20260412CreateHostTokens {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HostTokens::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(HostTokens::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(HostTokens::TokenHash).string().not_null().unique_key())
                    .col(ColumnDef::new(HostTokens::UserId).integer().not_null())
                    .col(ColumnDef::new(HostTokens::UsedAt).timestamp().null())
                    .col(ColumnDef::new(HostTokens::HostId).string().null())
                    .col(ColumnDef::new(HostTokens::CreatedAt).timestamp().not_null().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(HostTokens::ExpiresAt).timestamp().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(HostTokens::Table).to_owned()).await
    }
}

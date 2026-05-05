// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm_migration::prelude::*;

pub struct M20260412AddUserPoolToApiKeys;

impl MigrationName for M20260412AddUserPoolToApiKeys {
    fn name(&self) -> &str {
        "m20260412_000005_add_user_pool_to_api_keys"
    }
}

#[derive(DeriveIden)]
enum ApiKeys {
    Table,
    UserId,
    PoolId,
    Label,
}

#[async_trait::async_trait]
impl MigrationTrait for M20260412AddUserPoolToApiKeys {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(
            Table::alter()
                .table(ApiKeys::Table)
                .add_column(ColumnDef::new(ApiKeys::UserId).integer().null())
                .to_owned(),
        ).await?;
        manager.alter_table(
            Table::alter()
                .table(ApiKeys::Table)
                .add_column(ColumnDef::new(ApiKeys::PoolId).integer().null())
                .to_owned(),
        ).await?;
        manager.alter_table(
            Table::alter()
                .table(ApiKeys::Table)
                .add_column(ColumnDef::new(ApiKeys::Label).string().null())
                .to_owned(),
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(
            Table::alter().table(ApiKeys::Table).drop_column(ApiKeys::UserId).to_owned(),
        ).await?;
        manager.alter_table(
            Table::alter().table(ApiKeys::Table).drop_column(ApiKeys::PoolId).to_owned(),
        ).await?;
        manager.alter_table(
            Table::alter().table(ApiKeys::Table).drop_column(ApiKeys::Label).to_owned(),
        ).await
    }
}

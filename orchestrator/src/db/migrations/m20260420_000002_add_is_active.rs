// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm_migration::prelude::*;

pub struct M20260420AddIsActive;

impl MigrationName for M20260420AddIsActive {
    fn name(&self) -> &str {
        "m20260420_000002_add_is_active"
    }
}

#[derive(DeriveIden)]
enum Pools {
    Table,
    IsActive,
}

#[derive(DeriveIden)]
enum ApiKeys {
    Table,
    IsActive,
}

#[derive(DeriveIden)]
enum HostPoolMemberships {
    Table,
    IsActive,
}

#[async_trait::async_trait]
impl MigrationTrait for M20260420AddIsActive {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(
            Table::alter()
                .table(Pools::Table)
                .add_column(ColumnDef::new(Pools::IsActive).boolean().not_null().default(true))
                .to_owned(),
        ).await?;

        manager.alter_table(
            Table::alter()
                .table(ApiKeys::Table)
                .add_column(ColumnDef::new(ApiKeys::IsActive).boolean().not_null().default(true))
                .to_owned(),
        ).await?;

        manager.alter_table(
            Table::alter()
                .table(HostPoolMemberships::Table)
                .add_column(ColumnDef::new(HostPoolMemberships::IsActive).boolean().not_null().default(true))
                .to_owned(),
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(
            Table::alter().table(Pools::Table).drop_column(Pools::IsActive).to_owned(),
        ).await?;
        manager.alter_table(
            Table::alter().table(ApiKeys::Table).drop_column(ApiKeys::IsActive).to_owned(),
        ).await?;
        manager.alter_table(
            Table::alter().table(HostPoolMemberships::Table).drop_column(HostPoolMemberships::IsActive).to_owned(),
        ).await
    }
}

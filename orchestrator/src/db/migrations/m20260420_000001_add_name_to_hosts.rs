// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm_migration::prelude::*;

pub struct M20260420AddNameToHosts;

impl MigrationName for M20260420AddNameToHosts {
    fn name(&self) -> &str {
        "m20260420_000001_add_name_to_hosts"
    }
}

#[derive(DeriveIden)]
enum Hosts {
    Table,
    Name,
}

#[async_trait::async_trait]
impl MigrationTrait for M20260420AddNameToHosts {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(
            Table::alter()
                .table(Hosts::Table)
                .add_column(ColumnDef::new(Hosts::Name).string().null())
                .to_owned(),
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(
            Table::alter().table(Hosts::Table).drop_column(Hosts::Name).to_owned(),
        ).await
    }
}

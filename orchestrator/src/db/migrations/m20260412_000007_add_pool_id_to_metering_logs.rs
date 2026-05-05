// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm_migration::prelude::*;

pub struct M20260412AddPoolIdToMeteringLogs;

impl MigrationName for M20260412AddPoolIdToMeteringLogs {
    fn name(&self) -> &str {
        "m20260412_000007_add_pool_id_to_metering_logs"
    }
}

#[derive(DeriveIden)]
enum MeteringLogs {
    Table,
    PoolId,
}

#[async_trait::async_trait]
impl MigrationTrait for M20260412AddPoolIdToMeteringLogs {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(
            Table::alter()
                .table(MeteringLogs::Table)
                .add_column(ColumnDef::new(MeteringLogs::PoolId).integer().null())
                .to_owned(),
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(
            Table::alter().table(MeteringLogs::Table).drop_column(MeteringLogs::PoolId).to_owned(),
        ).await
    }
}

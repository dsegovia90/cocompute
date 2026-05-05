// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm_migration::prelude::*;

pub struct M20260327AddApiKeyIdToMeteringLogs;

impl MigrationName for M20260327AddApiKeyIdToMeteringLogs {
    fn name(&self) -> &str {
        "m20260327_add_api_key_id_to_metering_logs"
    }
}

#[derive(DeriveIden)]
enum MeteringLogs {
    Table,
    ApiKeyId,
}

#[async_trait::async_trait]
impl MigrationTrait for M20260327AddApiKeyIdToMeteringLogs {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MeteringLogs::Table)
                    .add_column(ColumnDef::new(MeteringLogs::ApiKeyId).integer().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MeteringLogs::Table)
                    .drop_column(MeteringLogs::ApiKeyId)
                    .to_owned(),
            )
            .await
    }
}

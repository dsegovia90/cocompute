// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm_migration::prelude::*;

pub struct M20260325CreateMeteringLogs;

impl MigrationName for M20260325CreateMeteringLogs {
    fn name(&self) -> &str {
        "m20260325_create_metering_logs"
    }
}

#[derive(DeriveIden)]
enum MeteringLogs {
    Table,
    Id,
    HostEndpointId,
    Model,
    RequestType,
    PromptTokens,
    CompletionTokens,
    ComputeMs,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for M20260325CreateMeteringLogs {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MeteringLogs::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(MeteringLogs::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(MeteringLogs::HostEndpointId).string().not_null())
                    .col(ColumnDef::new(MeteringLogs::Model).string().not_null())
                    .col(ColumnDef::new(MeteringLogs::RequestType).string().not_null())
                    .col(ColumnDef::new(MeteringLogs::PromptTokens).integer().not_null())
                    .col(ColumnDef::new(MeteringLogs::CompletionTokens).integer().not_null())
                    .col(ColumnDef::new(MeteringLogs::ComputeMs).big_integer().not_null())
                    .col(ColumnDef::new(MeteringLogs::CreatedAt).timestamp().not_null().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(MeteringLogs::Table).to_owned()).await
    }
}

use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(M20260325CreateApiKeys),
            Box::new(M20260325CreateHosts),
            Box::new(M20260325CreateMeteringLogs),
            Box::new(M20260327AddApiKeyIdToMeteringLogs),
        ]
    }
}

struct M20260325CreateApiKeys;

impl MigrationName for M20260325CreateApiKeys {
    fn name(&self) -> &str {
        "m20260325_create_api_keys"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for M20260325CreateApiKeys {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApiKeys::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ApiKeys::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(ApiKeys::KeyHash).string().not_null().unique_key())
                    .col(ColumnDef::new(ApiKeys::CreatedAt).timestamp().not_null().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(ApiKeys::Table).to_owned()).await
    }
}

struct M20260325CreateHosts;

impl MigrationName for M20260325CreateHosts {
    fn name(&self) -> &str {
        "m20260325_create_hosts"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for M20260325CreateHosts {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Hosts::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Hosts::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Hosts::EndpointId).string().not_null().unique_key())
                    .col(ColumnDef::new(Hosts::Capabilities).json().null())
                    .col(ColumnDef::new(Hosts::Status).string().not_null().default("offline"))
                    .col(ColumnDef::new(Hosts::LastSeen).timestamp().null())
                    .to_owned(),
            )
            .await?;

        // Enable WAL mode for concurrent metering writes
        manager
            .get_connection()
            .execute_unprepared("PRAGMA journal_mode=WAL")
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Hosts::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum ApiKeys {
    Table,
    Id,
    KeyHash,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Hosts {
    Table,
    Id,
    EndpointId,
    Capabilities,
    Status,
    LastSeen,
}

struct M20260325CreateMeteringLogs;

impl MigrationName for M20260325CreateMeteringLogs {
    fn name(&self) -> &str {
        "m20260325_create_metering_logs"
    }
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
    ApiKeyId,
}

struct M20260327AddApiKeyIdToMeteringLogs;

impl MigrationName for M20260327AddApiKeyIdToMeteringLogs {
    fn name(&self) -> &str {
        "m20260327_add_api_key_id_to_metering_logs"
    }
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

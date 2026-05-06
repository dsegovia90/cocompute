use sea_orm_migration::prelude::*;

pub struct M20260325CreateHosts;

impl MigrationName for M20260325CreateHosts {
    fn name(&self) -> &str {
        "m20260325_create_hosts"
    }
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

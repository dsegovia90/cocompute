use sea_orm_migration::prelude::*;

pub struct M20260406AddTotalMsToMeteringLogs;

impl MigrationName for M20260406AddTotalMsToMeteringLogs {
    fn name(&self) -> &str {
        "m20260406_add_total_ms_to_metering_logs"
    }
}

#[derive(DeriveIden)]
enum MeteringLogs {
    Table,
    TotalMs,
    IrohRttMs,
}

#[async_trait::async_trait]
impl MigrationTrait for M20260406AddTotalMsToMeteringLogs {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MeteringLogs::Table)
                    .add_column(ColumnDef::new(MeteringLogs::TotalMs).big_integer().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(MeteringLogs::Table)
                    .add_column(ColumnDef::new(MeteringLogs::IrohRttMs).double().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MeteringLogs::Table)
                    .drop_column(MeteringLogs::TotalMs)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(MeteringLogs::Table)
                    .drop_column(MeteringLogs::IrohRttMs)
                    .to_owned(),
            )
            .await
    }
}

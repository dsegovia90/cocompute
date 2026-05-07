use sea_orm_migration::prelude::*;

pub struct M20260507AddIsActiveToHosts;

impl MigrationName for M20260507AddIsActiveToHosts {
    fn name(&self) -> &str {
        "m20260507_000001_add_is_active_to_hosts"
    }
}

#[derive(DeriveIden)]
enum Hosts {
    Table,
    IsActive,
}

#[async_trait::async_trait]
impl MigrationTrait for M20260507AddIsActiveToHosts {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Hosts::Table)
                    .add_column(
                        ColumnDef::new(Hosts::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Hosts::Table)
                    .drop_column(Hosts::IsActive)
                    .to_owned(),
            )
            .await
    }
}

use sea_orm_migration::prelude::*;

pub struct M20260412AddUserToHosts;

impl MigrationName for M20260412AddUserToHosts {
    fn name(&self) -> &str {
        "m20260412_000006_add_user_to_hosts"
    }
}

#[derive(DeriveIden)]
enum Hosts {
    Table,
    UserId,
}

#[async_trait::async_trait]
impl MigrationTrait for M20260412AddUserToHosts {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(
            Table::alter()
                .table(Hosts::Table)
                .add_column(ColumnDef::new(Hosts::UserId).integer().null())
                .to_owned(),
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(
            Table::alter().table(Hosts::Table).drop_column(Hosts::UserId).to_owned(),
        ).await
    }
}

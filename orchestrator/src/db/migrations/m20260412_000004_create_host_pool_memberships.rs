use sea_orm_migration::prelude::*;

pub struct M20260412CreateHostPoolMemberships;

impl MigrationName for M20260412CreateHostPoolMemberships {
    fn name(&self) -> &str {
        "m20260412_000004_create_host_pool_memberships"
    }
}

#[derive(DeriveIden)]
enum HostPoolMemberships {
    Table,
    Id,
    HostEndpointId,
    PoolId,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for M20260412CreateHostPoolMemberships {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HostPoolMemberships::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(HostPoolMemberships::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(HostPoolMemberships::HostEndpointId).string().not_null())
                    .col(ColumnDef::new(HostPoolMemberships::PoolId).integer().not_null())
                    .col(ColumnDef::new(HostPoolMemberships::CreatedAt).timestamp().not_null().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        // Unique constraint on (host_endpoint_id, pool_id)
        manager
            .create_index(
                Index::create()
                    .name("idx_host_pool_unique")
                    .table(HostPoolMemberships::Table)
                    .col(HostPoolMemberships::HostEndpointId)
                    .col(HostPoolMemberships::PoolId)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(HostPoolMemberships::Table).to_owned()).await
    }
}

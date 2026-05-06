use sea_orm_migration::prelude::*;

pub struct M20260412CreatePoolMembers;

impl MigrationName for M20260412CreatePoolMembers {
    fn name(&self) -> &str {
        "m20260412_000002_create_pool_members"
    }
}

#[derive(DeriveIden)]
enum PoolMembers {
    Table,
    Id,
    PoolId,
    UserId,
    Role,
    InvitedAt,
    AcceptedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for M20260412CreatePoolMembers {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PoolMembers::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PoolMembers::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(PoolMembers::PoolId).integer().not_null())
                    .col(ColumnDef::new(PoolMembers::UserId).integer().not_null())
                    .col(ColumnDef::new(PoolMembers::Role).string().not_null())
                    .col(ColumnDef::new(PoolMembers::InvitedAt).timestamp().not_null().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(PoolMembers::AcceptedAt).timestamp().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(PoolMembers::Table).to_owned()).await
    }
}

use sea_orm_migration::prelude::*;

pub struct M20260412CreatePools;

impl MigrationName for M20260412CreatePools {
    fn name(&self) -> &str {
        "m20260412_000001_create_pools"
    }
}

#[derive(DeriveIden)]
enum Pools {
    Table,
    Id,
    Pid,
    Name,
    OwnerId,
    IsGlobal,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for M20260412CreatePools {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Pools::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Pools::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Pools::Pid).string().not_null().unique_key())
                    .col(ColumnDef::new(Pools::Name).string().not_null())
                    .col(ColumnDef::new(Pools::OwnerId).integer().not_null())
                    .col(ColumnDef::new(Pools::IsGlobal).boolean().not_null().default(false))
                    .col(ColumnDef::new(Pools::CreatedAt).timestamp().not_null().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        // Seed the global pool
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Pools::Table)
                    .columns([Pools::Pid, Pools::Name, Pools::OwnerId, Pools::IsGlobal])
                    .values_panic(["global".into(), "Global Pool".into(), 0.into(), true.into()])
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Pools::Table).to_owned()).await
    }
}

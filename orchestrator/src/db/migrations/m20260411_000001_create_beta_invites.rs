use sea_orm_migration::prelude::*;

pub struct M20260411CreateBetaInvites;

impl MigrationName for M20260411CreateBetaInvites {
    fn name(&self) -> &str {
        "m20260411_000001_create_beta_invites"
    }
}

#[derive(DeriveIden)]
enum BetaInvites {
    Table,
    Id,
    Email,
    Role,
    GpuInfo,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for M20260411CreateBetaInvites {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BetaInvites::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(BetaInvites::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(BetaInvites::Email).string().not_null().unique_key())
                    .col(ColumnDef::new(BetaInvites::Role).string().not_null().default("consumer"))
                    .col(ColumnDef::new(BetaInvites::GpuInfo).string().null())
                    .col(ColumnDef::new(BetaInvites::CreatedAt).timestamp().not_null().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(BetaInvites::Table).to_owned()).await
    }
}

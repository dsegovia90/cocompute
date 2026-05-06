use sea_orm_migration::prelude::*;

pub struct M20260325CreateApiKeys;

impl MigrationName for M20260325CreateApiKeys {
    fn name(&self) -> &str {
        "m20260325_create_api_keys"
    }
}

#[derive(DeriveIden)]
enum ApiKeys {
    Table,
    Id,
    KeyHash,
    CreatedAt,
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

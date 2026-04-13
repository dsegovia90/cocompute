use sea_orm_migration::prelude::*;

pub struct M20260411CreateUsers;

impl MigrationName for M20260411CreateUsers {
    fn name(&self) -> &str {
        "m20260411_000002_create_users"
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Pid,
    Email,
    PasswordHash,
    Name,
    EmailVerificationToken,
    EmailVerificationSentAt,
    EmailVerifiedAt,
    ResetToken,
    ResetSentAt,
    CreatedAt,
    UpdatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for M20260411CreateUsers {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Users::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Users::Pid).string().not_null().unique_key())
                    .col(ColumnDef::new(Users::Email).string().not_null().unique_key())
                    .col(ColumnDef::new(Users::PasswordHash).string().not_null())
                    .col(ColumnDef::new(Users::Name).string().not_null())
                    .col(ColumnDef::new(Users::EmailVerificationToken).string().null())
                    .col(ColumnDef::new(Users::EmailVerificationSentAt).timestamp().null())
                    .col(ColumnDef::new(Users::EmailVerifiedAt).timestamp().null())
                    .col(ColumnDef::new(Users::ResetToken).string().null())
                    .col(ColumnDef::new(Users::ResetSentAt).timestamp().null())
                    .col(ColumnDef::new(Users::CreatedAt).timestamp().not_null().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(Users::UpdatedAt).timestamp().not_null().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Users::Table).to_owned()).await
    }
}

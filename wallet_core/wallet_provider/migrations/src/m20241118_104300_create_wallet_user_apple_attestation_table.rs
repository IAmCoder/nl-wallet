use async_trait::async_trait;
use sea_orm_migration::prelude::*;

use crate::m20230616_000001_create_wallet_user_table::WalletUser;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WalletUserAppleAttestation::Table)
                    .col(
                        ColumnDef::new(WalletUserAppleAttestation::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WalletUserAppleAttestation::WalletUserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WalletUserAppleAttestation::AssertionCounter)
                            .big_integer()
                            .not_null()
                            .default(0)
                            .check(
                                // Emulate a u32 with a CHECK constraint, since
                                // PostgreSQL does not support unsigned integers.
                                Expr::col(WalletUserAppleAttestation::AssertionCounter)
                                    .gte(0)
                                    .and(Expr::col(WalletUserAppleAttestation::AssertionCounter).lte(u32::MAX)),
                            ),
                    )
                    .col(
                        ColumnDef::new(WalletUserAppleAttestation::AttestationData)
                            .binary()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_wallet_user_id")
                            .from(
                                WalletUserAppleAttestation::Table,
                                WalletUserAppleAttestation::WalletUserId,
                            )
                            .to(WalletUser::Table, WalletUser::Id)
                            .on_delete(ForeignKeyAction::NoAction),
                    )
                    .index(
                        Index::create()
                            .unique()
                            .name("wallet_user_apple_attestation_unique_wallet_user_id")
                            .col(WalletUserAppleAttestation::WalletUserId),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum WalletUserAppleAttestation {
    Table,
    Id,
    WalletUserId,
    AssertionCounter,
    AttestationData,
}

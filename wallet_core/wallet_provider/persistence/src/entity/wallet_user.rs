//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.1

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "wallet_user")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub wallet_id: String,
    #[sea_orm(column_type = "VarBinary(StringLen::None)")]
    pub hw_pubkey_der: Vec<u8>,
    #[sea_orm(column_type = "VarBinary(StringLen::None)")]
    pub encrypted_pin_pubkey_sec1: Vec<u8>,
    #[sea_orm(column_type = "VarBinary(StringLen::None)")]
    pub pin_pubkey_iv: Vec<u8>,
    #[sea_orm(column_type = "VarBinary(StringLen::None)", nullable)]
    pub encrypted_previous_pin_pubkey_sec1: Option<Vec<u8>>,
    #[sea_orm(column_type = "VarBinary(StringLen::None)", nullable)]
    pub previous_pin_pubkey_iv: Option<Vec<u8>>,
    pub instruction_sequence_number: i32,
    pub pin_entries: i16,
    pub last_unsuccessful_pin: Option<DateTimeWithTimeZone>,
    pub is_blocked: bool,
    pub has_wte: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::wallet_user_apple_attestation::Entity")]
    WalletUserAppleAttestation,
    #[sea_orm(has_one = "super::wallet_user_instruction_challenge::Entity")]
    WalletUserInstructionChallenge,
    #[sea_orm(has_many = "super::wallet_user_key::Entity")]
    WalletUserKey,
}

impl Related<super::wallet_user_apple_attestation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WalletUserAppleAttestation.def()
    }
}

impl Related<super::wallet_user_instruction_challenge::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WalletUserInstructionChallenge.def()
    }
}

impl Related<super::wallet_user_key::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WalletUserKey.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

use chrono::{DateTime, Local};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, Eq, PartialEq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum TransactionType {
    #[sea_orm(string_value = "PidIssuance")]
    PidIssuance,
    #[sea_orm(string_value = "Disclosure")]
    Disclosure,
}

#[derive(Clone, Debug, Eq, PartialEq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum TransactionStatus {
    #[sea_orm(string_value = "Success")]
    Success,
    #[sea_orm(string_value = "Error")]
    Error,
    #[sea_orm(string_value = "Cancelled")]
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "transaction")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub r#type: TransactionType,
    pub timestamp: DateTime<Local>,
    pub remote_party_certificate: Option<Vec<u8>>,
    pub status: TransactionStatus,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

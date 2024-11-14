use sea_orm::entity::prelude::*;

use crate::history_doc_type;
use crate::issuance_history_event;

#[derive(Clone, Debug, Eq, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "issuance_history_event_doc_type")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub issuance_history_event_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub history_doc_type_id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    HistoryEvent,
    HistoryDocType,
}

impl ActiveModelBehavior for ActiveModel {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::HistoryEvent => Entity::belongs_to(issuance_history_event::Entity)
                .from(Column::IssuanceHistoryEventId)
                .to(issuance_history_event::Column::Id)
                .into(),
            Self::HistoryDocType => Entity::belongs_to(history_doc_type::Entity)
                .from(Column::HistoryDocTypeId)
                .to(history_doc_type::Column::Id)
                .into(),
        }
    }
}

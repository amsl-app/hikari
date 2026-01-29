use sea_orm::entity::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "llm_step_state_enum")]
pub enum Status {
    #[sea_orm(string_value = "running")]
    Running,
    #[sea_orm(string_value = "waiting_for_input")]
    WaitingForInput,
    #[sea_orm(string_value = "completed")]
    Completed,
    #[sea_orm(string_value = "error")]
    Error,
    #[sea_orm(string_value = "not_started")]
    NotStarted,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "llm_conversation_state")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub conversation_id: Uuid,

    pub step_state: Status,

    pub current_step: String,

    pub value: Option<String>,

    pub last_interaction_at: DateTime,
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::conversation::Entity",
        from = "Column::ConversationId",
        to = "super::conversation::Column::ConversationId"
    )]
    Conversation,
}

impl Related<super::conversation::Entity> for Entity {
    fn to() -> RelationDef {
        crate::llm::slot::conversation_slot::Relation::Conversation.def()
    }
}

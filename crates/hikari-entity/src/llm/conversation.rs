use sea_orm::entity::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "conversation_status_enum")]
pub enum Status {
    #[sea_orm(string_value = "open")]
    Open,
    #[sea_orm(string_value = "completed")]
    Completed,
    #[sea_orm(string_value = "closed")]
    Closed,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "llm_conversation")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub conversation_id: Uuid,

    pub user_id: Uuid,

    pub module_id: String,

    pub session_id: String,

    pub created_at: DateTime,

    pub completed_at: Option<DateTime>,

    pub status: Status,
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::message::Entity")]
    Message,
    #[sea_orm(has_many = "super::slot::conversation_slot::Entity")]
    Slot,
    #[sea_orm(has_one = "super::conversation_state::Entity")]
    ConversationState,
    #[sea_orm(
        belongs_to = "crate::user::Entity",
        from = "Column::UserId",
        to = "crate::user::Column::Id"
    )]
    User,
}

impl Related<crate::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::message::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Message.def()
    }
}

impl Related<super::slot::conversation_slot::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Slot.def()
    }
}

impl Related<super::conversation_state::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ConversationState.def()
    }
}

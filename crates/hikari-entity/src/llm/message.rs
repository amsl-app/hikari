use sea_orm::entity::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "message_status_enum")]
pub enum Status {
    #[sea_orm(string_value = "generating")]
    Generating,
    #[sea_orm(string_value = "completed")]
    Completed,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "message_direction_enum")]
pub enum Direction {
    #[sea_orm(string_value = "send")]
    Send,
    #[sea_orm(string_value = "receive")]
    Receive,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "content_type_enum")]
pub enum ContentType {
    #[sea_orm(string_value = "text")]
    Text,
    #[sea_orm(string_value = "payload")]
    Payload,
    #[sea_orm(string_value = "buttons")]
    Buttons,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "llm_message")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub conversation_id: Uuid,

    #[sea_orm(primary_key)]
    pub message_order: i32,

    pub step: String,

    pub created_at: DateTime,

    pub content_type: ContentType,

    pub payload: String,

    pub direction: Direction,

    pub status: Status,
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
        Relation::Conversation.def()
    }
}

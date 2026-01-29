use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, NotSet, PaginatorTrait, QueryFilter, Set,
};
use uuid::Uuid;

use hikari_entity::llm::message::{ContentType, Direction, Status};
use hikari_entity::llm::{message, message::Model as MessageModel};

pub struct Mutation;

impl Mutation {
    pub async fn insert_new_message(
        db: &DatabaseConnection,
        conversation_id: Uuid,
        step: String,
        content_type: ContentType,
        payload: String,
        direction: Direction,
        status: Status,
    ) -> Result<MessageModel, DbErr> {
        let message_count = message::Entity::find()
            .filter(message::Column::ConversationId.eq(conversation_id))
            .count(db)
            .await?;
        let message_count = i32::try_from(message_count)
            .map_err(|_| DbErr::Custom("message count is too large to fit in a u16".to_string()))?;

        let new_message = message::ActiveModel {
            conversation_id: Set(conversation_id),
            message_order: Set(message_count),
            step: Set(step),
            created_at: Set(Utc::now().naive_utc()),
            content_type: Set(content_type),
            payload: Set(payload),
            direction: Set(direction),
            status: Set(status),
        };
        new_message.insert(db).await
    }

    pub async fn update_message(
        db: &DatabaseConnection,
        conversation_id: Uuid,
        message_order: i32,
        payload: String,
        status: Option<Status>,
    ) -> Result<MessageModel, DbErr> {
        let model = message::ActiveModel {
            conversation_id: Set(conversation_id),
            message_order: Set(message_order),
            payload: Set(payload),
            status: status.map_or(NotSet, Set),
            ..Default::default()
        };
        model.update(db).await
    }
}

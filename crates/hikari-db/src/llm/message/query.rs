use hikari_entity::llm::message::{self, Entity as Message, Model as MessageModel, Status};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};
use std::error::Error;
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn get_memory_from_conversation(
        db: &DatabaseConnection,
        conversation_id: &Uuid,
        steps: Option<&[String]>,
        limit: Option<u64>,
    ) -> Result<Vec<MessageModel>, DbErr> {
        let mut query = Message::find()
            .filter(message::Column::ConversationId.eq(*conversation_id))
            .filter(message::Column::Status.eq(Status::Completed));
        if let Some(steps) = steps {
            query = query.filter(message::Column::Step.is_in(steps));
        }
        // Get newest messages first (if we limit the number of messages)
        query = query.order_by_desc(message::Column::MessageOrder);
        if let Some(limit) = limit {
            query = query.limit(limit);
        }
        let mut messages = query.all(db).await.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, "failed to load messages");
        })?;
        // Then revers the order to get the oldest messages first
        messages.reverse();
        Ok(messages)
    }

    pub async fn get_not_finished_message<C: ConnectionTrait + TransactionTrait>(
        db: &C,
        conversation_id: Uuid,
        step: &str,
    ) -> Result<Option<MessageModel>, DbErr> {
        Message::find()
            .filter(message::Column::ConversationId.eq(conversation_id))
            .filter(message::Column::Status.eq(Status::Generating))
            .filter(message::Column::Step.eq(step))
            .order_by_desc(message::Column::CreatedAt)
            .one(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn Error, "failed to load messages");
            })
    }
}

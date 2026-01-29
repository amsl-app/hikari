use std::error::Error;

use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder, TransactionTrait,
};
use uuid::Uuid;

use hikari_entity::llm::conversation::{self, Entity as Conversation, Model as ConversationModel, Status};

pub struct Query;

impl Query {
    pub async fn get_conversation(
        db: &DatabaseConnection,
        conversation_id: Uuid,
    ) -> Result<Option<ConversationModel>, DbErr> {
        Conversation::find_by_id(conversation_id)
            .order_by_desc(conversation::Column::CreatedAt)
            .one(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn Error, "failed to load conversation");
            })
    }

    pub async fn get_all_conversations_from_user(
        db: &DatabaseConnection,
        user_id: Uuid,
    ) -> Result<Vec<ConversationModel>, DbErr> {
        Conversation::find()
            .filter(conversation::Column::UserId.eq(user_id))
            .order_by_desc(conversation::Column::CreatedAt)
            .all(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn Error, "failed to load all user conversations");
            })
    }

    pub async fn get_last_conversation_by_module_session_user<C: ConnectionTrait + TransactionTrait>(
        db: &C,
        user_id: Uuid,
        module_id: &str,
        session_id: &str,
    ) -> Result<Option<ConversationModel>, DbErr> {
        let res = Conversation::find()
            .filter(conversation::Column::UserId.eq(user_id))
            .filter(conversation::Column::ModuleId.eq(module_id))
            .filter(conversation::Column::SessionId.eq(session_id))
            .order_by_desc(conversation::Column::CreatedAt)
            .one(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn Error, "failed to load all user conversations");
            })?;
        if let Some(conv) = &res
            && conv.status == Status::Closed
        {
            return Ok(None);
        }
        Ok(res)
    }

    pub async fn get_last_conversation(
        db: &DatabaseConnection,
        user_id: Uuid,
    ) -> Result<Option<ConversationModel>, DbErr> {
        Conversation::find()
            .filter(conversation::Column::UserId.eq(user_id))
            .order_by_desc(conversation::Column::CreatedAt)
            .one(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn Error, "failed to load last conversation");
            })
    }
}

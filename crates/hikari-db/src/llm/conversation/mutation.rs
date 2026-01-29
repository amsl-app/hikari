use chrono::Utc;
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, IntoActiveValue, NotSet, QueryFilter,
    TransactionTrait,
};
use uuid::Uuid;

use hikari_entity::llm::conversation::Status;
use hikari_entity::llm::{
    conversation, conversation::Entity as Conversation, conversation::Model as ConversationModel,
};

pub struct Mutation;

impl Mutation {
    pub async fn create_conversation<C: ConnectionTrait + TransactionTrait>(
        db: &C,
        user_id: Uuid,
        module_id: String,
        session_id: String,
    ) -> Result<ConversationModel, DbErr> {
        let now = Utc::now().naive_utc();

        let conversation = conversation::ActiveModel {
            conversation_id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            module_id: Set(module_id),
            session_id: Set(session_id),
            created_at: Set(now),
            completed_at: NotSet,
            status: Set(Status::Open),
        };

        conversation.insert(db).await
    }

    pub async fn complete_conversation<C: ConnectionTrait>(conn: &C, conversation_id: Uuid) -> Result<(), DbErr> {
        let updated_values = conversation::ActiveModel {
            conversation_id: conversation_id.into_active_value(),
            status: Set(Status::Completed),
            completed_at: Set(Some(Utc::now().naive_utc())),
            ..Default::default()
        };

        let update_result = Conversation::update_many()
            .filter(conversation::Column::ConversationId.eq(conversation_id))
            .set(updated_values)
            .exec(conn)
            .await?;

        if update_result.rows_affected == 0 {
            return Err(DbErr::RecordNotFound("Llm conversation not found".to_owned()));
        }

        Ok(())
    }

    pub async fn close_open_conversations<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        module_id: &str,
        session_id: &str,
    ) -> Result<(), DbErr> {
        let updated_values = conversation::ActiveModel {
            status: Set(Status::Closed),
            ..Default::default()
        };
        Conversation::update_many()
            .filter(conversation::Column::UserId.eq(user_id))
            .filter(conversation::Column::ModuleId.eq(module_id))
            .filter(conversation::Column::SessionId.eq(session_id))
            .filter(conversation::Column::Status.eq(Status::Open))
            .set(updated_values)
            .exec(conn)
            .await?;
        Ok(())
    }
}

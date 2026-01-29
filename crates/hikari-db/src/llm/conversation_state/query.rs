use hikari_entity::llm::{
    conversation_state::Entity as ConversationState, conversation_state::Model as ConversationStateModel,
};
use sea_orm::{ConnectionTrait, DbErr, EntityTrait, TransactionTrait};
use std::error::Error;
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn get_conversation_state<C: ConnectionTrait + TransactionTrait>(
        db: &C,
        conversation_id: Uuid,
    ) -> Result<Option<ConversationStateModel>, DbErr> {
        ConversationState::find_by_id(conversation_id)
            .one(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn Error, "failed to load conversation state");
            })
    }
}

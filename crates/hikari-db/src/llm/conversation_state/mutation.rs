use chrono::Utc;
use sea_orm::ActiveValue::Set;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ConnectionTrait, DbErr, EntityTrait, IntoActiveValue, NotSet};
use uuid::Uuid;

use hikari_entity::llm::conversation_state::Status;
use hikari_entity::llm::{conversation_state, conversation_state::Entity as ConversationState};

pub struct Mutation;

impl Mutation {
    pub async fn set_current_step_for_conversation_step<C: ConnectionTrait>(
        conn: &C,
        conversation_id: Uuid,
        current_step: String,
        value: Option<String>,
    ) -> Result<(), DbErr> {
        Self::upsert_conversation_state(conn, conversation_id, None, Some(current_step), value).await
    }

    pub async fn set_status_for_conversation_state<C: ConnectionTrait>(
        conn: &C,
        conversation_id: Uuid,
        step_state: Status,
        value: Option<String>,
    ) -> Result<(), DbErr> {
        Self::upsert_conversation_state(conn, conversation_id, Some(step_state), None, value).await
    }

    pub async fn upsert_conversation_state<C: ConnectionTrait>(
        conn: &C,
        conversation_id: Uuid,
        step_state: Option<Status>,
        current_step: Option<String>,
        value: Option<String>,
    ) -> Result<(), DbErr> {
        let model = conversation_state::ActiveModel {
            conversation_id: conversation_id.into_active_value(),
            step_state: step_state.map_or(NotSet, Set),
            current_step: current_step.map_or(NotSet, Set),
            last_interaction_at: Set(Utc::now().naive_utc()),
            value: value.into_active_value(),
        };

        ConversationState::insert(model)
            .on_conflict(
                OnConflict::column(conversation_state::Column::ConversationId)
                    .update_columns([
                        conversation_state::Column::StepState,
                        conversation_state::Column::CurrentStep,
                        conversation_state::Column::Value,
                    ])
                    .to_owned(),
            )
            .exec(conn)
            .await?;

        Ok(())
    }
}

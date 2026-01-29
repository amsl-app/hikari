use super::{LlmStepContent, LlmStepResponse, LlmStepTrait};
use crate::{
    builder::{slot::SaveTarget, steps::Condition},
    execution::{error::LlmExecutionError, utils::get_slots},
};
use futures_core::future::BoxFuture;
use futures_util::FutureExt;
use hikari_config::module::llm_agent::LlmService;
use hikari_core::llm_config::LlmConfig;
use hikari_model::llm::state::{LlmConversationState, LlmStepStatus};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone)]
pub struct Counter {
    id: String,
    slot: SaveTarget,
    conditions: Vec<Condition>,
    status: LlmStepStatus,
}

impl Counter {
    #[must_use]
    pub fn new(id: String, slot: SaveTarget, conditions: Vec<Condition>) -> Self {
        Self {
            id,
            slot,
            conditions,
            status: LlmStepStatus::NotStarted,
        }
    }
}

impl LlmStepTrait for Counter {
    fn call<'a>(
        &'a mut self,
        _config: &'a LlmConfig,
        conversation_id: &'a Uuid,
        user_id: &'a Uuid,
        module_id: &'a str,
        session_id: &'a str,
        _llm_service: LlmService,
        conn: DatabaseConnection,
    ) -> BoxFuture<'a, Result<LlmStepResponse, LlmExecutionError>> {
        async move {
            let new_value = match &self.slot {
                SaveTarget::Slot(path) => {
                    let slot = get_slots(
                        &conn,
                        conversation_id,
                        user_id,
                        module_id,
                        session_id,
                        vec![path.to_owned()],
                    )
                    .await?;
                    if let Some(slot) = slot.first() {
                        if let Some(value) = slot.value.0.as_i64() {
                            Ok(value + 1)
                        } else {
                            Err(LlmExecutionError::Unexpected(format!(
                                "Slot {} does not contain a valid number",
                                path.name
                            )))
                        }
                    } else {
                        Ok(1)
                    }
                }
            }?;
            let mut values = HashMap::new();
            values.insert(self.slot.clone(), new_value.into());

            Ok(LlmStepResponse::new(
                LlmStepContent::StepValue {
                    values,
                    next_step: None,
                },
                None,
            ))
        }
        .boxed()
    }

    fn add_previous_response(&mut self, _response: String) {
        tracing::warn!("Adding previous response to retriever should not happen");
    }

    fn remove_previous_response(&mut self) {
        tracing::warn!("Removing previous response to retriever should not happen");
    }

    fn set_status(&mut self, status: LlmStepStatus) -> LlmConversationState {
        self.status = status;
        self.state()
    }

    fn finish(&mut self) -> LlmConversationState {
        self.set_status(LlmStepStatus::Completed);
        self.state()
    }

    fn status(&self) -> LlmStepStatus {
        self.status
    }

    fn conditions(&self) -> &[Condition] {
        &self.conditions
    }

    fn id(&self) -> &str {
        &self.id
    }
}

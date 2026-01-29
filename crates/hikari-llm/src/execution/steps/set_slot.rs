use futures_core::future::BoxFuture;
use futures_util::FutureExt;
use hikari_config::module::llm_agent::LlmService;
use hikari_core::llm_config::LlmConfig;
use hikari_model::llm::state::{LlmConversationState, LlmStepStatus};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::builder::slot::{SaveTarget, SlotValuePair};
use crate::builder::steps::InjectionTrait;
use crate::{
    builder::steps::Condition,
    execution::{error::LlmExecutionError, utils::get_slots},
};

use super::{LlmStepContent, LlmStepResponse, LlmStepTrait};

#[derive(Clone)]
pub struct SetSlot {
    id: String,
    values: Vec<SlotValuePair>,
    conditions: Vec<Condition>,
    status: LlmStepStatus,
}

impl SetSlot {
    #[must_use]
    pub fn new(id: String, values: Vec<SlotValuePair>, conditions: Vec<Condition>) -> Self {
        Self {
            id,
            values,
            conditions,
            status: LlmStepStatus::NotStarted,
        }
    }
}

impl LlmStepTrait for SetSlot {
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
            let injection_slots = self
                .values
                .iter()
                .flat_map(crate::builder::steps::SlotsTrait::injection_slots)
                .collect::<Vec<_>>();
            let injection_slots =
                get_slots(&conn, conversation_id, user_id, module_id, session_id, injection_slots).await?;

            let values = self
                .values
                .iter()
                .map(|s| s.inject(&injection_slots))
                .collect::<Vec<_>>();

            let values = values
                .into_iter()
                .map(|v| (SaveTarget::Slot(v.path.clone()), v.value.0))
                .collect();

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
        tracing::error!(
            "Adding previous response to set_slot should not happen, since this step does not produce a response."
        );
    }

    fn remove_previous_response(&mut self) {
        // Nothing will happen here; Function gets called at the beginning of the step
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

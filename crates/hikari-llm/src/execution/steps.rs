use super::utils::{add_usage, get_slots};
use crate::builder::slot::SaveTarget;
use crate::builder::slot::SlotValuePair;
use crate::builder::slot::paths::SlotPath;
use crate::builder::steps::Condition;
use crate::builder::steps::ConditionOperation;
use crate::execution::error::LlmExecutionError;
use crate::execution::steps::api_call::ApiCall;
use crate::execution::steps::counter::Counter;
use crate::execution::steps::go_to::GoTo;
use crate::execution::steps::sse_call::SseCall;
use combined_step::CombinedStep;
use conversation_summarizer::ConversationSummarizer;
use conversation_validator::ConversationValidator;
use futures_core::future::BoxFuture;
use futures_util::FutureExt;
use hikari_config::module::llm_agent::LlmService;
use hikari_core::llm_config::LlmConfig;
use hikari_core::openai::streaming::MessageStream;
use hikari_model::llm::state::{LlmConversationState, LlmStepStatus, StateValue};
use message_generator::MessageGenerator;
use sea_orm::DatabaseConnection;
use serde_yml::Value;
use set_slot::SetSlot;
use std::collections::HashMap;
use std::error::Error;
use text_message::TextMessage;
use thiserror::Error;
use uuid::Uuid;
use value_extractor::ValueExtractor;
use vector_db_extractor::VectorDBExtractor;

pub mod api_call;
pub mod combined_step;
pub mod conversation_summarizer;
pub mod conversation_validator;
pub mod counter;
pub mod go_to;
pub mod message_generator;
pub mod set_slot;
pub mod sse_call;
pub mod text_message;
pub mod value_extractor;
pub mod vector_db_extractor;

#[derive(Debug, Error)]
enum ConditionError<'a> {
    #[error("slot type did not have the expected type")]
    WrongSlotType,
    #[error("slot path not found {0}")]
    SlotNotFound(&'a str),
}

fn parse_f64_slot<'a>(value: Option<&'_ Value>, path: &'a SlotPath) -> Result<f64, ConditionError<'a>> {
    let Some(value) = value else {
        return Err(ConditionError::SlotNotFound(&path.name));
    };

    value.as_f64().ok_or(ConditionError::WrongSlotType)
}

pub trait LlmStepTrait: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    fn execute<'a>(
        &'a mut self,
        config: &'a LlmConfig,
        conversation_id: &'a Uuid,
        user_id: &'a Uuid,
        module_id: &'a str,
        session_id: &'a str,
        llm_service: LlmService,
        conn: DatabaseConnection,
    ) -> BoxFuture<'a, Result<LlmStepContent, LlmExecutionError>> {
        async move {
            let slots_paths = self
                .conditions()
                .iter()
                .map(|condition| condition.slot.clone())
                .collect();
            let slots = get_slots(&conn, conversation_id, user_id, module_id, session_id, slots_paths).await?;

            if !self.condition_full_filled(&slots) {
                return Ok(LlmStepContent::Skipped);
            }
            let res = self
                .call(
                    config,
                    conversation_id,
                    user_id,
                    module_id,
                    session_id,
                    llm_service.clone(),
                    conn.clone(),
                )
                .await;
            let res = match res {
                Ok(res) => Ok(res),
                Err(err) => {
                    tracing::error!(error = &err as &dyn Error, id = %self.id(), "Error in step, try one more time");
                    self.call(
                        config,
                        conversation_id,
                        user_id,
                        module_id,
                        session_id,
                        llm_service,
                        conn.clone(),
                    )
                    .await
                }
            };
            match res {
                Err(err) => {
                    tracing::error!(error = ?err, id = %self.id(), "Error in step, second try failed");
                    self.set_status(LlmStepStatus::Error);
                    Err(err)
                }
                Ok(res) => {
                    let LlmStepResponse { content, tokens } = res;
                    if let Some(tokens) = tokens {
                        add_usage(&conn, user_id, tokens, self.id().to_owned()).await?;
                    }
                    Ok(content)
                }
            }
        }
        .boxed()
    }

    #[allow(clippy::too_many_arguments)]
    fn call<'a>(
        &'a mut self,
        config: &'a LlmConfig,
        conversation_id: &'a Uuid,
        user_id: &'a Uuid,
        module_id: &'a str,
        session_id: &'a str,
        llm_service: LlmService,
        conn: DatabaseConnection,
    ) -> BoxFuture<'a, Result<LlmStepResponse, LlmExecutionError>>;

    fn add_previous_response(&mut self, response: String);
    fn remove_previous_response(&mut self);
    fn set_status(&mut self, status: LlmStepStatus) -> LlmConversationState;
    fn finish(&mut self) -> LlmConversationState;
    fn status(&self) -> LlmStepStatus;
    fn conditions(&self) -> &[Condition];
    fn id(&self) -> &str;

    fn with_state(&mut self, state: LlmConversationState) -> Result<(), LlmExecutionError> {
        if state.current_step != self.id() {
            return Err(LlmExecutionError::InvalidState);
        }
        if let Some(response) = state.value.response {
            self.add_previous_response(response);
        }
        self.set_status(state.status);
        Ok(())
    }

    fn condition_full_filled(&self, slots: &[SlotValuePair]) -> bool {
        for Condition { slot, condition } in self.conditions() {
            let value = slots.iter().find_map(|s| {
                if s.path.name == slot.name {
                    Some(s.value.as_ref())
                } else {
                    None
                }
            });
            let full_filled: Result<bool, ConditionError> = match condition {
                ConditionOperation::Equals(equals) => check_equals(value, equals, slot),
                // With this implementation, x != None is false since None is always equal to None
                ConditionOperation::NotEquals(equals) => check_equals(value, equals, slot).map(|v| !v),
                ConditionOperation::Exists(should_exists) => Ok(should_exists == &value.is_some()),
                ConditionOperation::GreaterThan(border) => parse_f64_slot(value, slot).map(|n| n > *border),
                ConditionOperation::LessThan(border) => parse_f64_slot(value, slot).map(|n| n < *border),
                ConditionOperation::GreaterThanOrEqual(border) => parse_f64_slot(value, slot).map(|n| n >= *border),
                ConditionOperation::LessThanOrEqual(border) => parse_f64_slot(value, slot).map(|n| n <= *border),
            };
            match full_filled {
                Err(error) => {
                    tracing::warn!(%error, id = %self.id(), "could not process slot condition");
                    return false;
                }
                Ok(false) => return false,
                Ok(true) => {}
            }
        }
        true
    }

    fn state(&self) -> LlmConversationState {
        LlmConversationState {
            current_step: self.id().to_owned(),
            status: self.status(),
            value: StateValue::default(), // TODO implement
        }
    }

    fn reset(&mut self) -> LlmConversationState {
        self.set_status(LlmStepStatus::default());
        self.remove_previous_response();
        self.state()
    }
}

fn check_equals<'a>(
    value: Option<&'_ Value>,
    equals: &'_ Value,
    slot: &'a SlotPath,
) -> Result<bool, ConditionError<'a>> {
    if let Some(value) = value {
        Ok(value == equals)
    } else {
        Err(ConditionError::SlotNotFound(&slot.name))
    }
}

#[derive(Clone)]
pub enum LlmStep {
    TextMessage(TextMessage),
    MessageGenerator(MessageGenerator),
    CombinedStep(CombinedStep),
    ConversationSummarizer(ConversationSummarizer),
    ConversationValidator(ConversationValidator),
    ValueExtractor(ValueExtractor),
    VectorDBExtractor(VectorDBExtractor),
    ApiCall(ApiCall),
    SseCall(SseCall),
    SetSlot(SetSlot),
    Counter(Counter),
    GoTo(GoTo),
}

impl LlmStepTrait for LlmStep {
    fn call<'a>(
        &'a mut self,
        config: &'a LlmConfig,
        conversation_id: &'a Uuid,
        user_id: &'a Uuid,
        module_id: &'a str,
        session_id: &'a str,
        llm_service: LlmService,
        conn: DatabaseConnection,
    ) -> BoxFuture<'a, Result<LlmStepResponse, LlmExecutionError>> {
        match self {
            LlmStep::TextMessage(step) => step.call(
                config,
                conversation_id,
                user_id,
                module_id,
                session_id,
                llm_service,
                conn,
            ),
            LlmStep::MessageGenerator(step) => step.call(
                config,
                conversation_id,
                user_id,
                module_id,
                session_id,
                llm_service,
                conn,
            ),
            LlmStep::CombinedStep(step) => step.call(
                config,
                conversation_id,
                user_id,
                module_id,
                session_id,
                llm_service,
                conn,
            ),
            LlmStep::ConversationSummarizer(step) => step.call(
                config,
                conversation_id,
                user_id,
                module_id,
                session_id,
                llm_service,
                conn,
            ),
            LlmStep::ConversationValidator(step) => step.call(
                config,
                conversation_id,
                user_id,
                module_id,
                session_id,
                llm_service,
                conn,
            ),
            LlmStep::ValueExtractor(step) => step.call(
                config,
                conversation_id,
                user_id,
                module_id,
                session_id,
                llm_service,
                conn,
            ),
            LlmStep::VectorDBExtractor(step) => step.call(
                config,
                conversation_id,
                user_id,
                module_id,
                session_id,
                llm_service,
                conn,
            ),
            LlmStep::ApiCall(step) => step.call(
                config,
                conversation_id,
                user_id,
                module_id,
                session_id,
                llm_service,
                conn,
            ),
            LlmStep::SseCall(step) => step.call(
                config,
                conversation_id,
                user_id,
                module_id,
                session_id,
                llm_service,
                conn,
            ),
            LlmStep::SetSlot(step) => step.call(
                config,
                conversation_id,
                user_id,
                module_id,
                session_id,
                llm_service,
                conn,
            ),
            LlmStep::Counter(step) => step.call(
                config,
                conversation_id,
                user_id,
                module_id,
                session_id,
                llm_service,
                conn,
            ),
            LlmStep::GoTo(step) => step.call(
                config,
                conversation_id,
                user_id,
                module_id,
                session_id,
                llm_service,
                conn,
            ),
        }
    }

    fn add_previous_response(&mut self, response: String) {
        match self {
            LlmStep::TextMessage(step) => step.add_previous_response(response),
            LlmStep::MessageGenerator(step) => step.add_previous_response(response),
            LlmStep::CombinedStep(step) => step.add_previous_response(response),
            LlmStep::ConversationSummarizer(step) => step.add_previous_response(response),
            LlmStep::ConversationValidator(step) => step.add_previous_response(response),
            LlmStep::ValueExtractor(step) => step.add_previous_response(response),
            LlmStep::VectorDBExtractor(step) => step.add_previous_response(response),
            LlmStep::ApiCall(step) => step.add_previous_response(response),
            LlmStep::SseCall(step) => step.add_previous_response(response),
            LlmStep::SetSlot(step) => step.add_previous_response(response),
            LlmStep::Counter(step) => step.add_previous_response(response),
            LlmStep::GoTo(step) => step.add_previous_response(response),
        }
    }

    fn remove_previous_response(&mut self) {
        match self {
            LlmStep::TextMessage(step) => step.remove_previous_response(),
            LlmStep::MessageGenerator(step) => step.remove_previous_response(),
            LlmStep::CombinedStep(step) => step.remove_previous_response(),
            LlmStep::ConversationSummarizer(step) => step.remove_previous_response(),
            LlmStep::ConversationValidator(step) => step.remove_previous_response(),
            LlmStep::ValueExtractor(step) => step.remove_previous_response(),
            LlmStep::VectorDBExtractor(step) => step.remove_previous_response(),
            LlmStep::ApiCall(step) => step.remove_previous_response(),
            LlmStep::SseCall(step) => step.remove_previous_response(),
            LlmStep::SetSlot(step) => step.remove_previous_response(),
            LlmStep::Counter(step) => step.remove_previous_response(),
            LlmStep::GoTo(step) => step.remove_previous_response(),
        }
    }

    fn set_status(&mut self, status: LlmStepStatus) -> LlmConversationState {
        match self {
            LlmStep::TextMessage(step) => step.set_status(status),
            LlmStep::MessageGenerator(step) => step.set_status(status),
            LlmStep::CombinedStep(step) => step.set_status(status),
            LlmStep::ConversationSummarizer(step) => step.set_status(status),
            LlmStep::ConversationValidator(step) => step.set_status(status),
            LlmStep::ValueExtractor(step) => step.set_status(status),
            LlmStep::VectorDBExtractor(step) => step.set_status(status),
            LlmStep::ApiCall(step) => step.set_status(status),
            LlmStep::SseCall(step) => step.set_status(status),
            LlmStep::SetSlot(step) => step.set_status(status),
            LlmStep::Counter(step) => step.set_status(status),
            LlmStep::GoTo(step) => step.set_status(status),
        }
    }

    fn finish(&mut self) -> LlmConversationState {
        match self {
            LlmStep::TextMessage(step) => step.finish(),
            LlmStep::MessageGenerator(step) => step.finish(),
            LlmStep::CombinedStep(step) => step.finish(),
            LlmStep::ConversationSummarizer(step) => step.finish(),
            LlmStep::ConversationValidator(step) => step.finish(),
            LlmStep::ValueExtractor(step) => step.finish(),
            LlmStep::VectorDBExtractor(step) => step.finish(),
            LlmStep::ApiCall(step) => step.finish(),
            LlmStep::SseCall(step) => step.finish(),
            LlmStep::SetSlot(step) => step.finish(),
            LlmStep::Counter(step) => step.finish(),
            LlmStep::GoTo(step) => step.finish(),
        }
    }

    fn status(&self) -> LlmStepStatus {
        match self {
            LlmStep::TextMessage(step) => step.status(),
            LlmStep::MessageGenerator(step) => step.status(),
            LlmStep::CombinedStep(step) => step.status(),
            LlmStep::ConversationSummarizer(step) => step.status(),
            LlmStep::ConversationValidator(step) => step.status(),
            LlmStep::ValueExtractor(step) => step.status(),
            LlmStep::VectorDBExtractor(step) => step.status(),
            LlmStep::ApiCall(step) => step.status(),
            LlmStep::SseCall(step) => step.status(),
            LlmStep::SetSlot(step) => step.status(),
            LlmStep::Counter(step) => step.status(),
            LlmStep::GoTo(step) => step.status(),
        }
    }

    fn conditions(&self) -> &[Condition] {
        match self {
            LlmStep::TextMessage(step) => step.conditions(),
            LlmStep::MessageGenerator(step) => step.conditions(),
            LlmStep::CombinedStep(step) => step.conditions(),
            LlmStep::ConversationSummarizer(step) => step.conditions(),
            LlmStep::ConversationValidator(step) => step.conditions(),
            LlmStep::ValueExtractor(step) => step.conditions(),
            LlmStep::VectorDBExtractor(step) => step.conditions(),
            LlmStep::ApiCall(step) => step.conditions(),
            LlmStep::SseCall(step) => step.conditions(),
            LlmStep::SetSlot(step) => step.conditions(),
            LlmStep::Counter(step) => step.conditions(),
            LlmStep::GoTo(step) => step.conditions(),
        }
    }

    fn id(&self) -> &str {
        match self {
            LlmStep::TextMessage(step) => step.id(),
            LlmStep::MessageGenerator(step) => step.id(),
            LlmStep::CombinedStep(step) => step.id(),
            LlmStep::ConversationSummarizer(step) => step.id(),
            LlmStep::ConversationValidator(step) => step.id(),
            LlmStep::ValueExtractor(step) => step.id(),
            LlmStep::VectorDBExtractor(step) => step.id(),
            LlmStep::ApiCall(step) => step.id(),
            LlmStep::SseCall(step) => step.id(),
            LlmStep::SetSlot(step) => step.id(),
            LlmStep::Counter(step) => step.id(),
            LlmStep::GoTo(step) => step.id(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LlmStepResponse {
    content: LlmStepContent,
    tokens: Option<u32>,
}

impl LlmStepResponse {
    #[must_use]
    pub fn new(content: LlmStepContent, tokens: Option<u32>) -> Self {
        Self { content, tokens }
    }
}

#[derive(Clone, Debug)]
pub enum LlmStepContent {
    Message {
        message: MessageStream,
        store: Option<SaveTarget>,
    },
    StepValue {
        values: HashMap<SaveTarget, Value>,
        next_step: Option<String>,
    },
    Combined(Vec<LlmStepContent>),
    Skipped,
}

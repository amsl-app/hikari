use crate::builder::slot::SaveTarget;
use crate::builder::slot::paths::Destination;
use crate::execution::agent::response::{ChatChunk, Response};
use crate::execution::bubble::BubbleAccumulator;
use crate::execution::error::LlmExecutionError;
use crate::execution::iterator::LlmStepIterator;
use crate::execution::steps::LlmStepTrait;
use crate::utils::get_memory;
use async_stream::try_stream;
use futures_core::stream::Stream;
use futures_util::{FutureExt, StreamExt};
use hikari_config::module::llm_agent::LlmService;
use hikari_core::llm_config::LlmConfig;
use hikari_core::openai::Content;
use hikari_core::usage::add_usage;
use hikari_model::chat::{Direction, TextContent, TypeSafePayload};
use hikari_model::llm::message::MessageStatus;
use hikari_model::llm::slot::Slot;
use hikari_model::llm::state::{LlmConversationState, LlmStepStatus};
use hikari_model_tools::convert::IntoDbModel;
use hikari_model_tools::convert::llm::split_payload_for_database;
use hikari_utils::values::ValueDecoder;
use sea_orm::DatabaseConnection;
use std::error::Error;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use yaml_serde::Value;

use super::steps::{LlmStep, LlmStepContent};

pub mod response;

pub struct LlmAgent {
    user_id: Uuid,
    session_id: String,
    module_id: String,
    conversation_id: Uuid,
    iterator: LlmStepIterator,
    config: LlmConfig,
    llm_service: LlmService,
    conn: DatabaseConnection,
    current_action: Option<Arc<Mutex<LlmStep>>>,
    start_time: Option<tokio::time::Instant>,
}

impl LlmAgent {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        mut iterator: LlmStepIterator,
        state: Option<LlmConversationState>,
        conversation_id: Uuid,
        user_id: Uuid,
        session_id: String,
        module_id: String,
        config: LlmConfig,
        llm_service: LlmService,
        conn: DatabaseConnection,
    ) -> Result<LlmAgent, LlmExecutionError> {
        let current_action = iterator.next();

        if let Some(state) = state {
            current_action
                .as_ref()
                .ok_or(LlmExecutionError::NoAction)?
                .lock()
                .await
                .with_state(state)?;
        }
        let agent = LlmAgent {
            user_id,
            session_id,
            module_id,
            conversation_id,
            iterator,
            config,
            llm_service,
            conn,
            current_action,
            start_time: None,
        };
        // We return the current action state to trigger the current step in the websocket if needed (if state is running / error)
        Ok(agent)
    }
    pub fn chat(
        &mut self,
        mut message: Option<TypeSafePayload>,
        history_needed: bool,
    ) -> Pin<Box<dyn Stream<Item = Result<Response, LlmExecutionError>> + '_ + Send>> {
        tracing::debug!("starting chat with LLM agent");
        Box::pin(try_stream! {
                let start_time = tokio::time::Instant::now();
                self.start_time = Some(start_time);
                if history_needed {
                    let messages = get_memory(&self.conn, &self.conversation_id, None, None).await?;
                    yield Response::History(messages);
                }
                loop {
                    let (step_id, status) = {
                        let current_action = self.current_action.as_ref().ok_or(LlmExecutionError::NoAction)?.lock().await;
                        (current_action.id().to_owned(), current_action.status())
                    };
                    tracing::trace!(?step_id, ?status, "processing chat step");
                    match status {
                         LlmStepStatus::Completed => {
                            if let Err(LlmExecutionError::Completed) = self.next().await {
                                self.set_conversation_completed().await?;
                                yield Response::ConversationEnd;
                                return;
                            }
                            continue;
                        }
                        LlmStepStatus::WaitingForInput => {
                            if let Some(message) = message.take() {
                                let (content_type, message) = split_payload_for_database(message).map_err(|e| LlmExecutionError::Unexpected(e.to_string()))?;
                                hikari_db::llm::message::Mutation::insert_new_message(
                                    &self.conn,
                                    self.conversation_id,
                                    step_id,
                                    content_type,
                                    message,
                                    Direction::Receive.into_db_model(),
                                    MessageStatus::Completed.into_db_model(),
                                )
                                .await?;

                                // Now the action is completed
                                let mut current_action = self.current_action.as_ref().ok_or(LlmExecutionError::NoAction)?.lock().await;
                                let state = current_action.set_status(LlmStepStatus::Completed);
                                self.set_step_state(state).await?;
                                continue;
                            }
                            // If we already take the message, we need to wait for the next one
                            yield Response::Hold;
                            break;
                        }
                        LlmStepStatus::Error => {
                            tracing::trace!("Retry step due to previous error");
                        }
                        LlmStepStatus::Running => {
                            tracing::trace!("Continue step due to interrupted conversation");
                        }
                        LlmStepStatus::NotStarted => {
                            tracing::trace!("Begin step");
                        }
                    }
                    yield Response::Typing;
                    self.set_running(step_id.as_str()).await?;
                    let response = {
                        let mut current_action = self.current_action.as_ref().ok_or(LlmExecutionError::NoAction)?.lock().await;
                        let resp = current_action.execute(&self.config, &self.conversation_id, &self.user_id, &self.module_id, &self.session_id, self.llm_service.clone(), self.conn.clone()).await;
                        let state = current_action.state();
                        self.set_step_state(state).await?;
                        resp?
                    };
                    tracing::trace!(?response, "response");
                    let mut handle = self.handle_response(response, step_id.as_str());
                    while let Some(item) = handle.next().await {
                        let item = item?;
                        if let Some(item) = item {
                            yield item;
                        }
                    }
                }
                // The precision loss is fine here, as we are only using it for metrics.
                // TODO use as_millis_f64() once it is stable
                #[allow(clippy::cast_precision_loss)]
                metrics::histogram!("agent_time_to_last_token_ms").record(start_time.elapsed().as_millis() as f64);
        })
    }

    fn handle_response<'a>(
        &'a mut self,
        response: LlmStepContent,
        step_id: &'a str,
    ) -> Pin<Box<dyn Stream<Item = Result<Option<Response>, LlmExecutionError>> + 'a + Send>> {
        Box::pin(try_stream! {
                match response {
                LlmStepContent::Skipped => {
                    tracing::debug!("Skipped step");
                    let mut current_action = self.current_action.as_ref().ok_or(LlmExecutionError::NoAction)?.lock().await;
                    let state = current_action.set_status(LlmStepStatus::Completed);
                    self.set_step_state(state).await?;
                    yield None;
                }
                LlmStepContent::Combined(combined_steps) => {
                    for step in combined_steps {
                        let mut response = self.handle_response(step, step_id);
                        while let Some(item) = response.next().await {
                            yield item?;
                        }
                    }
                }
                LlmStepContent::Message{message, store} => {
                    let conn = self.conn.clone();
                    let conversation_id = self.conversation_id;
                    let step_id_owned = step_id.to_owned();
                    let mut acc = BubbleAccumulator::new(move |content, id, is_last| {
                        let conn = conn.clone();
                        let step_id_owned = step_id_owned.clone();
                        async move {
                            let status = if is_last { MessageStatus::Completed } else { MessageStatus::Generating };
                            if let Some(id) = id {
                                let (_, message) = split_payload_for_database(TypeSafePayload::Text(TextContent { text: content }))
                                    .map_err(|e| LlmExecutionError::Unexpected(e.to_string()))?;
                                hikari_db::llm::message::Mutation::update_message(
                                    &conn,
                                    conversation_id,
                                    id,
                                    message,
                                    Some(status.into_db_model()),
                                ).await?;
                                Ok::<i32, LlmExecutionError>(id)
                            } else {
                                let (content_type, message) = split_payload_for_database(TypeSafePayload::Text(TextContent { text: content }))
                                    .map_err(|e| LlmExecutionError::Unexpected(e.to_string()))?;
                                let res = hikari_db::llm::message::Mutation::insert_new_message(
                                    &conn,
                                    conversation_id,
                                    step_id_owned,
                                    content_type,
                                    message,
                                    Direction::Send.into_db_model(),
                                    status.into_db_model(),
                                ).await?;
                                Ok(res.message_order)
                            }
                        }.boxed()
                    });




                    while let Some(result) = message.next().await {
                        match result {
                            Ok(value) => {
                                if let Some(start_time) = self.start_time.take() {
                                    // The precision loss is fine here, as we are only using it for metrics.
                                    // TODO use as_millis_f64() once it is stable
                                    #[allow(clippy::cast_precision_loss)]
                                    let elapsed = start_time.elapsed().as_millis() as f64;
                                    metrics::histogram!("agent_time_to_first_token_ms").record(elapsed);
                                }
                                tracing::trace!(?value, "message chunk");

                                let content = if let Content::Text { text, .. } = &value.content {
                                    Ok(text.as_deref().unwrap_or(""))
                                } else {
                                    Err(LlmExecutionError::UnexpectedResponseFormat)
                                }?;

                                let usage = value.tokens.unwrap_or(0);
                                tracing::trace!(?usage, "tokens used");
                                add_usage(&self.conn, &self.user_id, usage, step_id).await?;

                                // Push the new content
                                let bubble_chunks = acc.push(content).await?;

                                for bubble in bubble_chunks {
                                    let bubble_id = bubble.1;
                                    let delta = bubble.0;
                                    yield Some(Response::Chat(ChatChunk::new(delta, bubble_id, step_id.to_owned())));
                                }

                            }
                            Err(error) => {
                                tracing::error!(error = &*error as &dyn Error, "error sending streaming message");
                                self.set_error(step_id).await?;
                                Err(error)?;
                                return;
                            }
                        }
                    }

                    // Finalize the last bubbble for the database
                    let complete_message = acc.finalize().await?;


                    if let Some(SaveTarget::Slot(slot_path)) = store {
                        let destination = slot_path.destination().clone();
                        let slot = Slot {
                            name: slot_path.name,
                            value: Value::String(complete_message),
                        };
                        self.set_slot(slot, destination).await?;
                    }
                    self.set_finished(step_id).await?;
                }
                LlmStepContent::StepValue{values, next_step} => {
                    for (target, value) in values {
                        match target {
                            SaveTarget::Slot(slot_path) => {
                                let destination = slot_path.destination().clone();
                                let slot = Slot {
                                    name: slot_path.name,
                                    value,
                                };
                                self.set_slot(slot, destination).await?;
                            }
                        }
                    }
                    self.set_finished(step_id).await?;

                    if let Some(next_step) = next_step {
                        self.goto(&next_step).await?;
                    }

                    yield None;
                }
            }
        })
    }

    // Slots
    async fn set_slot(&self, slot: Slot, destination: Destination) -> Result<(), LlmExecutionError> {
        // Convert the JSON value to a string for database storage
        tracing::debug!(?slot, ?destination, "Setting slot");
        let value_string = slot.value.encode();

        match destination {
            Destination::Global => {
                hikari_db::llm::slot::global_slot::Mutation::insert_or_update_global_slot(
                    &self.conn,
                    self.user_id,
                    slot.name.clone(),
                    value_string,
                )
                .await?;
            }
            Destination::Conversation => {
                hikari_db::llm::slot::conversation_slot::Mutation::insert_or_update_slot(
                    &self.conn,
                    self.conversation_id,
                    slot.name.clone(),
                    value_string,
                )
                .await?;
            }
            Destination::Session => {
                hikari_db::llm::slot::session_slot::Mutation::insert_or_update_session_slot(
                    &self.conn,
                    self.user_id,
                    self.module_id.clone(),
                    self.session_id.clone(),
                    slot.name.clone(),
                    value_string,
                )
                .await?;
            }
            Destination::Module => {
                hikari_db::llm::slot::module_slot::Mutation::insert_or_update_module_slot(
                    &self.conn,
                    self.user_id,
                    self.module_id.clone(),
                    slot.name.clone(),
                    value_string,
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn set_running(&self, step_id: &str) -> Result<(), LlmExecutionError> {
        // State changed after execution
        let action = self.iterator.get_step(step_id).ok_or(LlmExecutionError::NoAction)?;
        let mut action = action.lock().await;

        let state = action.set_status(LlmStepStatus::Running); // Start resets the state
        tracing::trace!(current_action = %action.id(), "setting state to running");
        self.set_step_state(state).await
    }

    async fn set_finished(&self, step_id: &str) -> Result<(), LlmExecutionError> {
        // State changed after execution
        let action = self.iterator.get_step(step_id).ok_or(LlmExecutionError::NoAction)?;
        let mut action = action.lock().await;

        let state = action.finish();
        tracing::trace!(current_action = %action.id(), "setting state to finished");
        self.set_step_state(state).await
    }

    async fn set_error(&self, step_id: &str) -> Result<(), LlmExecutionError> {
        // State changed after execution
        let action = self.iterator.get_step(step_id).ok_or(LlmExecutionError::NoAction)?;
        let mut action = action.lock().await;

        let state = action.set_status(LlmStepStatus::Error);
        tracing::trace!(current_action = %action.id(), "setting state to error");
        self.set_step_state(state).await
    }

    // Status
    async fn set_step_state(&self, state: LlmConversationState) -> Result<(), LlmExecutionError> {
        tracing::trace!(?state, "set step state");
        hikari_db::llm::conversation_state::Mutation::upsert_conversation_state(
            &self.conn,
            self.conversation_id,
            Some(state.status.into_db_model()),
            Some(state.current_step),
            None,
        )
        .await?;
        Ok(())
    }

    async fn set_conversation_completed(&self) -> Result<(), LlmExecutionError> {
        hikari_db::llm::conversation::Mutation::complete_conversation(&self.conn, self.conversation_id).await?;
        tracing::debug!("Set conversation state to completed");
        Ok(())
    }

    // Action

    async fn next(&mut self) -> Result<(), LlmExecutionError> {
        tracing::trace!("Next step");
        self.current_action = self.iterator.next();
        let mut lock_guard = self
            .current_action
            .as_ref()
            .ok_or(LlmExecutionError::Completed)?
            .lock()
            .await;
        tracing::trace!(current_step = ?lock_guard.id(), "Next step");
        let state = lock_guard.reset(); // Start resets the state
        self.set_step_state(state).await?;
        Ok(())
    }

    async fn goto(&mut self, step_id: &str) -> Result<(), LlmExecutionError> {
        tracing::trace!(?step_id, "goto");
        self.iterator.goto(step_id)?;
        self.current_action = self.iterator.next();
        let mut lock_guard = self
            .current_action
            .as_ref()
            .ok_or(LlmExecutionError::Completed)?
            .lock()
            .await;
        tracing::trace!(current_step = ?lock_guard.id(), "going to step");
        let state = lock_guard.reset(); // Start resets the state
        self.set_step_state(state).await?;
        Ok(())
    }
}

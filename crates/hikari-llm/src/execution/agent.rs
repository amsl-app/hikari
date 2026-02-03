use crate::builder::slot::SaveTarget;
use crate::builder::slot::paths::Destination;
use crate::execution::agent::response::{ChatChunk, Response};
use crate::execution::error::LlmExecutionError;
use crate::execution::iterator::LlmStepIterator;
use crate::execution::steps::LlmStepTrait;
use crate::execution::utils::{add_usage, get_memory};
use async_stream::try_stream;
use futures_core::stream::Stream;
use futures_util::StreamExt;
use hikari_config::module::llm_agent::LlmService;
use hikari_core::llm_config::LlmConfig;
use hikari_core::openai::Content;
use hikari_core::tts::config::TTSConfig;
use hikari_core::tts::message_stream_to_combined_stream_cached;
use hikari_core::tts::streaming::{CombinedStream, CombinedStreamItem};
use hikari_model::chat::{Direction, TextContent, TypeSafePayload};
use hikari_model::llm::message::{ConversationMessage, MessageStatus};
use hikari_model::llm::slot::Slot;
use hikari_model::llm::state::{LlmConversationState, LlmStepStatus};
use hikari_model_tools::convert::llm::split_payload_for_database;
use hikari_model_tools::convert::{IntoDbModel, IntoModel};
use hikari_utils::values::ValueDecoder;
use sea_orm::DatabaseConnection;
use serde_yml::Value;
use std::error::Error;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use super::steps::{LlmStep, LlmStepContent};

pub mod response;

const BUFFER_SIZE: usize = 16;
const AUDIO_CHUNK_SIZE: usize = 4096;

pub struct LlmAgent {
    user_id: Uuid,
    session_id: String,
    module_id: String,
    conversation_id: Uuid,
    iterator: LlmStepIterator,
    config: LlmConfig,
    tts_config: Option<TTSConfig>,
    llm_service: LlmService,
    conn: DatabaseConnection,
    current_action: Option<Arc<Mutex<LlmStep>>>,
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
        tts_config: Option<TTSConfig>,
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
            tts_config,
            llm_service,
            conn,
            current_action,
        };
        // We return the current action state to trigger the current step in the websocket if needed (if state is running / error)
        Ok(agent)
    }

    pub fn chat(
        &mut self,
        mut message: Option<TypeSafePayload>,
        history_needed: bool,
        voice_mode: bool,
    ) -> Pin<Box<dyn Stream<Item = Result<Response, LlmExecutionError>> + '_ + Send>> {
        tracing::debug!("starting chat with LLM agent");
        Box::pin(try_stream! {
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
                                self.add_message_to_memory(step_id, message.clone(), Direction::Receive, MessageStatus::Completed).await?;
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
                    let mut handle = self.handle_response(response, voice_mode, step_id.as_str());
                    while let Some(item) = handle.next().await {
                        let item = item?;
                        if let Some(item) = item {
                            yield item;
                        }
                    }
            }
        })
    }

    fn handle_response<'a>(
        &'a mut self,
        response: LlmStepContent,
        voice_mode: bool,
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
                        let mut response = self.handle_response(step, voice_mode, step_id);
                        while let Some(item) = response.next().await {
                            yield item?;
                        }
                    }
                }
                LlmStepContent::Message{mut message, store} => {
                     let mut combined_stream: CombinedStream = if voice_mode {
                         let tts_config = self.tts_config.as_ref().ok_or(LlmExecutionError::TextToSpeechNotConfigured)?;
                            message_stream_to_combined_stream_cached(message, Arc::new(self.conn.clone()), Arc::new(tts_config.clone()))
                        } else {
                            Box::pin(try_stream! {
                                while let Some(data) = message.next().await {
                                    let data = data?;
                                    yield CombinedStreamItem::Message(data);
                                }
                              })
                        };
                    let mut complete_message = String::new(); // Initialize a complete message buffer
                    let mut message_offset: usize = 0;
                    let mut complete_audio: Vec<u8> = Vec::new(); // Initialize a complete message buffer
                    let mut audio_offset: usize = 0;
                    // Init a new message in the database
                    let mut id = None;
                    // Create streaming content with the current message chunk
                    while let Some(result) = combined_stream.next().await {
                        match result {
                            Ok(message) => {
                                tracing::trace!(?message, "message chunk");
                                match message {
                                    CombinedStreamItem::Message(message) => {
                                        let content = if let Content::Text(content) = &message.content {
                                                Ok(content)
                                        } else {
                                            Err(LlmExecutionError::UnexpectedResponseFormat)
                                        }?;
                                        complete_message.push_str(content.as_str());
                                        let usage = message.tokens.unwrap_or(0);
                                        tracing::trace!(?usage, "tokens used");
                                        add_usage(&self.conn, &self.user_id, usage, step_id.to_owned()).await?;
                                    },
                                    CombinedStreamItem::Audio(audio) => {
                                        complete_audio.extend(audio);
                                    }
                                }

                                if complete_message.len() > message_offset.saturating_add(BUFFER_SIZE) || complete_audio.len() > audio_offset.saturating_add(AUDIO_CHUNK_SIZE) {
                                        let payload = TypeSafePayload::Text(TextContent{ text: complete_message.clone()});
                                        let id = match id {
                                            None => {
                                                // Only create a message, when we have something to send
                                                let new_id = self
                                                    .add_message_to_memory(
                                                        step_id.to_owned(),
                                                        payload,
                                                        Direction::Send,
                                                        MessageStatus::Generating,
                                                    )
                                                    .await?
                                                    .message_order;
                                                id = Some(new_id);
                                                new_id
                                            }
                                            Some(id) => {
                                                self.update_message(id, payload, Some(MessageStatus::Generating))
                                                    .await?;
                                                id
                                            }
                                        };
                                        let chunk = ChatChunk::new(
                                            complete_message[message_offset..].to_string(),
                                            complete_audio[audio_offset..].to_owned(),
                                            false,
                                            id,
                                            step_id.to_owned(),
                                        );
                                        yield Some(Response::Chat(chunk));
                                        message_offset = complete_message.len();
                                        audio_offset = complete_audio.len();
                                    }
                            }
                            Err(error) => {
                                tracing::error!(error = &error as &dyn Error, "error sending streaming message");
                                self.set_error(step_id).await?;
                                Err(error)?;
                                return;
                            }
                        }
                    }


                    match id {
                        Some(id) => {
                            let message = complete_message[message_offset..].to_string();
                            let audio = complete_audio[audio_offset..].to_owned();
                            let payload = TypeSafePayload::Text(TextContent {
                                text: complete_message.clone()
                            });
                            self.update_message(id, payload, Some(MessageStatus::Completed)).await?;
                            // When we have voice_mode active, we always need to send a last chunk with audio_end = true
                            if !message.is_empty()  || voice_mode {
                                yield Some(Response::Chat(ChatChunk::new(message, audio, true, id, step_id.to_owned())));
                            }
                        }
                        None => {
                            if !complete_message.is_empty() {
                                    // We have some chars but didn't create a message yet.
                                    //  So create a message and send it.
                                      let new_id = self
                                        .add_message_to_memory(
                                            step_id.to_owned(),
                                            TypeSafePayload::Text(TextContent {
                                                text: complete_message.clone()
                                            }),
                                            Direction::Send,
                                            MessageStatus::Completed,
                                        )
                                        .await?
                                        .message_order;
                                    yield Some(Response::Chat(ChatChunk::new(complete_message.clone(), complete_audio.clone(), true, new_id,  step_id.to_owned())));
                                }
                        }
                    }
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

    // fn handle_response<'a>(
    //     &'a mut self,
    //     response: LlmStepContent,
    //     voice_mode: bool,
    //     step_id: &'a str,
    // ) -> Pin<Box<dyn Stream<Item = Result<Option<Response>, LlmExecutionError>> + 'a + Send>> {
    //     Box::pin(try_stream! {
    //             match response {
    //             LlmStepContent::Skipped => {
    //                 tracing::debug!("Skipped step");
    //                 let mut current_action = self.current_action.as_ref().ok_or(LlmExecutionError::NoAction)?.lock().await;
    //                 let state = current_action.set_status(LlmStepStatus::Completed);
    //                 self.set_step_state(state).await?;
    //                 yield None;
    //             }
    //             LlmStepContent::Combined(combined_steps) => {
    //                 for step in combined_steps {
    //                     let mut response = self.handle_response(step, voice_mode, step_id);
    //                     while let Some(item) = response.next().await {
    //                         yield item?;
    //                     }
    //                 }
    //             }
    //             LlmStepContent::Message{mut message, store} => {
    //                 let mut complete_message = String::new(); // Initialize a complete message buffer
    //                 let mut offset: usize = 0;
    //                 // Init a new message in the database
    //                 let mut id = None;
    //                 // Create streaming content with the current message chunk
    //                 while let Some(result) = message.next().await {
    //                     match result {
    //                         Ok(value) => {
    //                             tracing::trace!(?value, "message chunk");
    //                             let content = if let Content::Text(content) = &value.content {
    //                                     Ok(content)
    //                             } else {
    //                                 Err(LlmExecutionError::UnexpectedResponseFormat)
    //                             }?;
    //                             complete_message.push_str(content.as_str());
    //                             let usage = value.tokens.unwrap_or(0);
    //                             tracing::trace!(?usage, "tokens used");
    //                             add_usage(&self.conn, &self.user_id, usage, step_id.to_owned()).await?;

    //                             if complete_message.len() > offset.saturating_add(BUFFER_SIZE) {
    //                                 let payload = TypeSafePayload::Text(TextContent {
    //                                     text: complete_message.clone(),
    //                                 });
    //                                 let id = match id {
    //                                     None => {
    //                                         // Only create a message, when we have something to send
    //                                         let new_id = self
    //                                             .add_message_to_memory(
    //                                                 step_id.to_owned(),
    //                                                 payload,
    //                                                 Direction::Send,
    //                                                 MessageStatus::Generating,
    //                                             )
    //                                             .await?
    //                                             .message_order;
    //                                         id = Some(new_id);
    //                                         new_id
    //                                     }
    //                                     Some(id) => {
    //                                         self.update_message(id, payload, Some(MessageStatus::Generating))
    //                                             .await?;
    //                                         id
    //                                     }
    //                                 };
    //                                 yield Some(Response::Chat(ChatChunk::new(
    //                                     complete_message[offset..].to_string(),
    //                                     id,
    //                                     step_id.to_owned(),
    //                                 )));
    //                                 offset = complete_message.len();
    //                             }
    //                         }
    //                         Err(error) => {
    //                             tracing::error!(error = &*error as &dyn Error, "error sending streaming message");
    //                             self.set_error(step_id).await?;
    //                             Err(error)?;
    //                             return;
    //                         }
    //                     }
    //                 }
    //                 match id {
    //                     Some(id) => {
    //                         let chunk = complete_message[offset..].to_string();
    //                         let payload = TypeSafePayload::Text(TextContent { text: complete_message.clone()});
    //                         self.update_message(id, payload, Some(MessageStatus::Completed)).await?;
    //                         if !chunk.is_empty() {
    //                             yield Some(Response::Chat(ChatChunk::new(chunk, id, step_id.to_owned())));
    //                         }
    //                     }
    //                     None => {
    //                         if !complete_message.is_empty() {
    //                             // We have some chars but didn't create a message yet.
    //                             //  So create a message and send it.
    //                             let id = self
    //                                 .add_message_to_memory(
    //                                     step_id.to_owned(),
    //                                     TypeSafePayload::Text(TextContent {
    //                                         text: complete_message.clone(),
    //                                     }),
    //                                     Direction::Send,
    //                                     MessageStatus::Completed,
    //                                 )
    //                                 .await?
    //                                 .message_order;
    //                             yield Some(Response::Chat(ChatChunk::new(complete_message.clone(), id, step_id.to_owned())));
    //                         }
    //                     }
    //                 }
    //                 if let Some(SaveTarget::Slot(slot_path)) = store {
    //                     let destination = slot_path.destination().clone();
    //                     let slot = Slot {
    //                         name: slot_path.name,
    //                         value: Value::String(complete_message),
    //                     };
    //                     self.set_slot(slot, destination).await?;
    //                 }
    //                 self.set_finished(step_id).await?;
    //             }
    //             LlmStepContent::StepValue{values, next_step} => {
    //                 for (target, value) in values {
    //                     match target {
    //                         SaveTarget::Slot(slot_path) => {
    //                             let destination = slot_path.destination().clone();
    //                             let slot = Slot {
    //                                 name: slot_path.name,
    //                                 value,
    //                             };
    //                             self.set_slot(slot, destination).await?;
    //                         }
    //                     }
    //                 }
    //                 self.set_finished(step_id).await?;

    //                 if let Some(next_step) = next_step {
    //                     self.goto(&next_step).await?;
    //                 }

    //                 yield None;
    //             }
    //         }
    //     })
    // }

    // Memory
    async fn add_message_to_memory(
        &self,
        step: String,
        payload: TypeSafePayload,
        direction: Direction,
        status: MessageStatus,
    ) -> Result<ConversationMessage, LlmExecutionError> {
        let (content_type, message) =
            split_payload_for_database(payload).map_err(|e| LlmExecutionError::Unexpected(e.to_string()))?;

        let direction = direction.into_db_model();
        let res = hikari_db::llm::message::Mutation::insert_new_message(
            &self.conn,
            self.conversation_id,
            step,
            content_type,
            message,
            direction,
            status.into_db_model(),
        )
        .await?;
        Ok(res.into_model())
    }

    async fn update_message(
        &self,
        message_order: i32,
        payload: TypeSafePayload,
        status: Option<MessageStatus>,
    ) -> Result<ConversationMessage, LlmExecutionError> {
        let (_, message) =
            split_payload_for_database(payload).map_err(|e| LlmExecutionError::Unexpected(e.to_string()))?;
        let res = hikari_db::llm::message::Mutation::update_message(
            &self.conn,
            self.conversation_id,
            message_order,
            message,
            status.map(hikari_model_tools::convert::IntoDbModel::into_db_model),
        )
        .await?;
        Ok(res.into_model())
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

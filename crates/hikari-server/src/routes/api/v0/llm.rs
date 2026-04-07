use super::modules::messaging::ChatRequest;
use crate::AppConfig;
use crate::data::modules::session::get_session;
use crate::db::sea_orm::util::{
    finish_llm_conversation, get_last_or_create_llm_conversation, start_new_llm_conversation,
};
use crate::permissions::Permission;
use crate::routes::api::v0::llm::error::LlmError;
use crate::routes::api::v0::llm::load_slots::load_slots;
use crate::routes::api::v0::modules::error::ModuleError;
use crate::user::ExtractUser;
use axum::Extension;
use axum::extract::ws::{Message as WsMessage, Message, WebSocket};
use axum::extract::{Path, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::{Router, get};
use bytes::Bytes;
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use hikari_config::module::llm_agent::LlmService;
use hikari_core::llm_config::LlmConfig;
use hikari_llm::builder::LlmStructureBuilder;
use hikari_llm::execution::agent::LlmAgent;
use hikari_llm::execution::agent::response::Response;
use hikari_llm::execution::iterator::LlmStepIterator;
use hikari_model::chat::TypeSafePayload;
use hikari_model::llm::conversation::LlmConversation;
use hikari_model::llm::state::LlmConversationState;
use hikari_model::module::locked_until;
use hikari_model::module::session::instance::SessionInstance;
use hikari_model::user::User;
use hikari_model_tools::convert::IntoModel;
use protect_axum::protect;
use sea_orm::DatabaseConnection;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::pin::pin;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::interval;
use utoipa::ToSchema;

pub(crate) mod error;

mod load_slots;

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/chat/{module_id}/{session_id}/ws", get(handler))
        .with_state(())
}

type Documents = Vec<String>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResponseAction {
    SetFinished,
    Restart,
    Abort,
    Nothing,
}

#[derive(ToSchema, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub(crate) struct ConnectionInfo {
    pub history_needed: bool,
    pub current_sequence: u16, //Fixme: Don't know if u16 makes sense
}

#[derive(ToSchema, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub(crate) enum Request {
    Chat(ChatRequest<TypeSafePayload>),
    ConnectionInfo(ConnectionInfo),
    Abort,
    Restart,
    ControllMessage,
}

async fn start_conversation(
    user: &User,
    config: &AppConfig,
    module_id: &str,
    session_id: &str,
    entries: &[SessionInstance],
    conn: &DatabaseConnection,
    force_start: bool,
) -> Result<
    (
        LlmConversation,
        LlmStructureBuilder,
        Option<LlmConversationState>,
        Documents,
        Documents,
        LlmService,
    ),
    LlmError,
> {
    let module_config = config.module_config();
    let llm_structures = &config.llm_data().structures;
    let (module, session) =
        get_session(module_id, session_id, module_config, &user.groups).map_err(ModuleError::DataError)?;

    let llm_agent = session.llm_agent.as_ref().ok_or(LlmError::AgentUnspecified)?;

    let llm_service = llm_agent.provider.clone();
    let structure_id: &str = llm_agent.llm_agent.as_ref();

    let mut tailored_session = session.clone();

    // Since we extract content from the session we want to filter out locked content; Thats is also important for load_slots
    tailored_session.contents.retain(|content| {
        content
            .unlock
            .as_ref()
            .is_none_or(|unlock| locked_until(unlock, entries).is_none())
    });

    let primary_documents = tailored_session
        .contents
        .iter()
        .flat_map(|c| c.sources.primary())
        .map(|s| s.file_id.clone())
        .collect::<Documents>();

    let secondary_documents = tailored_session
        .contents
        .iter()
        .flat_map(|c| c.sources.secondary())
        .map(|s| s.file_id.clone())
        .collect::<Documents>();

    let llm_structure: &LlmStructureBuilder = llm_structures
        .structures
        .get(structure_id)
        .ok_or(LlmError::AgentNotFound(structure_id.to_owned()))?;

    let (llm_conversation, llm_state) = if force_start {
        tracing::trace!(user_id = ?user.id, ?module_id, ?session_id, "Force starting new conversation");
        (
            start_new_llm_conversation(conn, user.id, module_id, session_id).await?,
            None,
        )
    } else {
        get_last_or_create_llm_conversation(conn, user.id, module_id, session_id).await?
    };

    // Alawys load slots to have the latest data
    load_slots(
        conn,
        llm_conversation.conversation_id,
        &llm_structure.slots,
        module_config,
        user,
        module,
        &tailored_session,
    )
    .await?;

    Ok((
        llm_conversation,
        llm_structure.clone(),
        llm_state,
        primary_documents,
        secondary_documents,
        llm_service,
    ))
}

#[allow(clippy::too_many_arguments)]
async fn create_agent(
    user: &User,
    config: &AppConfig,
    module_id: &str,
    session_id: &str,
    conn: &DatabaseConnection,
    llm_config: &LlmConfig,
    constants: &HashMap<String, serde_yml::Value>,
    force: bool,
) -> Result<LlmAgent, LlmError> {
    let session_instances: Vec<_> = hikari_db::module::session::status::Query::for_module(conn, user.id, module_id)
        .await?
        .into_iter()
        .map(IntoModel::into_model)
        .collect();

    let (llm_conversation, llm_structure, llm_state, primary_documents, secondary_documents, llm_service) =
        start_conversation(user, config, module_id, session_id, &session_instances, conn, force).await?;

    let mut llm_structure = llm_structure;
    llm_structure.with_constants(constants, false);
    llm_structure.with_documents(
        hikari_llm::builder::steps::Documents::new(primary_documents, secondary_documents),
        false,
    );
    let current_step = llm_state.as_ref().map(|state| state.current_step.clone());
    let iterator = LlmStepIterator::new(llm_structure, current_step)?;
    let llm_agent = LlmAgent::new(
        iterator,
        llm_state,
        llm_conversation.conversation_id,
        user.id,
        session_id.to_owned(),
        module_id.to_owned(),
        llm_config.clone(),
        llm_service,
        conn.clone(),
    )
    .await?;

    Ok(llm_agent)
}

#[utoipa::path(
    get,
    path = "/api/v0/llm/chat/{module_id}/self-learning/ws",
    tag = "v0/llm",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
async fn self_learning_handler(
    ws: WebSocketUpgrade,
    ExtractUser(user): ExtractUser,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
    Path(module_id): Path<String>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, user, app_config, module_id, "self-learning".to_string(), conn))
}

#[utoipa::path(
    get,
    path = "/api/v0/llm/chat/{module_id}/{session_id}/ws",
    tag = "v0/llm",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
async fn handler(
    ws: WebSocketUpgrade,
    ExtractUser(user): ExtractUser,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
    Path((module_id, session_id)): Path<(String, String)>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, user, app_config, module_id, session_id, conn))
}

pub async fn handle_socket(
    socket: WebSocket,
    user: User,
    config: AppConfig,
    module_id: String,
    session_id: String,
    conn: DatabaseConnection,
) {
    tracing::debug!("handling websocket connection");
    // By splitting socket we can send and receive at the same time.
    let (mut sender, mut receiver) = socket.split();
    let llm_config = config.llm_config().clone();
    let constants = &config.llm_data().constants.constants;
    let llm_agent = create_agent(
        &user,
        &config,
        &module_id,
        &session_id,
        &conn,
        &llm_config,
        constants,
        false,
    )
    .await;

    let llm_agent = match llm_agent {
        Ok(agent) => Ok(agent),
        Err(error) => {
            // This happens when we edit the structure and the saved step does not exist anymore
            tracing::warn!(?error, "Starting a new chat due to error");
            create_agent(
                &user,
                &config,
                &module_id,
                &session_id,
                &conn,
                &llm_config,
                constants,
                true,
            )
            .await
        }
    };

    let mut llm_agent = match llm_agent {
        Ok(agent) => Some(Mutex::new(agent)),
        Err(e) => {
            send_error(&mut sender, e).await;
            None
        }
    };

    let mut ping_interval = interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            message = receiver.next() => {
                match message {
                    Some(msg) => {
                        let reqeust_message = convert_message(msg);
                        let res = handle_message(
                            &user,
                            &config,
                            &module_id,
                            &session_id,
                            &conn,
                            &mut sender,
                            &llm_config,
                            constants,
                            &mut llm_agent,
                            reqeust_message,
                        )
                        .await;
                        if let Err(e) = res {
                            send_error(&mut sender, e).await;
                        }
                    }
                    None => break,
                }
            }
            _ = ping_interval.tick() => {
                if let Err(e) = sender.send(WsMessage::Ping(Bytes::new())).await {
                    tracing::debug!(error = &e as &dyn Error, "error sending ping");
                    break;
                }
            }
        }
    }
    tracing::debug!("closing websocket connection");
}

fn convert_message(message: Result<WsMessage, axum::Error>) -> Result<Request, LlmError> {
    let message = match message {
        Ok(message) => message,
        Err(error) => {
            tracing::warn!(warning = &error as &dyn Error, "received error message");
            return Err(LlmError::ReceiveError(error));
        }
    };
    let message = match message {
        WsMessage::Text(message) => message,
        WsMessage::Binary(_) => {
            // We just close the connection if we receive a binary message because we don't
            // expect any binary messages
            tracing::error!("received unexpected binary message");
            return Err(LlmError::RequestError("unexpected binary message".to_string()));
        }
        WsMessage::Close(close) => {
            tracing::info!(?close, "received close message");
            // The library handles control messages
            return Ok(Request::ControllMessage);
        }
        WsMessage::Ping(_) | WsMessage::Pong(_) => {
            tracing::debug!("received control message");
            return Ok(Request::ControllMessage);
        }
    };
    let message = serde_json::from_str(&message)?;
    Ok(message)
}

#[allow(clippy::too_many_arguments)]
async fn handle_message(
    user: &User,
    config: &AppConfig,
    module_id: &str,
    session_id: &str,
    conn: &DatabaseConnection,
    sender: &mut SplitSink<WebSocket, Message>,
    llm_config: &LlmConfig,
    constants: &HashMap<String, serde_yml::Value>,
    llm_agent: &mut Option<Mutex<LlmAgent>>,
    message: Result<Request, LlmError>,
) -> Result<(), LlmError> {
    let message = message?;
    let action = handle_request(llm_agent.as_ref(), sender, message).await?;
    match action {
        ResponseAction::Abort => {
            hikari_db::llm::conversation::Mutation::close_open_conversations(conn, user.id, module_id, session_id)
                .await
                .map_err(Into::into)
        }
        ResponseAction::Restart => {
            let new_agent =
                create_agent(user, config, module_id, session_id, conn, llm_config, constants, true).await?;
            *llm_agent = Some(Mutex::new(new_agent));
            send_connection_info(llm_agent.as_ref(), sender, false).await?;
            Ok(())
        }
        ResponseAction::SetFinished => {
            let module = config
                .module_config()
                .get(module_id)
                .ok_or(LlmError::ModuleNotFound(module_id.to_owned()))?;
            finish_llm_conversation(conn, user.id, module, session_id)
                .await
                .map_err(Into::into)
        }
        ResponseAction::Nothing => Ok(()),
    }
}

async fn handle_request(
    llm_agent: Option<&Mutex<LlmAgent>>,
    sender: &mut SplitSink<WebSocket, WsMessage>,
    request: Request,
) -> Result<ResponseAction, LlmError> {
    tracing::debug!(request = ?request, "received request");
    // TODO we currently block the websocket connection until the message is processed
    // TODO consider moving the processing to a separate task
    match request {
        Request::Chat(chat_message) => {
            tracing::trace!("chat message received");
            let message = chat_message.payload;
            generate_agent_response(llm_agent, sender, Some(message.clone()), false).await
        }
        Request::ConnectionInfo(connection_info) => {
            tracing::trace!(
                current_sequence = connection_info.current_sequence,
                "connection info received"
            );
            send_connection_info(llm_agent, sender, connection_info.history_needed).await
        }
        Request::Abort => {
            tracing::trace!("abort message received");
            Ok(ResponseAction::Abort)
        }
        Request::Restart => {
            tracing::trace!("restart message received");
            Ok(ResponseAction::Restart)
        }
        Request::ControllMessage => Ok(ResponseAction::Nothing),
    }
}

async fn send_connection_info(
    llm_agent: Option<&Mutex<LlmAgent>>,
    sender: &mut SplitSink<WebSocket, Message>,
    history_needed: bool,
) -> Result<ResponseAction, LlmError> {
    generate_agent_response(llm_agent, sender, None, history_needed).await
}

async fn generate_agent_response(
    llm_agent: Option<&Mutex<LlmAgent>>,
    sender: &mut SplitSink<WebSocket, WsMessage>,
    message: Option<TypeSafePayload>,
    history_needed: bool,
) -> Result<ResponseAction, LlmError> {
    tracing::debug!(?message, "sending message to agent");
    let llm_agent = llm_agent.ok_or(LlmError::NoAgent)?;
    let mut agent_guard = llm_agent.lock().await;
    let stream = agent_guard.chat(message, history_needed);
    let mut stream = pin!(stream);
    while let Some(response) = stream.next().await {
        match response {
            Ok(response) => {
                send_response(sender, &response).await?;
                if matches!(response, Response::ConversationEnd) {
                    return Ok(ResponseAction::SetFinished);
                }
            }
            Err(err) => {
                return Err(err.into());
            }
        }
    }
    Ok(ResponseAction::Nothing)
}

async fn send_error(sender: &mut SplitSink<WebSocket, WsMessage>, error: LlmError) {
    // We do the typecasting here for sentry. See https://crates.io/crates/sentry-tracing/
    tracing::warn!(error = &error as &dyn Error, "sending error response");
    let res = send_response(sender, &Response::Error(error.as_response())).await;
    if let Err(e) = res {
        tracing::error!(
            error = &e as &dyn Error,
            "error sending error response -> closing connection"
        );
        let close_frame = error.into_close_frame();
        if let Err(e) = sender.send(WsMessage::Close(Some(close_frame))).await {
            tracing::error!(error = &e as &dyn Error, "error sending close frame");
        }
    }
}

async fn send_response(sink: &mut SplitSink<WebSocket, WsMessage>, response: &Response) -> Result<(), LlmError> {
    let message_data = serde_json::to_string(response)?;
    let message = WsMessage::Text(message_data.into());
    sink.send(message).await.map_err(LlmError::SendError)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use hikari_llm::execution::agent::response::ChatChunk;

    use super::*;

    #[test]
    fn test_response_serialization() {
        let id = 1;
        let chat_chunk = Response::Chat(ChatChunk::new("...".to_owned(), id, "test".to_owned()));
        let serialized = serde_json::to_value(chat_chunk).unwrap();
        let expected = json!(
            {
                "type": "chat",
                "value": {
                    "content": "...",
                    "id": id,
                    "step": "test"
                }
            }
        );
        assert_eq!(serialized, expected);
    }

    #[test]
    fn text_hold() {
        let hold = Response::Hold;
        let serialized = serde_json::to_value(hold).unwrap();

        let expected = json!(
            {
                "type": "hold"
            }
        );
        assert_eq!(serialized, expected);
    }
}

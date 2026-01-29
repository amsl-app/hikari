use crate::data::bots::generate_channel_name;
use crate::data::modules::session::get_session;
use crate::db::sea_orm::util::{get_conversation_history, start_conversation};
use crate::permissions::Permission;
use crate::routes::api::v0::bots::configs_into_map;
use crate::routes::api::v0::modules::error::MessagingError;
use crate::user::{ExtractUser, ExtractUserId};
use crate::{AppConfig, db};
use axum::Json;
use axum::extract::{Path, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Extension, Router};
use csml_model::FlowTrigger;
use hikari_db::config;
use hikari_db::module::session::status;
use hikari_db::sea_orm::DatabaseConnection;
use hikari_db::util::FlattenTransactionResultExt;
use hikari_entity::module::session::status::Status;
use hikari_model::chat::{Client, Direction, Message, MessageResponse, Payload, RequestMetadata, TypeSafePayload};
use hikari_model::module::session::instance::SessionInstanceStatus;
use hikari_model::user::User;
use hikari_model_tools::convert::{FromDbModel, IntoDbModel};
use http::StatusCode;
use protect_axum::protect;
use sea_orm::TransactionTrait;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Debug;
use utoipa::ToSchema;
use uuid::Uuid;

//TODO use app name

impl From<HistoryMessage> for Message<Payload> {
    fn from(value: HistoryMessage) -> Self {
        Self {
            direction: value.direction,
            payload: value.payload,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub(crate) struct StartRequest {
    #[serde(default)]
    metadata: RequestMetadata,
    #[serde(default)]
    exclusive: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub(crate) struct HistoryMessage {
    client: Client,
    payload: Payload,
    direction: Direction,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub(crate) struct ChatRequest<T> {
    pub(crate) payload: T,
    #[serde(default)]
    pub(crate) metadata: RequestMetadata,
}

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/chat", post(chat_session))
        .route("/chat_v2", post(chat_session_v2))
        .route("/chat/ws", post(chat_session_ws))
        .route("/start", post(start_session))
        .route("/reset", post(reset_session))
        .route("/abort", post(abort_session))
        .route("/status", get(get_session_status).put(set_session_status))
        .with_state(())
}

#[utoipa::path(
    post,
    path = "/api/v0/modules/{module}/sessions/{session}/start",
    request_body = Option<StartRequest>,
    responses(
        (status = OK, body = MessageResponse<Payload>, description = "Starts the session and returns the first message or returns the chat history if the session is already started"),
        (status = CONFLICT, description = "Another session is already started and this session requested to be started exclusively"),
    ),
    params(
        ("module" = String, Path, description = "the module id of the module from the session should be started"),
        ("session" = String, Path, description = "the session id of the session to be started"),
    ),
    tag = "v0/modules",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]

pub(crate) async fn start_session(
    ExtractUser(user): ExtractUser,
    Path((module_id, session_id)): Path<(String, String)>,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
    Json(request): Json<Option<StartRequest>>,
) -> Result<impl IntoResponse, MessagingError> {
    let configs = config::Query::get_user_config(&conn, user.id).await?;
    let configs = configs_into_map(configs)?;
    check_exclusivity(request.as_ref(), user.id, &conn, &module_id, &session_id).await?;

    let res = conn
        .transaction(|txn| {
            Box::pin(async move {
                let module_config = app_config.module_config();
                let bots = app_config.bots();
                let (module, session) = get_session(&module_id, &session_id, module_config, &user.groups)?;

                hikari_db::module::status::Mutation::create(
                    txn,
                    user.id,
                    module_id.clone(),
                ).await?;
                let mut instance = status::Mutation::create(
                    txn,
                    user.id,
                    module_id.clone(),
                    session_id.clone(),
                    session.get_bot().map(String::from),
                )
                    .await?;

                if instance.status != Status::NotStarted {
                    tracing::debug!(%user.id, %module_id, %session_id, "returning conversation history because the session was already started");
                    match instance.last_conv_id {
                        None => {
                            tracing::error!(%user.id, %module_id, %session_id, "no conversation id for started conversation");
                        }
                        Some(conversation_id) => {
                            return get_conversation_history(txn, conversation_id).await
                        }
                    }
                }
                hikari_db::module::status::Mutation::set_status_for_user(
                    txn,
                    user.clone().id,
                    &module_id,
                    hikari_entity::module::status::Status::Started,
                )
                    .await?;
                instance = status::Mutation::set_status_for_user(
                    txn,
                    user.id,
                    &module_id,
                    &session_id,
                    Status::Started,
                )
                    .await?;

                let (bot_id, flow_id) = session
                    .get_bot_and_flow().ok_or(MessagingError::NoBot(session.get_id().to_string()))?;

                let csml_bot = bots.find(bot_id).ok_or(MessagingError::BotNotFound {
                    bot_id: bot_id.to_string(),
                })?;

                let flow_id = flow_id.unwrap_or(&csml_bot.default_flow);

                let payload = TypeSafePayload::FlowTrigger(FlowTrigger { flow_id: flow_id.to_string(), step_id: None }).try_into()?;
                start_conversation(txn, user, payload, request.map_or(RequestMetadata::default(), |r| r.metadata), csml_bot.clone(), module, session, instance, configs, module_config).await
            })
        })
        .await;

    let res = res.flatten_res()?;

    Ok(Json(res))
}

async fn check_exclusivity(
    request: Option<&StartRequest>,
    user_id: Uuid,
    sea_orm_db: &DatabaseConnection,
    module_id: &str,
    session_id: &str,
) -> Result<(), MessagingError> {
    if request.as_ref().is_some_and(|r| r.exclusive) {
        let instances = status::Query::find_other_running_sessions(sea_orm_db, user_id, module_id, session_id).await?;
        if !instances.is_empty() {
            return Err(MessagingError::Exclusivity);
        }
    }
    Ok(())
}

/// Restarts the session and returns the first message
#[utoipa::path(
    post,
    path = "/api/v0/modules/{module}/sessions/{session}/reset",
    request_body = Option<StartRequest>,
    responses(
        (status = OK, body = MessageResponse<Payload>, description = "First messages of the new session."),
        (status = NO_CONTENT, description = "Session has no bot so no messages are returned."),
        (status = CONFLICT, description = "Another session is already started and this session requested to be started exclusively"),
    ),
    params(
        ("module" = String, Path, description = "the module id of the module from the session should be restarted"),
        ("session" = String, Path, description = "the session id of the session to be restarted"),
    ),
    tag = "v0/modules",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]

pub(crate) async fn reset_session(
    ExtractUser(user): ExtractUser,
    Path((module_id, session_id)): Path<(String, String)>,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
    Json(request): Json<Option<StartRequest>>,
) -> Result<impl IntoResponse, MessagingError> {
    let configs = config::Query::get_user_config(&conn, user.id).await?;
    let configs = configs_into_map(configs)?;

    check_exclusivity(request.as_ref(), user.id, &conn, &module_id, &session_id).await?;

    let res = conn
        .transaction(|txn| {
            Box::pin(async move {
                let module_config = app_config.module_config();
                let bots = app_config.bots();
                let (module, session) = get_session(&module_id, &session_id, module_config, &user.groups)?;

                let instance = status::Mutation::create(
                    txn,
                    user.id,
                    module_id.clone(),
                    session_id.clone(),
                    session.get_bot().map(String::from),
                )
                .await?;

                if instance.status == Status::Started {
                    //TODO return history
                    return Err(MessagingError::AlreadyStarted);
                }

                let instance =
                    status::Mutation::set_status_for_user(txn, user.id, &module_id, &session_id, Status::Started)
                        .await?;
                let Some((bot_id, flow_id)) = session.get_bot_and_flow() else {
                    return Ok(None);
                };

                let csml_bot = bots.find(bot_id).ok_or(MessagingError::BotNotFound {
                    bot_id: bot_id.to_string(),
                })?;

                let flow_id = flow_id.unwrap_or(&csml_bot.default_flow);

                let payload = TypeSafePayload::FlowTrigger(FlowTrigger {
                    flow_id: flow_id.to_string(),
                    step_id: None,
                })
                .try_into()?;
                start_conversation(
                    txn,
                    user,
                    payload,
                    request.map_or_else(Default::default, |r| r.metadata),
                    csml_bot.clone(),
                    module,
                    session,
                    instance,
                    configs,
                    module_config,
                )
                .await
                .map(Some)
            })
        })
        .await;

    let res = res.flatten_res()?;

    Ok(if res.is_some() {
        Json(res).into_response()
    } else {
        StatusCode::NO_CONTENT.into_response()
    })
}

/// Aborts a session and sets it to `not_started`
#[utoipa::path(
    post,
    path = "/api/v0/modules/{module}/sessions/{session}/abort",
    responses(
        (status = NO_CONTENT),
        (status = NOT_FOUND, description = "Session was never started")
    ),
    params(
        ("module" = String, Path, description = "the module id of the module from the session should be aborted"),
        ("session" = String, Path, description = "the session id of the session to be aborted"),
    ),
    tag = "v0/modules",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]

pub(crate) async fn abort_session(
    ExtractUserId(user): ExtractUserId,
    Path((module_id, session_id)): Path<(String, String)>,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, MessagingError> {
    let model = status::Query::get_for_user(&conn, user, &module_id, &session_id).await?;
    let model = model.ok_or(MessagingError::NotRunning)?;
    match model.status {
        Status::NotStarted | Status::Finished => {
            tracing::debug!(%user, moduled = %module_id, session = %session_id, "session not running - not aborting");
            return Ok(StatusCode::NO_CONTENT);
        }
        Status::Started => {}
    }
    db::sea_orm::module::session::user_session_status::abort_session_instance(&conn, user, model).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/v0/modules/{module}/sessions/{session}/chat",
    responses(
        (status = OK, body = MessageResponse<Payload>, description = "Chats with the corresponding bot of the session and returns chat engine results"),
    ),
    request_body = Payload,
    params(
        ("module" = String, Path, description = "module id from which to use a session for the conversation"),
        ("session" = String, Path, description = "the session id of the session to use for the conversation"),
    ),
    tag = "v0/modules",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn chat_session(
    ExtractUser(user): ExtractUser,
    Path((module_id, session_id)): Path<(String, String)>,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
    Json(payload): Json<Payload>,
) -> Result<impl IntoResponse, MessagingError> {
    chat_session_inner(
        user,
        module_id,
        session_id,
        ChatRequest {
            payload,
            metadata: RequestMetadata::default(),
        },
        app_config,
        conn,
    )
    .await
}

#[utoipa::path(
    post,
    path = "/api/v0/modules/{module}/sessions/{session}/chat_v2",
    responses(
        (status = OK, body = MessageResponse<Payload>, description = "Chats with the corresponding bot of the session and returns chat engine results"),
    ),
    request_body = ChatRequest<Payload>,
    params(
        ("module" = String, Path, description = "module id from which to use a session for the conversation"),
        ("session" = String, Path, description = "the session id of the session to use for the conversation"),
    ),
    tag = "v0/modules",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn chat_session_v2(
    ExtractUser(user): ExtractUser,
    Path((module_id, session_id)): Path<(String, String)>,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
    Json(chat_request): Json<ChatRequest<Payload>>,
) -> Result<impl IntoResponse, MessagingError> {
    chat_session_inner(user, module_id, session_id, chat_request, app_config, conn).await
}

pub(crate) async fn chat_session_inner(
    user: User,
    module_id: String,
    session_id: String,
    chat_request: ChatRequest<Payload>,
    app_config: AppConfig,
    conn: DatabaseConnection,
) -> Result<impl IntoResponse, MessagingError> {
    let configs = config::Query::get_user_config(&conn, user.id).await?;
    let configs = configs_into_map(configs)?;

    let res = conn
        .transaction::<_, MessageResponse<Payload>, MessagingError>(|txn| {
            Box::pin(async move {
                // Make sure the module and session actually exist
                let (module, session) = get_session(&module_id, &session_id, app_config.module_config(), &user.groups)?;

                let bot = session.get_bot();

                let session_entry = status::Mutation::create(
                    txn,
                    user.id,
                    module_id.clone(),
                    session_id.clone(),
                    bot.map(String::from),
                )
                .await?;

                let bot_id = session_entry
                    .bot_id
                    .as_deref()
                    .ok_or_else(|| MessagingError::NoBot(format! {"{module_id}-{session_id}"}))?;

                let bot = app_config
                    .bots()
                    .find(bot_id)
                    .ok_or_else(|| MessagingError::BotNotFound {
                        bot_id: bot_id.to_string(),
                    })?;

                let ChatRequest { payload, metadata } = chat_request;
                start_conversation(
                    txn,
                    user,
                    payload,
                    metadata,
                    bot.clone(),
                    module,
                    session,
                    session_entry,
                    configs,
                    app_config.module_config(),
                )
                .await
            })
        })
        .await;

    res.flatten_res().map(Json)
}

#[utoipa::path(
    get,
    path = "/api/v0/modules/{module}/sessions/{session}/chat/ws",
    tag = "v0/modules",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
async fn chat_session_ws(
    ws: WebSocketUpgrade,
    ExtractUser(user): ExtractUser,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
    Path((module_id, session_id)): Path<(String, String)>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| {
        crate::routes::api::v0::llm::handle_socket(socket, user, app_config, module_id, session_id, conn)
    })
}

/// Get the session status for the current user
#[utoipa::path(
    get,
    path = "/api/v0/modules/{module}/sessions/{session}/status",
    responses(
        (status = OK, body = SessionInstanceStatus, description = "Status of the session"),
    ),
    params(
        ("module" = String, Path, description = "module id"),
        ("session" = String, Path, description = "session id"),
    ),
    tag = "v0/modules",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn get_session_status(
    ExtractUserId(user): ExtractUserId,
    Path((module_id, session_id)): Path<(String, String)>,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, MessagingError> {
    let instance = status::Query::get_for_user(&conn, user, module_id.as_str(), session_id.as_str()).await?;
    let status = instance
        .map(|model| model.status)
        .map_or(SessionInstanceStatus::NotStarted, FromDbModel::from_db_model);
    Ok(Json(status))
}

/// Set the session status for the current user
#[utoipa::path(
    put,
    path = "/api/v0/modules/{module}/sessions/{session}/status",
    request_body = SessionInstanceStatus,
    responses(
        (status = NO_CONTENT),
    ),
    params(
        ("module" = String, Path, description = "module id"),
        ("session" = String, Path, description = "session id"),
    ),
    tag = "v0/modules",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn set_session_status(
    ExtractUserId(user): ExtractUserId,
    Path((module_id, session_id)): Path<(String, String)>,
    Extension(conn): Extension<DatabaseConnection>,
    Json(status): Json<SessionInstanceStatus>,
) -> Result<impl IntoResponse, MessagingError> {
    status::Mutation::set_status_for_user(
        &conn,
        user,
        module_id.as_str(),
        session_id.as_str(),
        IntoDbModel::into_db_model(status),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) fn generate_client(user_id: Uuid, bot_id: String, module: &str, session: &str) -> Client {
    Client {
        channel_id: generate_channel_name(module, session),
        user_id: user_id.as_hyphenated().to_string(),
        bot_id,
    }
}

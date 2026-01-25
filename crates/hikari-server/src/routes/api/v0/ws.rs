mod error;

use crate::AppConfig;
use crate::data::modules::session::get_session;
use crate::permissions::Permission;
use crate::routes::api::v0::modules::messaging::generate_client;
use crate::user::ExtractUser;
use axum::extract::WebSocketUpgrade;
use axum::extract::ws::{Message, WebSocket};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Extension, Router};
use csml_engine::data::AsyncDatabase;
use csml_engine::data::models::BotOpt;
use csml_engine::future::start_conversation_stream;
use error::WsError;
use futures::pin_mut;
use futures_util::future::FusedFuture;
use futures_util::{FutureExt, select, select_biased};
use futures_util::{SinkExt, StreamExt};
use hikari_common::csml_utils::init_request;
use hikari_db::module::session::status;
use hikari_model::chat::{ChatRequest, MessageResponse, Payload};
use hikari_model::user::User;
use pin_project::pin_project;
use protect_axum::protect;
use sea_orm::DatabaseConnection;
use serde_derive::Deserialize;
use std::error::Error;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::sync::{Mutex, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/", get(setup_ws)).with_state(())
}

#[utoipa::path(
    post,
    path = "/api/v0/ws",
    responses(
        (status = OK, body = MessageResponse<Payload>, description = "Chats with the corresponding bot of the session and returns chat engine results"),
    ),
    request_body = ChatRequest<Payload>,
    params(
        ("module" = String, Path, description = "module id from which to use a session for the conversation"),
        ("session" = String, Path, description = "the session id of the session to use for the conversation"),
    ),
    tag = "v0/ws",
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
pub(crate) async fn setup_ws(
    ws: WebSocketUpgrade,
    ExtractUser(user): ExtractUser,
    // Path((module_id, session_id)): Path<(String, String)>,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
    // Json(chat_request): Json<ChatRequest<Payload>>,
) -> impl IntoResponse {
    tracing::debug!(user.id = %user.id, "setting up websocket");
    ws.on_upgrade(move |socket| handle_socket(socket, app_config, conn, Arc::new(user)).boxed())
}

#[pin_project]
struct ErrorSignal<E> {
    #[pin]
    signal: CancellationToken,
    error: Arc<Mutex<Option<E>>>,
}

impl<E> ErrorSignal<E> {
    fn new() -> Self {
        Self {
            signal: CancellationToken::new(),
            error: Arc::new(Mutex::new(None)),
        }
    }

    async fn set(&self, error: E) {
        let mut error_guard = self.error.lock().await;
        *error_guard = Some(error);
        self.notify();
    }

    fn notify(&self) {
        self.signal.cancel();
    }

    fn child_token(&self) -> CancellationToken {
        self.signal.child_token()
    }

    fn cancelled(&self) -> Pin<Box<impl FusedFuture<Output = ()> + Send + use<E>>> {
        let token = self.signal.clone();
        Box::pin(
            async move {
                token.cancelled().await;
            }
            .fuse(),
        )
    }

    async fn take(&self) -> Option<E> {
        let mut error_guard = self.error.lock().await;
        error_guard.take()
    }
}

impl<E> Clone for ErrorSignal<E> {
    fn clone(&self) -> Self {
        Self {
            signal: self.signal.clone(),
            error: Arc::clone(&self.error),
        }
    }
}

async fn handle_socket(socket: WebSocket, app_config: AppConfig, conn: DatabaseConnection, user: Arc<User>) {
    // By splitting socket we can send and receive at the same time.
    let (mut sender, receiver) = socket.split();
    let (send_channel_sender, send_channel_receiver) = mpsc::channel::<Message>(16);
    let error_signal: ErrorSignal<WsError> = ErrorSignal::new();

    let send_processor_error_signal = error_signal.clone();
    let mut send_processor_signal = error_signal.cancelled();
    let send_processor = tokio::task::spawn(async move {
        let mut receive_stream = ReceiverStream::new(send_channel_receiver).fuse();
        loop {
            select_biased! {
                () = &mut send_processor_signal => {
                    tracing::debug!("error signal received: closing send processor");
                    break
                },
                message = receive_stream.next() => {
                    let Some(message) = message else {
                        break
                    };
                    if let Err(error) = sender.send(message).await {
                        tracing::error!(error = %error, "error sending message");
                        send_processor_error_signal.notify();
                        break
                    }
                },
            }
        }
        sender
    });

    let mut receiver = receiver.fuse();
    let tracker = TaskTracker::new();
    let token = error_signal.child_token();
    let mut error_signal_future = error_signal.cancelled();
    loop {
        select_biased!(
            () = &mut error_signal_future => {
                tracing::debug!("error signal received: closing receiver");
                break
            },
            message = receiver.next() => {
                let Some(message) = message else {
                    break
                };
                let app_config = app_config.clone();
                let conn = conn.clone();
                let error_signal = error_signal.clone();
                let send_channel_sender = send_channel_sender.clone();
                let token = token.clone();
                let user = Arc::clone(&user);
                tracker.spawn(async move {
                    select! {
                        () = handle_message(&app_config, &conn, user, send_channel_sender, message, error_signal).fuse() => {},
                        () = token.cancelled().fuse() => {
                            tracing::debug!("task cancelled");
                        },
                    }
                });
            },
        );
    }
    token.cancel();
    tracker.close();
    tracker.wait().await;

    let sender = send_processor.await;
    match sender {
        Ok(mut sender) => {
            if let Some(error) = error_signal.take().await {
                tracing::debug!(error = &error as &dyn Error, "closing connection because of error");
                sender
                    .send(Message::Close(Some(error.into_close_frame())))
                    .await
                    .unwrap_or_else(|error| tracing::error!(error = &error as &dyn Error, "error closing connection"));
            }
        }
        Err(error) => {
            tracing::error!(error = %error, "error processing request");
            if let Some(error) = error_signal.take().await {
                // We can't send the close frame,
                //  because the sender is owned by the task that errored
                tracing::warn!(
                    error = &error as &dyn Error,
                    "closing connection because of error without sending close frame"
                );
            }
        }
    }
    tracing::debug!("websocket connection closed");
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WsMessage {
    ChatRequest {
        module_id: String,
        session_id: String,
        request: ChatRequest<Payload>,
    },
    Text(String),
    Sleep(String),
}

async fn handle_message(
    app_config: &AppConfig,
    conn: &DatabaseConnection,
    user: Arc<User>,
    sender: Sender<Message>,
    message: Result<Message, axum::Error>,
    error_signal: ErrorSignal<WsError>,
) {
    let result = match message {
        Ok(message) => handle_message_inner(app_config, conn, user, sender, message).await,
        Err(error) => {
            tracing::error!(error = &error as &dyn Error, "error receiving message");
            Err(error.into())
        }
    };
    if let Err(error) = result {
        if let WsError::Send(_) = &error {
            tracing::info!("aborting request response handling because the sender is closed");
            return;
        }
        tracing::error!(error = &error as &dyn Error, "error handling request response");
        error_signal.set(error).await;
    }
}

async fn handle_message_inner(
    app_config: &AppConfig,
    conn: &DatabaseConnection,
    user: Arc<User>,
    sender: Sender<Message>,
    message: Message,
) -> Result<(), WsError> {
    const SLEEP_DURATION: std::time::Duration = std::time::Duration::from_secs(5);
    tracing::debug!(message = ?message, "received message");
    // TODO we currently block the websocket connection until the message is processed
    // TODO consider moving the processing to a separate task

    tracing::debug!(message = ?message, "handling message");
    let message = match message {
        Message::Text(message) => message,
        Message::Binary(_) => {
            // We just close the connection if we receive a binary message because we don't
            // expect any binary messages
            return Err(WsError::RequestError("unexpected binary message".to_string()));
        }
        Message::Close(_) | Message::Ping(_) | Message::Pong(_) => {
            // The library handles control messages
            return Ok(());
        }
    };
    let message = serde_json::from_str(&message)?;

    match message {
        WsMessage::ChatRequest {
            module_id,
            session_id,
            request,
        } => {
            tracing::debug!(chat_request = ?request, "handling chat request");

            let (_, session) = get_session(&module_id, &session_id, app_config.module_config(), &user.groups)
                .expect("TODO error handling");

            let bot = session.get_bot();

            let session_entry = status::Mutation::create(
                conn,
                user.id,
                module_id.clone(),
                session_id.clone(),
                bot.map(String::from),
            )
            .await?;

            let bot_id = session_entry.bot_id.as_deref().expect("TODO handle bot_id not found");
            let bot = app_config.bots().find(bot_id).expect("TODO handle bot_id not found");

            let client = generate_client(user.id, bot.id.clone(), &session_entry.module, &session_entry.session);
            let mut csml_conn = AsyncDatabase::sea_orm(conn);
            let ChatRequest { payload, metadata } = request;
            let payload = serde_json::to_value(payload)?;
            let metadata = serde_json::to_value(metadata)?;
            let chat_request = init_request(client.clone(), payload, metadata);
            let bot_opt = BotOpt::CsmlBot(Box::new(bot.clone()));
            let (conversation, stream) = start_conversation_stream(chat_request, bot_opt, &mut csml_conn).await?;

            sender
                .send(Message::Text(format!("started conversation: {conversation:?}").into()))
                .await?;
            if let Some(mut stream_data) = stream {
                {
                    let stream = stream_data.stream().await?;
                    pin_mut!(stream);
                    while let Some(message) = stream.next().await {
                        let message = message?;
                        sender
                            .send(Message::Text(format!("csml message: {message:?}").into()))
                            .await?;
                    }
                }
                let conversation = stream_data.finalize().await?;
                sender
                    .send(Message::Text(format!("conversation ended: {conversation:?}").into()))
                    .await?;
            }
        }
        WsMessage::Text(text) => {
            tracing::debug!(text = ?text, "handling text message");
            // TODO Remove
            sender
                .send(Message::Text(format!("received text: {text:?}").into()))
                .await?;
        }
        WsMessage::Sleep(text) => {
            tracing::debug!(text = ?text, "handling sleep message");
            // TODO Remove
            sender
                .send(Message::Text(
                    format!("Sleeping for: {SLEEP_DURATION:?} ({text})").into(),
                ))
                .await?;
            tokio::time::sleep(SLEEP_DURATION).await;
            sender.send(Message::Text(format!("Woke up ({text})").into())).await?;
        }
    }

    Ok(())
}

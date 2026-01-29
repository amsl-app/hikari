use super::error::{BotError, MessageError};
use crate::AppConfig;
use crate::data::csml::flow_info_from_csml;
use crate::permissions::Permission;
use crate::user::ExtractUser;
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use csml_model::FlowTrigger;
use hikari_db::sea_orm::DatabaseConnection;
use hikari_model::chat::{ClientInfo, FlowInfo, RequestMetadata, TypeSafePayload};
use protect_axum::protect;

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(list_flows))
        .route("/{flow_id}/trigger", post(trigger_flow))
        .with_state(())
}

#[utoipa::path(
    get,
    path = "/api/v0/bots/{id}/flows",
    tag = "v0/bots",
    responses(
        (status = OK, body = [FlowInfo], description = "List flows (id and name) based on id the bot id"),
        (status = NOT_FOUND, description = "The given bot could not be found"),
    ),
    params(
        ("id" = String, Path, description = "bots id from which the flows should be listed", example = "exampleBotID")
    ),
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]
pub(crate) async fn list_flows(
    Path(bot_id): Path<String>,
    Extension(app_config): Extension<AppConfig>,
) -> Result<impl IntoResponse, BotError> {
    let bot = app_config.bots().find(&bot_id).ok_or_else(|| {
        tracing::warn!(bot_id, "csml bot does not exist");
        BotError::BotNotFound
    })?;

    let flows: Vec<FlowInfo> = bot.flows.iter().map(flow_info_from_csml).collect();

    // Have to call into_response() because of lifetime issues
    Ok(Json(flows).into_response())
}

#[allow(clippy::type_complexity)]
#[utoipa::path(
    post,
    path = "/api/v0/bots/{id}/flows/{flow_id}/trigger",
    tag = "v0/bots",
    request_body = ClientInfo,
    responses(
        (status = OK, description = "Start a new conversation with a bot"),
        (status = NOT_FOUND, description = "Bot could not be found"),
    ),
    params(
        ("id" = String, Path, description = "The bot-id the conversation should be started"),
        ("flow_id" = String, Path, description = "The flow-id the conversation should be started"),
    ),
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]
pub(crate) async fn trigger_flow(
    ExtractUser(user): ExtractUser,
    Path((bot_id, flow_id)): Path<(String, String)>,
    Extension(conn): Extension<DatabaseConnection>,
    Extension(app_config): Extension<AppConfig>,
    Json(client_info): Json<ClientInfo>,
) -> Result<impl IntoResponse, MessageError> {
    let payload = TypeSafePayload::FlowTrigger(FlowTrigger { flow_id, step_id: None }).try_into()?;

    let message_response = super::start_conversation(
        user,
        &bot_id,
        client_info.client,
        payload,
        RequestMetadata::default(),
        app_config.bots(),
        &conn,
    )
    .await?;

    Ok(Json(message_response))
}

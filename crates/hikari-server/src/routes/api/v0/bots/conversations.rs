use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Extension, Json, Router};
use protect_axum::protect;

use csml_engine::data::AsyncDatabase;
use csml_engine::future::db_connectors::conversations::get_client_conversations;
use hikari_model::chat::{Client, ConversationInfo};
use sea_orm::DatabaseConnection;

use super::error::ConversationError;
use super::{Conversations, CsmlClient};
use crate::permissions::Permission;
use crate::user::ExtractUserId;

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(get_conversations))
        .route("/open", get(get_open_conversations))
        .with_state(())
}

#[utoipa::path(
    get,
    path = "/api/v0/bots/{id}/conversations",
    tag = "v0/bots",
    responses(
        (status = OK, description = "Returns the conversations which the current user had with the given bot", body = [ConversationInfo]),
    ),
    params(
        ("id" = String, Path, description = "the bot id from which to load the conversation history", example = "botID"),
        ("client_id" = String, Query, description = "the client id from which to load the conversation", example = "exampleChannelClientID"),
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

pub(crate) async fn get_conversations(
    ExtractUserId(user_id): ExtractUserId,
    Path(id): Path<String>,
    Query(conversation): Query<Conversations>,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, ConversationError> {
    let client_id = conversation.client_id;

    let client = CsmlClient {
        bot_id: id,
        channel_id: client_id,
        user_id: user_id.as_hyphenated().to_string(),
    };

    let mut db: AsyncDatabase<DatabaseConnection> = AsyncDatabase::sea_orm(&conn);
    let res = get_client_conversations(&client, &mut db, None, None).await?;
    let response_format: Vec<ConversationInfo> = res
        .data
        .into_iter()
        .map(|conv| ConversationInfo {
            client: Client {
                bot_id: conv.client.bot_id,
                channel_id: conv.client.channel_id,
                user_id: conv.client.user_id,
            },
            flow_id: conv.flow_id,
            step_id: conv.step_id,
            last_interaction_at: conv.last_interaction_at.fixed_offset(),
            created_at: conv.created_at.fixed_offset(),
            updated_at: conv.updated_at.fixed_offset(),
        })
        .collect();
    Ok(Json(response_format))
}

#[utoipa::path(
    get,
    path = "/api/v0/bots/{id}/conversations/open",
    tag = "v0/bots",
    responses(
        (status = OK, description = "Gets open conversation with the bot on a certain channel", body = Option<ConversationInfo>),
    ),
    params(
        ("id" = String, Path, description = "bots id from which the flows should be listed", example = "exampleBotID"),
        ("client_id" = String, Query, description = "the client id of the requested conversation", example = "exampleClientId"),
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

pub(crate) async fn get_open_conversations(
    ExtractUserId(user_id): ExtractUserId,
    Path(id): Path<String>,
    Query(conversation): Query<Conversations>,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<Response, ConversationError> {
    let client_id = conversation.client_id;

    let client = CsmlClient {
        bot_id: id,
        channel_id: client_id,
        user_id: user_id.as_hyphenated().to_string(),
    };

    let mut db: AsyncDatabase<DatabaseConnection> = AsyncDatabase::sea_orm(&conn);

    let conv = csml_engine::future::db_connectors::conversations::get_latest_open(&client, &mut db).await?;

    Ok(Json(conv).into_response())
}

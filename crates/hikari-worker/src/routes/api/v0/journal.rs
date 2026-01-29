pub(crate) mod error;

use crate::routes::api::v0::journal::error::AssistantError;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Extension, Json, Router};
use chrono::{DateTime, FixedOffset};
use hikari_core::journal::summarize::{SummaryResponse, summarize};

use hikari_core::llm_config::LlmConfig;
use http::StatusCode;
use sea_orm::DatabaseConnection;
use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub(crate) struct SummaryOptions {
    pub user_id: Uuid,
    pub time: DateTime<FixedOffset>,
}

type RouterState = Arc<LlmConfig>;

pub fn create_router<S>(llm_config: Arc<LlmConfig>) -> Router<S> {
    Router::new()
        .route("/summarize", post(summarize_handler))
        .with_state(llm_config)
}

#[utoipa::path(
    post,
    path = "/api/v0/journal/summarize",
    request_body = Option<SummaryOptions>,
    responses(
        (status = OK, description = "The prompt from the assistant.", body = SummaryResponse),
        (status = NOT_FOUND, description = "No journal entries exist for the user."),
        (status = INTERNAL_SERVER_ERROR, description = "Something went wrong. Check response body."),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
pub(crate) async fn summarize_handler(
    State(state): State<RouterState>,
    Extension(conn): Extension<DatabaseConnection>,
    Json(data): Json<SummaryOptions>,
) -> Result<impl IntoResponse, AssistantError> {
    sentry::configure_scope(|scope| {
        scope.set_user(Some(sentry::User {
            id: Some(data.user_id.to_string()),
            ..Default::default()
        }));
    });
    tracing::debug!(user_id = %data.user_id, time = %data.time, "processing summarize request");
    let response = summarize(conn, data.user_id, state, Some(data.time)).await?;
    let status_code = if response.summary.is_some() {
        tracing::debug!(user_id = %data.user_id, time = %data.time, "returning summary");
        StatusCode::OK
    } else {
        tracing::debug!(user_id = %data.user_id, time = %data.time, "no journal entries found");
        StatusCode::NOT_FOUND
    };
    Ok((status_code, Json(response)))
}

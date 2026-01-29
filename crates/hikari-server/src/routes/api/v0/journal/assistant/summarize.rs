use crate::AppConfig;
use crate::permissions::Permission;
use crate::routes::api::v0::journal::assistant::error::{AssistantError, AssistantErrorType};
use crate::routes::error::ErrorData;
use crate::user::ExtractUserId;
use axum::Extension;
use axum::Json;
use axum::response::IntoResponse;
use chrono::{DateTime, FixedOffset, Utc};
use hikari_core::journal::summarize::SummaryResponse;
use http::StatusCode;
use protect_axum::protect;
use reqwest_tracing::{DefaultSpanBackend, TracingMiddleware};
use serde_derive::{Deserialize, Serialize};
use std::error::Error;
use std::time::Duration;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub(crate) struct SummaryOptions {
    pub time: Option<DateTime<FixedOffset>>,
}

#[derive(Serialize)]
pub(crate) struct WorkerSummaryOptions {
    pub user_id: Uuid,
    pub time: DateTime<FixedOffset>,
}

fn build_client() -> Result<reqwest_middleware::ClientWithMiddleware, AssistantError> {
    let builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(125));

    let client = builder.build()?;

    let middleware_client = reqwest_middleware::ClientBuilder::new(client)
        .with(TracingMiddleware::<DefaultSpanBackend>::new())
        .build();
    Ok(middleware_client)
}

/// Summarize journal entries
#[utoipa::path(
    post,
    path = "/api/v0/journal/assistant/summarize",
    request_body = Option<SummaryOptions>,
    responses(
        (status = OK, description = "The prompt from the assistant.", body = SummaryResponse),
        (status = NOT_FOUND, description = "No journal entries exist for the user."),
        (status = INTERNAL_SERVER_ERROR, description = "Something went wrong. Check response body.", body = ErrorData<AssistantErrorType>),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Journal", ty = "Permission")]
pub(crate) async fn summarize_handler(
    ExtractUserId(user_id): ExtractUserId,
    Extension(app_config): Extension<AppConfig>,
    Json(options): Json<SummaryOptions>,
) -> Result<impl IntoResponse, AssistantError> {
    let client = build_client()?;
    let worker_url = app_config.worker_url();
    let url = worker_url.join("api/v0/journal/summarize").map_err(|error| {
        tracing::error!(error = &error as &dyn Error, "error building worker endpoint url");
        AssistantError::Other
    })?;
    tracing::debug!(%user_id, url = %url, "calling summarize endpoint of worker");
    let resp = client
        .post(url)
        .json(&WorkerSummaryOptions {
            user_id,
            time: options.time.unwrap_or_else(|| Utc::now().fixed_offset()),
        })
        .send()
        .await
        .inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, "error calling worker summarize endpoint");
        })?;
    let status = resp.status();
    if status.is_success() || status == StatusCode::NOT_FOUND {
        tracing::debug!(%user_id, ?status, "worker returned response");
        let res = resp.json::<SummaryResponse>().await;
        let summary = res.map_err(|error| {
            tracing::error!(error = &error as &dyn Error, "error parsing worker response");
            AssistantError::Other
        })?;

        tracing::debug!(%user_id, "returning summary response");
        let status_code = StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        return Ok((status_code, Json(summary).into_response()));
    }
    tracing::error!(%user_id, status = ?resp.status(), "worker returned error");
    Err(AssistantError::Other)
}

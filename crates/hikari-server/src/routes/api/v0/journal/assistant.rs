pub(crate) mod error;
pub(crate) mod summarize;
use crate::AppConfig;
use crate::permissions::Permission;
use crate::routes::api::v0::journal::assistant::error::{AssistantError, AssistantErrorType};
use crate::routes::error::ErrorData;
use crate::user::ExtractUserId;
use axum::response::IntoResponse;
use axum::routing::{Router, post};
use axum::{Extension, Json};
use hikari_core::journal::assistant::{
    MergeResponse, PromptResponse, generate_prompt, generate_text_prompt, merge_prompts, text_merge_prompts,
};
use http::StatusCode;
use protect_axum::protect;
use sea_orm::DatabaseConnection;
use serde_derive::Deserialize;
use summarize::summarize_handler;
use utoipa::ToSchema;

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/merge", post(merge))
        .route("/prompt", post(prompt))
        .route("/summarize", post(summarize_handler))
        .route("/text_merge", post(text_merge))
        .route("/text_prompt", post(text_prompt))
        .with_state(())
}

#[derive(Debug, Clone, ToSchema, Deserialize)]
pub(crate) struct PromptInput {
    pub prompt: String,
    pub input: String,
}

#[derive(Debug, Clone, ToSchema, Deserialize)]
pub(crate) struct TextPromptInput {
    pub prompts: Vec<String>,
    pub input: String,
}

#[derive(Debug, Clone, ToSchema, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct TextMergeInput {
    pub original: TextPromptInput,
    pub other: Vec<PromptInput>,
}

/// Gets a prompt from the assistant.
#[utoipa::path(
    post,
    path = "/api/v0/journal/assistant/prompt",
    request_body(content = PromptInput, description = "The user input."),
    responses(
        (status = OK, description = "The prompt from the assistant.", body = PromptResponse),
        (status = INTERNAL_SERVER_ERROR, description = "Something went wrong. Check response body.", body = ErrorData<AssistantErrorType>),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Journal", ty = "Permission")]
pub(crate) async fn prompt(
    Extension(app_config): Extension<AppConfig>,
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Json(body): Json<PromptInput>,
) -> Result<impl IntoResponse, AssistantError> {
    let PromptInput { prompt, input } = body;
    let llm_config = app_config.llm_config();

    let res: PromptResponse = generate_prompt(&user_id, prompt, input, llm_config, &conn).await?;

    Ok(Json(res).into_response())
}

/// Merge multiple prompt responses
///
/// This can, for example, be used to multiple responses after consulting the /prompt endpoint.
#[utoipa::path(
    post,
    path = "/api/v0/journal/assistant/merge",
    request_body(content = Vec<PromptInput>, description = "Prompt responses to merge."),
    responses(
        (status = OK, description = "The prompt from the assistant.", body = MergeResponse),
        (status = INTERNAL_SERVER_ERROR, description = "Something went wrong. Check response body.", body = ErrorData<AssistantErrorType>),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Journal", ty = "Permission")]
pub(crate) async fn merge(
    Extension(app_config): Extension<AppConfig>,
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Json(prompt_inputs): Json<Vec<PromptInput>>,
) -> Result<impl IntoResponse, AssistantError> {
    if prompt_inputs.len() < 2 {
        return Ok((StatusCode::BAD_REQUEST, "Need at least two inputs").into_response());
    }

    let prompt_inputs: Vec<(String, String)> = prompt_inputs
        .into_iter()
        .map(|PromptInput { prompt, input }| (prompt, input))
        .collect();

    let res: MergeResponse = merge_prompts(&user_id, prompt_inputs, app_config.llm_config(), &conn).await?;

    Ok(Json(res).into_response())
}

/// Gets a prompt from the assistant.
///
/// Specialized function for text prompts
#[utoipa::path(
    post,
    path = "/api/v0/journal/assistant/text_prompt",
    request_body(content = PromptInput, description = "The user input."),
    responses(
        (status = OK, description = "The prompt from the assistant.", body = PromptResponse),
        (status = INTERNAL_SERVER_ERROR, description = "Something went wrong. Check response body.", body = ErrorData<AssistantErrorType>),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Journal", ty = "Permission")]
pub(crate) async fn text_prompt(
    Extension(app_config): Extension<AppConfig>,
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Json(body): Json<TextPromptInput>,
) -> Result<impl IntoResponse, AssistantError> {
    let TextPromptInput { prompts, input } = body;

    let llm_config = app_config.llm_config();

    let res: PromptResponse = generate_text_prompt(&user_id, prompts, input, llm_config, &conn).await?;

    Ok(Json(res).into_response())
}

/// Merge multiple prompt responses
///
/// This can, for example, be used to multiple responses after consulting the /`text_prompt` endpoint.
#[utoipa::path(
    post,
    path = "/api/v0/journal/assistant/text_merge",
    request_body(content = TextMergeInput, description = "Prompt responses to merge."),
    responses(
        (status = OK, description = "The prompt from the assistant.", body = MergeResponse),
        (status = INTERNAL_SERVER_ERROR, description = "Something went wrong. Check response body.", body = ErrorData<AssistantErrorType>),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Journal", ty = "Permission")]
pub(crate) async fn text_merge(
    Extension(app_config): Extension<AppConfig>,
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Json(body): Json<TextMergeInput>,
) -> Result<impl IntoResponse, AssistantError> {
    let TextMergeInput {
        original: TextPromptInput {
            prompts: original_prompts,
            input: original_input,
        },
        other: prompts_inputs,
    } = body;
    if prompts_inputs.is_empty() {
        return Ok((StatusCode::BAD_REQUEST, "Need at least one input").into_response());
    }

    let prompt_inputs: Vec<(String, String)> = prompts_inputs
        .into_iter()
        .map(|PromptInput { prompt, input }| (prompt, input))
        .collect();

    let res: MergeResponse = text_merge_prompts(
        &user_id,
        original_input,
        original_prompts,
        prompt_inputs,
        app_config.llm_config(),
        &conn,
    )
    .await?;

    Ok(Json(res).into_response())
}

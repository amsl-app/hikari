pub(crate) mod error;
pub(crate) mod summarize;

use crate::AppConfig;
use crate::permissions::Permission;
use crate::routes::api::v0::journal::assistant::error::{AssistantError, AssistantErrorType};
use crate::routes::error::ErrorData;
use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
};
use axum::response::IntoResponse;
use axum::routing::{Router, post};
use axum::{Extension, Json};
use hikari_core::openai::error::OpenAiError;
use hikari_core::openai::{CallConfig, FunctionResponse, openai_call_function_with_timeout};
use http::StatusCode;
use protect_axum::protect;
use serde_derive::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;
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

#[derive(Debug, ToSchema, Serialize, Deserialize)]
pub(crate) struct PromptResponse {
    pub summary: String,
    pub prompt: String,
}

impl FunctionResponse for PromptResponse {
    fn function_name() -> &'static str {
        "tiefere_nachfrage"
    }

    fn function_description() -> &'static str {
        "Erstellen einer tiefere Nachfrage auf deutsch basierend auf der gestellten Frage sowie der noch unfertigen Nutzereingabe."
    }

    fn function_definition() -> Value {
        json! (
            {
                "type": "object",
                "properties": {
                  "summary": { "type": "string", "description": "Deutsche Zusammenfassung der bisherigen Eingabe des Nutzers in maximal zwei Sätzen und eigenen Worten. Ohne Wertung oder Interpretation." },
                  "prompt": {
                    "type": "string",
                    "description": "Präzise Frage die dem Nutzer gestellt werden soll um den Inhalt zu vertiefen.",
                  },
                },
                "required": ["prompt", "summary"]
            }
        )
    }

    fn fix_escapes(&mut self) {
        self.summary = html_escape::decode_html_entities(&self.summary).to_string();
        self.prompt = html_escape::decode_html_entities(&self.prompt).to_string();
    }
}

#[derive(Debug, Clone, ToSchema, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct TextMergeInput {
    pub original: TextPromptInput,
    pub other: Vec<PromptInput>,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub(crate) struct MergeResponse {
    pub alternatives: Vec<String>,
}

impl FunctionResponse for MergeResponse {
    fn function_name() -> &'static str {
        "text_zusammenfuehren"
    }

    fn function_description() -> &'static str {
        "Erstelle deutsche Formulierungen basierend auf den Antworten des Nutzers für einen Journaleintrag."
    }

    fn function_definition() -> Value {
        json! (
            {
              "type": "object",
              "properties": {
                "alternatives": {
                  "type": "array",
                  "items": {
                    "type": "string"
                  },
                  "description": "Mögliche Formulierungsalternativen für einen Journaleintrag. Alle alternativen sollen aus der Ich-Perspektive geschrieben sein.",
                },
              },
              "required": ["alternatives"]
            }
        )
    }

    fn fix_escapes(&mut self) {
        self.alternatives.iter_mut().for_each(|s| {
            *s = html_escape::decode_html_entities(s).to_string();
        });
    }
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
    Json(body): Json<PromptInput>,
) -> Result<impl IntoResponse, AssistantError> {
    let PromptInput { prompt, input } = body;

    // Single system prompt because some AI models only support a single system message
    let mut messages: Vec<ChatCompletionRequestMessage> = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(
                format!(
                    "\
Du bist ein Assistent der Studenten helfen soll ihr Lernjournal zu erstellen.\n\
Das Lernjournal wird in einem Dialog mit einem Chatbot erstellt, dessen Rolle du einnimmst.\n\
Der Nutzer hat mehrere zusammenhängende Fragen beantwortet.\n\
\n
Umschreibe die Eingabe des Nutzers auf deutsch in dem du sie kurz aus der Sicht des Nutzers, in maximal 2 Sätzen, zusammenfasst. Nicht mehr, keine Wertung, Interpretation oder Fragen. \n\
Die Zusammenfassung soll den Nutzer Duzen (in zweiter Person ansprechen) und zeigen, dass du ihn verstanden hast. \n\
\n
Dann stelle eine präzise Rückfrage basierend auf der gestellten Frage sowie der noch unfertigen Nutzereingabe. Die frage sollte mehr in die Tiefe gehen.\n\
Die Rückfrage soll dem Nutzer helfen seinen Journaleintrag zu erweitern.\n\
\n
Rufe die Funktion `{}` auf um die Umschreibung und die Rückfrage zurückzugeben. Verwende für den Funktionsaufruf valides JSON.\n\
\n
Es folgt die Frage die dem Nutzer gestellt wurde sowie der dazugehörigen noch unfertigen Antwort:\
",
                    PromptResponse::function_name()
                ),
            )
            .build()
            .map_err(OpenAiError::from)?
            .into(),
    ];

    messages.push(
        ChatCompletionRequestAssistantMessageArgs::default()
            .content(prompt.clone())
            .build()
            .map_err(OpenAiError::from)?
            .into(),
    );

    messages.push(
        ChatCompletionRequestUserMessageArgs::default()
            .content(input)
            .build()
            .map_err(OpenAiError::from)?
            .into(),
    );

    tracing::info!("sending {} messages to openAI", messages.len());
    let call_config = CallConfig::builder()
        .iteration_timeout(Duration::from_secs(25))
        .total_timeout(Duration::from_secs(60))
        .max_retry_interval(Duration::from_secs(1))
        .build();
    let res: PromptResponse = openai_call_function_with_timeout(app_config.llm_config(), call_config, messages).await?;

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
    Json(prompt_inputs): Json<Vec<PromptInput>>,
) -> Result<impl IntoResponse, AssistantError> {
    if prompt_inputs.len() < 2 {
        return Ok((StatusCode::BAD_REQUEST, "Need at least two inputs").into_response());
    }

    let mut messages: Vec<ChatCompletionRequestMessage> = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(
                "\
Du bist ein Assistent der Studenten helfen soll ihr Lernjournal zu erstellen. \
Das Lernjournal wird in einem Dialog mit einem Chatbot erstellt, dessen Rolle du einnimmst. \
\n
Die Fragen des Chatbots und zugehörigen Antworten des Nutzers folgen als Nächstes immer abwechselnd. \
"
                .to_string(),
            )
            .build()
            .map_err(OpenAiError::from)?
            .into(),
    ];

    add_prompt_inputs(&mut messages, prompt_inputs)?;

    messages.push(
        ChatCompletionRequestSystemMessageArgs::default()
            .content(format!(
                "\
Generiere aus Fragen und Antworten verschiedene, vollständige Formulierungen auf deutsch den der Nutzer als Journal Eintrag verwenden kann.\n\
Liefere zwei bis drei Alternativen.\n\
Alle alternativen sollen aus der Ich-Perspektive geschrieben sein.

Rufe die Funktion `{}` auf die Formulierungen zurückzugeben. Verwende für den Funktionsaufruf valides JSON.\
",
                MergeResponse::function_name()
            ))
            .build()
            .map_err(OpenAiError::from)?
            .into(),
    );

    tracing::info!("sending {} messages to openAI", messages.len());
    let call_config = CallConfig::builder()
        .iteration_timeout(Duration::from_secs(25))
        .total_timeout(Duration::from_secs(60))
        .max_retry_interval(Duration::from_secs(1))
        .build();
    let res: MergeResponse = openai_call_function_with_timeout(app_config.llm_config(), call_config, messages).await?;

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
    Json(body): Json<TextPromptInput>,
) -> Result<impl IntoResponse, AssistantError> {
    let TextPromptInput { prompts, input } = body;

    let mut initial_prompt =
        "Du bist ein Assistent der Studenten helfen soll ihr Lernjournal zu erstellen.".to_string();

    let mut followup_specification = "";
    if !prompts.is_empty() {
        initial_prompt.push_str(
            "\n\
Der Nutzer ist gerade dabei eine liste von Frage zu beantworten.\n\
Es folgen die Fragen.",
        );
        followup_specification = " basierend auf den gestellten Fragen sowie der noch unfertigen Nutzereingabe";
    }

    let mut messages: Vec<ChatCompletionRequestMessage> = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(initial_prompt)
            .build()
            .map_err(OpenAiError::from)?
            .into(),
    ];

    for prompt in prompts {
        messages.push(
            ChatCompletionRequestAssistantMessageArgs::default()
                .content(prompt)
                .build()
                .map_err(OpenAiError::from)?
                .into(),
        );
    }

    messages.push(
        ChatCompletionRequestSystemMessageArgs::default()
            .content(
                "\
Als nächstes folgt noch unfertige Antwort des Nutzers auf diese Fragen.\
"
                .to_string(),
            )
            .build()
            .map_err(OpenAiError::from)?
            .into(),
    );

    messages.push(
        ChatCompletionRequestUserMessageArgs::default()
            .content(input)
            .build()
            .map_err(OpenAiError::from)?
            .into(),
    );

    messages.push(
        ChatCompletionRequestSystemMessageArgs::default()
            .content(format!(
                "\
Umschreibe die Eingabe des Nutzers auf deutsch in dem du sie kurz aus der Sicht des Nutzers, in maximal 2 Sätzen, zusammenfasst. Nicht mehr, keine Wertung, Interpretation oder Fragen. \n\
Die Zusammenfassung soll den Nutzer Duzen (in zweiter Person ansprechen) und zeigen das du ihn verstanden hast. \n\
\n
Dann stelle eine präzise deutsche Rückfrage{followup_specification}. Die Frage soll mehr in die Tiefe gehen.\n\
Die Rückfrage soll dem Nutzer helfen seinen Journaleintrag zu erweitern.

Rufe die Funktion `{}` auf die Umschreibung und Frage zurückzugeben. Verwende für den Funktionsaufruf valides JSON.\
",
                PromptResponse::function_name()
            ))
            .build()
            .map_err(OpenAiError::from)?
            .into(),
    );

    tracing::info!("sending {} messages to openAI", messages.len());
    let call_config = CallConfig::builder()
        .iteration_timeout(Duration::from_secs(25))
        .total_timeout(Duration::from_secs(60))
        .max_retry_interval(Duration::from_secs(1))
        .build();
    let res: PromptResponse = openai_call_function_with_timeout(app_config.llm_config(), call_config, messages).await?;
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
    Json(body): Json<TextMergeInput>,
) -> Result<impl IntoResponse, AssistantError> {
    let TextMergeInput {
        original: original_prompt,
        other: other_prompts,
    } = body;
    if other_prompts.is_empty() {
        return Ok((StatusCode::BAD_REQUEST, "Need at least one input").into_response());
    }

    let mut initial_prompt =
        "Du bist ein Assistent der Studenten helfen soll ihr Lernjournal zu erstellen.".to_string();

    if !original_prompt.prompts.is_empty() {
        initial_prompt.push_str(
            "\n
Anfangs hat der Nutzer eine liste von Fragen erhalten auf die er Antworten soll.\
Die Liste der Fragen folgt als Nächstes.",
        );
    }

    let mut messages: Vec<ChatCompletionRequestMessage> = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(initial_prompt)
            .build()
            .map_err(OpenAiError::from)?
            .into(),
    ];

    let TextPromptInput { prompts, input } = original_prompt;
    for prompt in prompts {
        messages.push(
            ChatCompletionRequestAssistantMessageArgs::default()
                .content(prompt)
                .build()
                .map_err(OpenAiError::from)?
                .into(),
        );
    }

    messages.push(
        ChatCompletionRequestSystemMessageArgs::default()
            .content("Auf diese Fragen hat der Nutzer mit Folgender, noch unfertigen Eingabe, geantwortet".to_string())
            .build()
            .map_err(OpenAiError::from)?
            .into(),
    );

    messages.push(
        ChatCompletionRequestUserMessageArgs::default()
            .content(input)
            .build()
            .map_err(OpenAiError::from)?
            .into(),
    );

    messages.push(ChatCompletionRequestSystemMessageArgs::default().content(
            format!("Anschließend wurden dem Nutzer {} Rückfragen gestellt. Die Rückfragen und Antworten des Nutzers folgen als Nächstes", other_prompts.len())
        ).build().map_err(OpenAiError::from)?.into());

    add_prompt_inputs(&mut messages, other_prompts)?;

    messages.push(ChatCompletionRequestSystemMessageArgs::default().content(
            format!("\
Generiere aus Fragen und Antworten verschiedene, vollständige Formulierungen auf deutsch den der Nutzer als Journal Eintrag verwenden kann.\n\
Liefere zwei bis drei Alternativen.\n\
Alle alternativen sollen aus der Ich-Perspektive geschrieben sein.

Rufe die Funktion `{}` auf um die Formulierungen zurückzugeben. Verwende für den Funktionsaufruf valides JSON.", MergeResponse::function_name()),
        ).build().map_err(OpenAiError::from)?.into());

    tracing::info!("sending {} messages to openAI", messages.len());
    let call_config = CallConfig::builder()
        .iteration_timeout(Duration::from_secs(30))
        .total_timeout(Duration::from_secs(65))
        .max_retry_interval(Duration::from_secs(1))
        .build();
    let res: MergeResponse = openai_call_function_with_timeout(app_config.llm_config(), call_config, messages).await?;

    Ok(Json(res).into_response())
}

fn add_prompt_inputs(
    messages: &mut Vec<ChatCompletionRequestMessage>,
    prompt_inputs: Vec<PromptInput>,
) -> Result<(), AssistantError> {
    for PromptInput { prompt, input } in prompt_inputs {
        messages.push(
            ChatCompletionRequestAssistantMessageArgs::default()
                .content(prompt)
                .build()
                .map_err(OpenAiError::from)?
                .into(),
        );
        messages.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(input)
                .build()
                .map_err(OpenAiError::from)?
                .into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_response_fix_escapes() {
        let mut res = PromptResponse {
            summary: "f&uuml; &amp; bar".to_string(),
            prompt: "fo&#246; &amp; b&#xe4;r".to_string(),
        };
        res.fix_escapes();
        assert_eq!(res.summary, "fü & bar");
        assert_eq!(res.prompt, "foö & bär");
    }

    #[test]
    fn test_merge_response_fix_escapes() {
        let mut res = MergeResponse {
            alternatives: vec!["foo &amp; bar".to_string(), "foo &gt; bar".to_string()],
        };
        res.fix_escapes();
        assert_eq!(res.alternatives, vec!["foo & bar".to_string(), "foo > bar".to_string()]);
    }
}

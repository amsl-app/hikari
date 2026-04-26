use std::time::Duration;
use tracing::instrument;

use crate::{
    journal::assistant::error::AssistantError,
    llm_config::LlmConfig,
    openai::{CallConfig, error::OpenAiError, openai_single_tool_call},
};
use async_openai::types::chat::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
};
use schemars::JsonSchema;
use sea_orm::{DatabaseConnection, prelude::Uuid};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub mod error;

#[derive(Debug, ToSchema, Serialize, Deserialize, JsonSchema)]
#[schemars(
    title = "TiefereNachfrage",
    description = "Erstellen einer tiefere Nachfrage auf deutsch basierend auf der gestellten Frage sowie der noch unfertigen Nutzereingabe."
)]
pub struct PromptResponse {
    /// Deutsche Zusammenfassung der bisherigen Eingabe des Nutzers in maximal zwei Sätzen und eigenen Worten. Ohne Wertung oder Interpretation.
    pub summary: String,
    /// Präzise Frage die dem Nutzer gestellt werden soll um den Inhalt zu vertiefen.
    pub prompt: String,
}

impl PromptResponse {
    fn fix_escapes(&mut self) {
        self.summary = html_escape::decode_html_entities(&self.summary).to_string();
        self.prompt = html_escape::decode_html_entities(&self.prompt).to_string();
    }
}

#[instrument(skip(llm_config, conn))]
pub async fn generate_prompt(
    user_id: &Uuid,
    prompt: String,
    input: String,
    llm_config: &LlmConfig,
    conn: &DatabaseConnection,
) -> Result<PromptResponse, AssistantError> {
    // Single system prompt because some AI models only support a single system message
    let mut messages: Vec<ChatCompletionRequestMessage> = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(
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
                Rufe die Funktion `TiefereNachfrage` auf um die Umschreibung und die Rückfrage zurückzugeben. Verwende für den Funktionsaufruf valides JSON.\n\
                \n
                Es folgt die Frage die dem Nutzer gestellt wurde sowie der dazugehörigen noch unfertigen Antwort:\
                "
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

    let openai_config = llm_config.get_journaling_openai_config();
    let model = llm_config.get_journaling_model();

    let (mut res, tokens) = openai_single_tool_call::<PromptResponse>(
        CallConfig::builder()
            .iteration_timeout(Duration::from_secs(25))
            .total_timeout(Duration::from_secs(60))
            .max_retry_interval(Duration::from_secs(1))
            .build(),
        openai_config,
        None,
        None,
        model,
        messages,
    )
    .await?;

    if let Some(usage) = tokens {
        hikari_db::llm::usage::Mutation::add_usage(conn, user_id, usage, "assistant_prompt".to_owned()).await?;
    }

    res.fix_escapes();
    Ok(res)
}

#[instrument(skip(llm_config, conn))]
pub async fn generate_text_prompt(
    user_id: &Uuid,
    prompts: Vec<String>,
    input: String,
    llm_config: &LlmConfig,
    conn: &DatabaseConnection,
) -> Result<PromptResponse, AssistantError> {
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
            .content("Als nächstes folgt noch unfertige Antwort des Nutzers auf diese Fragen.".to_string())
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

                Rufe die Funktion `TiefereNachfrage` auf die Umschreibung und Frage zurückzugeben. Verwende für den Funktionsaufruf valides JSON.\
                "
            )
            )
            .build()
            .map_err(OpenAiError::from)?
            .into(),
    );

    tracing::info!("sending {} messages to openAI", messages.len());

    let openai_config = llm_config.get_journaling_openai_config();
    let model = llm_config.get_journaling_model();

    let (mut res, tokens) = openai_single_tool_call::<PromptResponse>(
        CallConfig::builder()
            .iteration_timeout(Duration::from_secs(25))
            .total_timeout(Duration::from_secs(60))
            .max_retry_interval(Duration::from_secs(1))
            .build(),
        openai_config,
        None,
        None,
        model,
        messages,
    )
    .await?;

    if let Some(usage) = tokens {
        hikari_db::llm::usage::Mutation::add_usage(conn, user_id, usage, "assisstant_text_prompt".to_owned()).await?;
    }

    res.fix_escapes();
    Ok(res)
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize, JsonSchema)]
#[schemars(
    title = "TextZusammenfuehren",
    description = "Erstelle deutsche Formulierungen basierend auf den Antworten des Nutzers für einen Journaleintrag."
)]
pub struct MergeResponse {
    /// Mögliche Formulierungsalternativen für einen Journaleintrag. Alle alternativen sollen aus der Ich-Perspektive geschrieben sein.
    pub alternatives: Vec<String>,
}

impl MergeResponse {
    fn fix_escapes(&mut self) {
        self.alternatives.iter_mut().for_each(|s| {
            *s = html_escape::decode_html_entities(s).to_string();
        });
    }
}

#[instrument(skip(llm_config, conn))]
pub async fn merge_prompts(
    user_id: &Uuid,
    prompt_inputs: Vec<(String, String)>,
    llm_config: &LlmConfig,
    conn: &DatabaseConnection,
) -> Result<MergeResponse, AssistantError> {
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
            .content(
                "\
                Generiere aus Fragen und Antworten verschiedene, vollständige Formulierungen auf deutsch den der Nutzer als Journal Eintrag verwenden kann.\n\
                Liefere zwei bis drei Alternativen.\n\
                Alle alternativen sollen aus der Ich-Perspektive geschrieben sein.

                Rufe die Funktion `TextZusammenfuehren` auf die Formulierungen zurückzugeben. Verwende für den Funktionsaufruf valides JSON."
            )
            .build()
            .map_err(OpenAiError::from)?
            .into(),
    );

    tracing::info!("sending {} messages to openAI", messages.len());

    let openai_config = llm_config.get_journaling_openai_config();
    let model = llm_config.get_journaling_model();

    let (mut res, tokens) = openai_single_tool_call::<MergeResponse>(
        CallConfig::builder()
            .iteration_timeout(Duration::from_secs(25))
            .total_timeout(Duration::from_secs(60))
            .max_retry_interval(Duration::from_secs(1))
            .build(),
        openai_config,
        None,
        None,
        model,
        messages,
    )
    .await?;

    if let Some(usage) = tokens {
        hikari_db::llm::usage::Mutation::add_usage(conn, user_id, usage, "assisstant_merge".to_owned()).await?;
    }

    res.fix_escapes();
    Ok(res)
}

#[instrument(skip(llm_config, conn))]
pub async fn text_merge_prompts(
    user_id: &Uuid,
    original_input: String,
    original_prompts: Vec<String>,
    prompt_inputs: Vec<(String, String)>,
    llm_config: &LlmConfig,
    conn: &DatabaseConnection,
) -> Result<MergeResponse, AssistantError> {
    let mut initial_prompt =
        "Du bist ein Assistent der Studenten helfen soll ihr Lernjournal zu erstellen.".to_string();

    if !original_prompts.is_empty() {
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

    for prompt in original_prompts {
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
            .content(original_input)
            .build()
            .map_err(OpenAiError::from)?
            .into(),
    );

    messages.push(ChatCompletionRequestSystemMessageArgs::default().content(
            format!("Anschließend wurden dem Nutzer {} Rückfragen gestellt. Die Rückfragen und Antworten des Nutzers folgen als Nächstes", prompt_inputs.len())
        ).build().map_err(OpenAiError::from)?.into());

    add_prompt_inputs(&mut messages, prompt_inputs)?;

    messages.push(ChatCompletionRequestSystemMessageArgs::default().content(
            "\
Generiere aus Fragen und Antworten verschiedene, vollständige Formulierungen auf deutsch den der Nutzer als Journal Eintrag verwenden kann.\n\
Liefere zwei bis drei Alternativen.\n\
Alle alternativen sollen aus der Ich-Perspektive geschrieben sein.

Rufe die Funktion `TextZusammnfuehren` auf um die Formulierungen zurückzugeben. Verwende für den Funktionsaufruf valides JSON."
        ).build().map_err(OpenAiError::from)?.into());

    tracing::info!("sending {} messages to openAI", messages.len());

    let openai_config = llm_config.get_journaling_openai_config();
    let model = llm_config.get_journaling_model();

    let (mut res, tokens) = openai_single_tool_call::<MergeResponse>(
        CallConfig::builder()
            .iteration_timeout(Duration::from_secs(25))
            .total_timeout(Duration::from_secs(60))
            .max_retry_interval(Duration::from_secs(1))
            .build(),
        openai_config,
        None,
        None,
        model,
        messages,
    )
    .await?;

    if let Some(usage) = tokens {
        hikari_db::llm::usage::Mutation::add_usage(conn, user_id, usage, "assisstant_text_merge".to_owned()).await?;
    }

    res.fix_escapes();
    Ok(res)
}

fn add_prompt_inputs(
    messages: &mut Vec<ChatCompletionRequestMessage>,
    prompt_inputs: Vec<(String, String)>,
) -> Result<(), AssistantError> {
    for (prompt, input) in prompt_inputs {
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

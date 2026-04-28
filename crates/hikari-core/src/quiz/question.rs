use crate::llm_config::LlmConfig;
use crate::openai::{CallConfig, openai_single_tool_call};
use crate::pgvector::search;
use crate::quiz::error::QuizError;
use crate::quiz::max_five_random_exam_questions;
use async_openai::types::chat::{
    ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessage, ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageContent,
};
use hikari_config::module::content::{ContentExam, QuestionBloomLevel};
use hikari_model::llm::vector::embedding_chunk::LlmEmbeddingQueryResult;
use hikari_model::quiz::question::{Question, QuestionFeedback};
use hikari_model_tools::convert::{IntoDbModel, IntoModel};
use rand::rng;
use rand::seq::IndexedRandom;
use schemars::JsonSchema;
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::instrument;

#[derive(Debug, Clone, Default)]
struct Operators {
    dos: Vec<&'static str>,
    donts: Vec<&'static str>,
}

#[derive(Serialize, JsonSchema, Deserialize)]
#[schemars(description = "Die generierte Frage. Es gibt entweder eine Textfrage oder eine Multiple Choice Frage.")]
struct QuizQuestion {
    question: QuestionType,
}

#[derive(Serialize, JsonSchema, Deserialize)]
#[schemars(
    description = "Die Art der Frage. Es gibt entweder eine Textfrage oder eine Multiple Choice Frage.",
    inline
)]
#[serde(untagged)]
enum QuestionType {
    Text(TextQuestion),
    MultipleChoice(MultipleChoiceQuestion),
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Dieses Tool sendet die textuelle Frage an den Nutzer.", inline)]
struct TextQuestion {
    /// Die Frage, die an den Nutzer gestellt werden soll.
    question: String,
    // Die richtige Antwort auf die Frage.
    solution: String,
}

#[derive(Serialize, JsonSchema, Deserialize)]
#[schemars(
    description = "Dieses Tool sended die Multiple Choice frage an den Nutzer. \
    Es enthält die Frage sowie die Antwortmöglichkeiten. Es muss mindestens eine richtige Antwortmöglichkeit geben.",
    inline
)]
struct MultipleChoiceQuestion {
    /// Die Frage, die an den Nutzer gestellt werden soll.
    question: String,
    /// Die Antwortmöglichkeiten. Ca. 4-5 Möglichkeiten wobei mindestens eine richtig sein soll.
    options: Vec<MultipleChoiceOption>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Eine Antwortmöglichkeit für eine Multiple Choice Frage.", inline)]
struct MultipleChoiceOption {
    /// Die textuelle Anwortmöglichkeit
    option: String,
    /// True, wenn dies eine richtige Möglichkeit ist
    correct: bool,
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip(exams, llm_config, conn), err)]
pub async fn create_question(
    user_id: &Uuid,
    session_id: &str,
    content: &str,
    topic: &str,
    exams: &[(String, ContentExam)],
    llm_config: &LlmConfig,
    conn: &DatabaseConnection,
    quiz_id: &Uuid,
    rag_documents: &[String],
) -> Result<Question, QuizError> {
    let score: f64 =
        (hikari_db::quiz::score::Query::get_score_by_topic(conn, user_id, session_id, topic).await?).unwrap_or(0.0);
    let level = random_bloom_level(score);
    tracing::debug!(%score, ?level, "determined bloom level for question generation");

    let old_questions = hikari_db::quiz::question::Query::get_question_by_user_topic_level(
        conn,
        user_id,
        topic,
        &level.into_db_model(),
    )
    .await?;

    // Get newest 10 questions
    let mut old_questions_model: Vec<Question> = old_questions
        .into_iter()
        .map(hikari_model_tools::convert::IntoModel::into_model)
        .collect();

    // Only keep questions without bad feedback and with grade > 3
    old_questions_model
        .retain(|q| !q.feedback.eq(&Some(QuestionFeedback::Bad)) && q.grade.is_some_and(|grade| grade > 3));
    old_questions_model.sort_by_key(|q| q.created_at);
    old_questions_model.reverse();
    old_questions_model.truncate(10);

    let operators = OPERATORS.get(&level).cloned().unwrap_or_default();

    // Shuffle and limit to 5 questions
    let max_five_exam_questions: Vec<(String, ContentExam)> = max_five_random_exam_questions(exams.to_owned(), level);

    let sources: Vec<LlmEmbeddingQueryResult> = search(llm_config, conn, content, 5, rag_documents).await?;

    let sources_string: String = sources
        .iter()
        .enumerate()
        .map(|(i, e)| format!("# Source {}\n{}", i + 1, e.content))
        .collect::<Vec<_>>()
        .join("\n\n");

    let mut prompt_messages: Vec<ChatCompletionRequestMessage> = vec![ChatCompletionRequestMessage::System(
        ChatCompletionRequestSystemMessage {
            content: ChatCompletionRequestSystemMessageContent::Text(format!("
                # ROLLE
                Du bist ein didaktisch versierter Hochschulprüfer und Experte für die Erstellung von Prüfungsfragen basierend auf Blooms Revised Taxonomy. Deine Aufgabe ist es, eine präzise, faire und anspruchsvolle Prüfungsfrage zu erstellen, die ausschließlich auf dem bereitgestellten Kontext basiert.

                # INPUT DATEN
                Die folgenden Parameter bestimmen die zu erstellende Frage:
                - **Blooms Level:** {}
                - **Zu nutzende Operatoren:** {}
                - **Verbotene Operatoren:** {}
                - **Lernmaterial (Kontext):**
                <context_sources>
                {}
                </context_sources>

                # ANWEISUNGEN Schritt für Schritt:
                1. **Analyse:** Lies das Lernmaterial gründlich. Identifiziere Kernkonzepte, die zum geforderten Blooms Level passen.
                2. **Konstruktion:** Erstelle genau EINE Prüfungsfrage. Entscheide dabei, ob es eine Textfrage oder eine Multiple-Choice-Frage wird, basierend auf dem Blooms Level und der Thematik.
                3. **Operatoren-Einsatz:** - Wähle 1-2 Operatoren aus der Liste der erlaubten Operatoren (oder passend zum Level).
                - Integriere diese grammatikalisch korrekt in den Satz. Beachte, dass du die Operatoren teilweise anpassen musst, damit sie in den Satz passen.
                - **Formatierung:** Markiere die Operatoren im Satz fett mit Markdown (z. B. **Analysieren** Sie...). Markiere NICHTS anderes fett.
                4. **Autarkie:** Die Frage muss ’self-contained' sein. Wenn ein Szenario oder Sachverhalt notwendig ist, um die Frage zu beantworten, stelle diesen VOR die eigentliche Frage. Der Student darf kein externes Wissen benötigen. Ausdrücke wie ’im obigen Text' sind nicht erlaubt.

                # CONSTRAINTS (Einschränkungen)
                - **KEINE Lösungen in der Frage:** Gib unter keinen Umständen die Lösung oder Beispiele für die Lösung mit in der Frage an. Die Lösung wird separat erfasst.
                - **KEIN Meta-Talk:** Beginne nicht mit 'Frage:', 'Aufgabe:' oder einer Einleitung. Gib nur den Sachverhalt (falls nötig) und die Frage aus.
                - **Quellentreue:** Stelle sicher, dass die Frage zu 100% mit dem bereitgestellten Lernmaterial beantwortbar ist. Halluziniere keine Fakten hinzu.

                # OUTPUT
                Erstelle nun die finale Prüfungsfrage basierend auf den obigen Anweisungen.
                Bei Textfragen, gib außerdem noch die richte Antwort auf die Frage an.
                Bei Multiple-Choice-Fragen, gib außerdem die Antwortmöglichkeiten an (2-5) wobei mindestens eine richtig sein muss.
                ",
                level,
                operators.dos.join(", "),
                operators.donts.join(", "),
                sources_string
            )),
            name: None,
        },
    )];

    for (question_topic, question) in max_five_exam_questions {
        tracing::debug!(
            "Adding exam question for topic '{}': {}",
            question_topic,
            question.question
        );
        prompt_messages.push(ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
            content: ChatCompletionRequestUserMessageContent::Text(format!(
                "Bitte generiere mir eine Frage auf bases deiner aktuellen Kontextinformation zum Thema '{question_topic}'."
            )),
            name: None,
        }));

        let mut question_json = serde_json::to_value(&question).unwrap_or_default();
        if let Some(obj) = question_json.as_object_mut() {
            obj.remove("level");
        }

        prompt_messages.push(ChatCompletionRequestMessage::Assistant(
            ChatCompletionRequestAssistantMessage {
                content: Some(ChatCompletionRequestAssistantMessageContent::Text(
                    question_json.to_string(),
                )),
                refusal: None,
                audio: None,
                tool_calls: None,
                #[allow(deprecated)]
                function_call: None,
                name: None,
            },
        ));
    }

    for old_question in old_questions_model {
        tracing::debug!(
            "Adding old question for topic '{}': {}",
            old_question.topic,
            old_question.question
        );

        let mut question_json = serde_json::to_value(&old_question).unwrap_or_default();

        if let Some(obj) = question_json.as_object_mut() {
            obj.retain(|key, _| key == "question" || key == "ai_solution" || key == "options");
        }

        if let Some(obj) = question_json.as_object_mut()
            && let Some(ai_solution) = obj.remove("ai_solution")
        {
            obj.insert("solution".to_string(), ai_solution);
        }

        prompt_messages.push(ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
            content: ChatCompletionRequestUserMessageContent::Text(format!(
                "Bitte generiere mir eine Frage auf bases deiner aktuellen Kontextinformation zum Thema '{}'.",
                old_question.topic
            )),
            name: None,
        }));
        prompt_messages.push(ChatCompletionRequestMessage::Assistant(
            ChatCompletionRequestAssistantMessage {
                content: Some(ChatCompletionRequestAssistantMessageContent::Text(
                    question_json.to_string(),
                )),
                refusal: None,
                audio: None,
                tool_calls: None,
                #[allow(deprecated)]
                function_call: None,
                name: None,
            },
        ));
    }

    prompt_messages.push(ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
        content: ChatCompletionRequestUserMessageContent::Text(format!(
            "Bitte generiere mir eine Frage auf bases deiner aktuellen Kontextinformation zum Thema '{content}'."
        )),
        name: None,
    }));

    let openai_config = llm_config.get_quiz_openai_config();
    let model = llm_config.get_quiz_model();

    let (question, tokens) = openai_single_tool_call::<QuizQuestion>(
        CallConfig::builder()
            .total_timeout(Duration::from_secs(120))
            .iteration_timeout(Duration::from_secs(30))
            .build(),
        openai_config,
        None,
        None,
        model,
        prompt_messages,
    )
    .await?;

    if let Some(usage) = tokens {
        hikari_db::llm::usage::Mutation::add_usage(conn, user_id, usage, "quiz_generation".to_owned()).await?;
    }

    match question.question {
        QuestionType::Text(text_question) => {
            let question = hikari_db::quiz::question::Mutation::create_text_question(
                conn,
                quiz_id,
                &text_question.question,
                &text_question.solution,
                &level.into_db_model(),
                session_id,
                topic,
                content,
            )
            .await?
            .into_model();

            Ok(question)
        }
        QuestionType::MultipleChoice(multiple_choice_question) => {
            let options = serde_json::to_string(&multiple_choice_question.options)?;
            let question = hikari_db::quiz::question::Mutation::create_multiple_choice_question(
                conn,
                quiz_id,
                &multiple_choice_question.question,
                &options,
                &level.into_db_model(),
                session_id,
                topic,
                content,
            )
            .await?
            .into_model();

            Ok(question)
        }
    }
}

#[derive(Serialize)]
pub struct TextQuestionTool {}

impl Default for TextQuestionTool {
    fn default() -> Self {
        Self::new()
    }
}

impl TextQuestionTool {
    #[must_use]
    pub fn new() -> TextQuestionTool {
        TextQuestionTool {}
    }
}

fn random_bloom_level(score: f64) -> QuestionBloomLevel {
    let levels = [
        (0.0, QuestionBloomLevel::Remember),
        (5.0, QuestionBloomLevel::Understand),
        (10.0, QuestionBloomLevel::Apply),
        (15.0, QuestionBloomLevel::Analyze),
        (20.0, QuestionBloomLevel::Evaluate),
        (25.0, QuestionBloomLevel::Create),
    ];

    let available_levels: Vec<QuestionBloomLevel> = levels
        .into_iter()
        .filter(|(threshold, _)| score >= *threshold)
        .map(|(_, level)| level)
        .collect();

    if available_levels.is_empty() {
        return QuestionBloomLevel::Remember;
    }

    let mut rng = rng();
    let level = *available_levels
        .choose(&mut rng)
        .expect("Available levels should not be empty");
    drop(rng);
    level
}

// This map is initialized the first time it is accessed.
static OPERATORS: std::sync::LazyLock<HashMap<QuestionBloomLevel, Operators>> = std::sync::LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert(
        QuestionBloomLevel::Remember,
        Operators {
            dos: vec![
                "Zählen Sie [...] auf",
                "Geben Sie [...] an",
                "Nennen",
                "Ordnen Sie [...] zu",
            ],
            donts: vec!["Erklären", "Wende an", "Begründen"],
        },
    );
    m.insert(
        QuestionBloomLevel::Understand,
        Operators {
            dos: vec![
                "Beschreiben",
                "Erklären",
                "Definieren",
                "Einordnen",
                "Beispiel geben",
                "Identifizieren",
            ],
            donts: vec!["Erstellen", "Bewerten"],
        },
    );
    m.insert(
        QuestionBloomLevel::Apply,
        Operators {
            dos: vec!["Wenden Sie [...] an", "Ergänzen", "Vervollständigen"],
            donts: vec!["Kritisieren", "Bewerten"],
        },
    );
    m.insert(
        QuestionBloomLevel::Analyze,
        Operators {
            dos: vec!["Vergleichen", "Unterscheiden", "Ordnen"],
            donts: vec!["Definieren", "Liste auf"],
        },
    );
    m.insert(
        QuestionBloomLevel::Evaluate,
        Operators {
            dos: vec!["Kritisieren", "Bewerten"],
            donts: vec!["Zusammenfassen", "Klassifizieren"],
        },
    );
    m.insert(
        QuestionBloomLevel::Create,
        Operators {
            dos: vec!["Erstellen", "Konstruieren"],
            donts: vec!["Ausführen", "Implementieren"],
        },
    );
    m
});

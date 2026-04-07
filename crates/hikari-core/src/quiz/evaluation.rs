use crate::llm_config::LlmConfig;
use crate::openai::tools::{OpenApiField, Tool, ToolChoice};
use crate::openai::{CallConfig, Content, OpenAiCallResult, openai_call_with_timeout};
use crate::pgvector::search;
use crate::quiz::error::QuizError;
use crate::quiz::max_five_random_exam_questions;
use async_openai::types::{
    ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessage, ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageContent,
};
use async_trait::async_trait;
use hikari_config::module::content::ContentExam;
use hikari_model::llm::vector::embedding_chunk::LlmEmbeddingQueryResult;
use hikari_model::quiz::question::Question;
use hikari_model::quiz::question::QuestionType;
use hikari_model_tools::convert::IntoModel;
use num_traits::cast::ToPrimitive;
use rand::rng;
use rand::seq::{IndexedRandom, SliceRandom};
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use std::vec;

#[allow(clippy::too_many_arguments)]
pub async fn evaluate_answer(
    user_id: &Uuid,
    module_id: &str,
    question: &Question,
    exams: &[(String, ContentExam)],
    answer: &str,
    llm_config: &LlmConfig,
    conn: &DatabaseConnection,
    session_sources: Vec<String>,
) -> Result<Question, QuizError> {
    let question_type = &question.r#type;
    let question_topic = question.topic.clone();
    let question_level = question.level;
    let question_content = question.content.clone();
    let question_session_id = question.session_id.clone();
    let question_question = question.question.clone();
    let question_solution = question.ai_solution.clone();
    let question_options = question.options.clone();

    let question_solution_for_prompt = match question_type {
        QuestionType::Text => question_solution
            .clone()
            .unwrap_or("Keine AI Lösung vorhanden.".to_string()),

        QuestionType::MultipleChoice => {
            let correct_options: Vec<String> = question_options
                .iter()
                .filter(|opt| opt.correct.unwrap_or_default())
                .map(|opt| opt.option.clone())
                .collect();

            if correct_options.is_empty() {
                "Keine korrekten Antwortmöglichkeiten vorhanden.".to_string()
            } else {
                correct_options.join(", ")
            }
        }
    };

    // Shuffle and limit to 5 questions
    let limited_exam_questions: Vec<(String, ContentExam)> =
        max_five_random_exam_questions(exams.to_owned(), question_level);

    let sources: Vec<LlmEmbeddingQueryResult> =
        search(llm_config, conn, &question_content, 5, &session_sources).await?;

    let sources_string: String = sources
        .iter()
        .enumerate()
        .map(|(i, e)| format!("# Source {}\n{}", i + 1, e.content))
        .collect::<Vec<_>>()
        .join("\n\n");

    let mut prompt_messages: Vec<ChatCompletionRequestMessage> = vec![ChatCompletionRequestMessage::System(
        ChatCompletionRequestSystemMessage {
            content: ChatCompletionRequestSystemMessageContent::Text(format!("
                        # SYSTEM ROLLE
                        Du bist ein erfahrener, wohlwollender Universitätsprofessor und Prüfer. Deine Aufgabe ist es, studentische Antworten auf Übungsfragen zu bewerten und konstruktives Feedback zu geben, um die Studierenden optimal auf die Prüfung vorzubereiten.
                        Du bist ein didaktisch versierter Hochschulprüfer und Experte für die Bewertung von Prüfungsfragen. Deine Aufgabe ist es, studentische Antworten auf Übungsfragen zu bewerten und konstruktives Feedback zu geben, um die Studierenden optimal auf die Prüfung vorzubereiten.

                        # TONALITÄT
                        * Freundlich, professionell, unterstützend und motivierend.
                        * Sprich den Studierenden direkt mit 'Du' an.

                        # BEWERTUNGSKRITERIEN & LOGIK
                        1.  **Skala:** 0 bis 5 Punkte.
                            * **5/5:** Die Antwort ist korrekt und deckt den Kern der Frage ab. Sie muss NICHT die Detailtiefe oder Länge einer KI-Antwort haben. Fehlende Quellen/Zitate führen zu keinem Punktabzug.
                            * **0/5:** Die Antwort ist faktisch falsch, verfehlt das Thema komplett oder beantwortet die Frage gar nicht.
                            * **Zwischenwerte:** Bewerte fair und nicht pedantisch. Wenn der Kern stimmt, gib die volle Punktzahl. Ziehe nur Punkte ab für signifikante inhaltliche Lücken oder Fehler.
                        2.  **Umfang:** Erwarte prägnante Antworten. Zusätzliche, nicht geforderte Details sind optional und nicht notwendig für die volle Punktzahl.

                        # KONTEXT & QUELLEN
                        Nutze ausschließlich die folgenden Informationen als 'Ground Truth' für deine Bewertung:
                        <context_sources>
                        {sources_string}
                        </context_sources>

                        # AI Lösung
                        Die von der KI generierte Lösung für die Frage lautet:
                        <ai_solution>
                        {question_solution_for_prompt}
                        </ai_solution>

                        # ANWEISUNGEN (SCHRITT-FÜR-SCHRITT)
                        1.  Analysiere die Frage und die bereitgestellten Kontextquellen.
                        2.  Vergleiche die Studentenantwort mit den Fakten aus den Quellen und der vorgenerten AI Lösung.
                        3.  Generiere die Ausgabe im unten definierten Format.

                        # AUSGABEFORMAT
                        Gib deine Antwort exakt in diesem Format zurück (keine Einleitungstexte):

                        **grade:** X (maximal 5 Punkte)

                        **evaluation:** Hier eine Bewertung in ganzen Sätzen."
            )),
            name: None,
        },
    )];

    let mut responses = Vec::new();

    // Add perfect evaluation examples
    for (exam_question_topic, question) in limited_exam_questions {
        tracing::debug!(
            "Adding good evaluation example for topic '{}': {}",
            exam_question_topic,
            question.question
        );

        let solution = if let Some(sol) = &question.solution {
            sol.clone()
        } else {
            question
                .options
                .iter()
                .filter(|opt| opt.is_correct)
                .map(|opt| opt.option.clone())
                .collect::<Vec<_>>()
                .join(", ")
        };

        let good_block = vec![
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: ChatCompletionRequestUserMessageContent::Text(format!(
                    "Bitte generiere mir eine Frage auf bases deiner aktuellen Kontextinformation zum Thema '{exam_question_topic}'."
                )),
                name: None,
            }),
            ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
                content: Some(ChatCompletionRequestAssistantMessageContent::Text(
                    question.question.clone(),
                )),
                refusal: None,
                audio: None,
                tool_calls: None,
                #[allow(deprecated)]
                function_call: None,
                name: None,
            }),
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: ChatCompletionRequestUserMessageContent::Text(format!("Meine Antwort: {solution}")),
                name: None,
            }),
            ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
                content: Some(ChatCompletionRequestAssistantMessageContent::Text(
                    random_perfect_evaluation_response().to_string(),
                )),
                refusal: None,
                audio: None,
                tool_calls: None,
                #[allow(deprecated)]
                function_call: None,
                name: None,
            }),
        ];

        responses.push(good_block);

        tracing::debug!(
            "Adding bad evaluation example for topic '{}': {}",
            exam_question_topic,
            question.question
        );
        let (answer, evaluation) = random_bad_evaluation_response();
        let block = vec![
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: ChatCompletionRequestUserMessageContent::Text(format!(
                    "Bitte generiere mir eine Frage auf bases deiner aktuellen Kontextinformation zum Thema '{exam_question_topic}'."
                )),
                name: None,
            }),
            ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
                content: Some(ChatCompletionRequestAssistantMessageContent::Text(
                    question.question.clone(),
                )),
                refusal: None,
                audio: None,
                tool_calls: None,
                #[allow(deprecated)]
                function_call: None,
                name: None,
            }),
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: ChatCompletionRequestUserMessageContent::Text(format!("Meine Antwort: {answer}")),
                name: None,
            }),
            ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
                content: Some(ChatCompletionRequestAssistantMessageContent::Text(
                    evaluation.to_string(),
                )),
                refusal: None,
                audio: None,
                tool_calls: None,
                #[allow(deprecated)]
                function_call: None,
                name: None,
            }),
        ];
        responses.push(block);
    }

    // Shuffle the response examples
    responses.shuffle(&mut rng());
    for response in responses {
        prompt_messages.extend(response);
    }

    prompt_messages.push(ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
        content: ChatCompletionRequestUserMessageContent::Text(format!(
            "Bitte generiere mir eine Frage auf bases deiner aktuellen Kontextinformation zum Thema '{question_content}'."
        )),
        name: None,
    }));

    prompt_messages.push(ChatCompletionRequestMessage::Assistant(
        ChatCompletionRequestAssistantMessage {
            content: Option::from(ChatCompletionRequestAssistantMessageContent::Text(
                question_question.clone(),
            )),
            refusal: None,
            name: None,
            audio: None,
            tool_calls: None,
            #[allow(deprecated)]
            function_call: None,
        },
    ));

    prompt_messages.push(ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
        content: ChatCompletionRequestUserMessageContent::Text(format!("Meine Antwort: {answer}")),
        name: None,
    }));

    let openai_config = llm_config.get_quiz_openai_config();
    let model = llm_config.get_quiz_model();

    let llm_response = openai_call_with_timeout(
        CallConfig::builder()
            .total_timeout(Duration::from_secs(120))
            .iteration_timeout(Duration::from_secs(30))
            .build(),
        openai_config,
        false,
        None,
        model,
        prompt_messages,
        vec![Box::new(EvaluationTool {})],
        Some(ToolChoice::Required),
    )
    .await?;

    let llm_response = match llm_response {
        OpenAiCallResult::Stream(_) => Err(QuizError::UnexpectedResponseFormat),
        OpenAiCallResult::Message(msg) => Ok(msg),
    }?;

    if let Some(usage) = llm_response.tokens {
        hikari_db::llm::usage::Mutation::add_usage(conn, user_id, usage, "quiz_generation".to_owned()).await?;
    }

    if let Content::Tool(tool_calls) = llm_response.content {
        let first = tool_calls
            .into_iter()
            .next()
            .ok_or(QuizError::UnexpectedResponseFormat)?;
        let arguments = first.arguments;

        // FIXME: These should probably be checked
        let grade = arguments
            .get("grade")
            .expect("missing grade")
            .as_f64()
            .unwrap_or(0.0)
            .to_i32()
            .unwrap_or(0);
        let evaluation = arguments
            .get("evaluation")
            .expect("missing evaluation")
            .as_str()
            .unwrap_or("")
            .to_string();

        let score_adjustment = f64::from(grade) - 2.5;

        let current_score: f64 =
            (hikari_db::quiz::score::Query::get_score_by_topic(conn, user_id, &question_session_id, &question_topic)
                .await?)
                .unwrap_or(0.0);

        let mut new_score: f64 = current_score + score_adjustment;

        // Clamp the new_score between 0.0 and 30.0
        new_score = new_score.clamp(0.0, 30.0);

        hikari_db::quiz::score::Mutation::upsert_score(
            conn,
            user_id,
            module_id,
            &question_session_id,
            &question_topic,
            &new_score,
        )
        .await?;

        let updated_question =
            hikari_db::quiz::question::Mutation::add_evaluation(conn, &question.id, answer, &evaluation, &grade)
                .await?;

        let question_model: Question = updated_question.into_model();

        Ok(question_model)
    } else {
        Err(QuizError::UnexpectedResponseFormat)
    }
}

#[derive(Serialize)]
pub struct EvaluationTool {}

#[async_trait]
impl Tool for EvaluationTool {
    fn name(&self) -> &'static str {
        "EvaluationTool"
    }

    fn description(&self) -> &'static str {
        "Dieses Tool verarbeitet und speichert die Bewertung einer Antwort auf eine Prüfungsfrage. \
        Immer verwenden, wenn eine Bewertung einer Antwort erzeugt werden soll. \
        Es erwartet eine Bewertung (grade) von 0 bis 5, eine textliche Begründung (evaluation) für die Bewertung, sowie Verbesserungsvorschläge (evaluation)."
    }

    fn parameters(&self) -> Value {
        let field = OpenApiField::object()
            .properties(HashMap::from([
                (
                    "grade",
                    OpenApiField::new("number").description("Bewertung der Antwort von 0 (schlecht) bis 5 (perfekt)"),
                ),
                (
                    "evaluation",
                    OpenApiField::new("string").description("Hier eine Bewertung in ganzen Sätzen."),
                ),
            ]))
            .required(vec!["grade", "evaluation"]);

        serde_json::to_value(field).expect("Serialization failed that should not fail")
    }
}

fn random_perfect_evaluation_response() -> &'static str {
    let mut rng = rng();
    let response = PERFECT_EVALUATION_RESPONSES
        .choose(&mut rng)
        .unwrap_or(&PERFECT_EVALUATION_RESPONSES[0]);
    drop(rng);
    response
}

const PERFECT_EVALUATION_RESPONSES: [&str; 5] = [
    r#"{
        "grade": 5,
        "evaluation": "Hervorragend, das ist eine Punktlandung. Die Antwort ist nicht nur vollkommen korrekt, sondern auch perfekt auf den Punkt gebracht. Du hast alle relevanten Aspekte genannt, ohne dich in unnötigen Details zu verlieren. Genau das war gefragt."
    }"#,
    r#"{
        "grade": 5,
        "evaluation": "Ausgezeichnet. Das war eine sehr klare und effiziente Antwort. Sie war inhaltlich richtig und hat sich genau auf die wesentlichen Punkte konzentriert. Die Detailtiefe war ideal gewählt – nicht zu oberflächlich, aber auch nicht überladen. Sehr gut strukturiert."
    }"#,
    r#"{
        "grade": 5,
        "evaluation": "Das ist eine wirklich starke Antwort. Sie ist inhaltlich absolut korrekt und zeigt ein tolles Gespür für die richtige Balance. Du hast genau das richtige Maß an Details geliefert – alles Wichtige war drin, aber du hast dich nicht in Nebensächlichkeiten verloren. Besser hätte man es kaum zusammenfassen können"
    }"#,
    r#"{
        "grade": 5,
        "evaluation": "Sehr schön. Die Antwort ist nicht nur richtig, sie ist auch analytisch hervorragend. Du hast die Kernpunkte klar identifiziert und dargelegt, ohne unnötig auszuschweifen. Das zeigt ein sehr gutes Verständnis dafür, was in dieser Frage wirklich wesentlich war"
    }"#,
    r#"{
        "grade": 5,
        "evaluation": "Perfekt. Inhaltlich exakt richtig und genau im richtigen Umfang. Du hast dich auf das Wesentliche konzentriert und alle wichtigen Punkte abgedeckt. Starke Leistung."
    }"#,
];

fn random_bad_evaluation_response() -> (&'static str, &'static str) {
    let mut rng = rng();
    let response = BAD_ANSWERES_AND_EVALUATION_RESPONSES
        .choose(&mut rng)
        .unwrap_or(&BAD_ANSWERES_AND_EVALUATION_RESPONSES[0]);
    drop(rng);
    *response
}

const BAD_ANSWERES_AND_EVALUATION_RESPONSES: [(&str, &str); 5] = [
    (
        "Ich habe keine Ahnung.",
        r#"{
            "grade": 0,
            "evaluation": "Die Antwort ist unzureichend, da sie die Frage überhaupt nicht beantwortet."
        }"#,
    ),
    (
        "Das ist schwer zu sagen.",
        r#"{
            "grade": 0,
            "evaluation": "Die Antwort liefert keine relevante Informationen. Eine präzisere Antwort wäre erforderlich."
        }"#,
    ),
    (
        "Richtige Anwort",
        r#"{
            "grade": 0,
            "evaluation": "Guter Versuch aber die Antwort muss tatsächlich die Frage beantworten. Und nicht nur sagen, dass es eine richtige Antwort ist."
        }"#,
    ),
    (
        "Ich glaube, das ist irgendwie richtig.",
        r#"{
            "grade": 0,
            "evaluation": "Deine Antwort behauptet, dass sie richtig ist, liefert aber keine wirkliche Arguementation oder Erklärung. Du musst konkret auf die Frage eingehen und sie beantworten."
        }"#,
    ),
    (
        "Ich weiß das",
        r#"{
            "grade": 0,
            "evaluation": "Es freut mich, dass du das weißt, aber deine Antwort muss tatsächlich die Frage beantworten. Nur zu sagen, dass du es weißt, reicht nicht aus."
        }"#,
    ),
];

use serde::{Deserialize, Serialize};
use crate::llm_config::LlmConfig;
use crate::openai::{CallConfig, openai_single_tool_call};
use utoipa::ToSchema;
use std::time::Duration;
use crate::quiz::error::QuizError;
use async_openai::types::chat::{
    ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessage, ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageContent,
};
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;
use tracing::instrument;

use hikari_model::quiz::goal_feedback::GoalFeedback;
use hikari_model::quiz::goal_feedback::FeedbackInput;
use crate::usage::add_usage;

#[instrument(skip(input, llm_config, conn), err)]
pub async fn generate_feedback(
    user_id: &Uuid,
    input: &FeedbackInput,
    llm_config: &LlmConfig,
    conn: &DatabaseConnection,
) -> Result<GoalFeedback, QuizError> {

    let session_ids = input.session_ids.join(", ");
    let target_mode = input.target_mode.clone();
    let level = input.level.clone();
    let deadline = input.deadline.clone();
    let lecture_targets = serde_json::to_string(&input.lecture_targets)?;
    let outcome = input.outcome.clone();
    let obstacles = input.obstacles.join(", ");
    let plans = input.plans.join(", ");


    let prompt_messages: Vec<ChatCompletionRequestMessage> = vec![
        ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
            content: ChatCompletionRequestSystemMessageContent::Text(format!("Du bist Amsl, ein experte in Goal-oriented learning. Deine expertise liegt konkret in der WOOP Methode. WOOP steht für Wish, Outcome, Obstacle und Plan und sie funktioniert wie folgt:\
            1. Wish: Hier definiert der Lernende ein Ziel, das er erreichen will. In diesem Kontext ist es eine Kombination aus Vorlesung(en), einem Level der Bloom Taxonomie, welches er in der/den Vorlesungen erreichen will und einem oder mehreren Daten, bis wann der lernende das Ziel erreichen will-\
            2. Outcome: Hier schreibt der Lernende den größtmöglichen Nutzen auf, welchen er durch das Erreichen des Ziels aus Schritt 1 erhält. Dies ist vor allem wichtig, damit sich der Nutzer motivieren kann.\
            3. Obstacle: Hier schreibt der Lernende eine oder mehrere Dinge auf, die ihn am erreichen des hindern könnten. Ein Beispiel wäre, dass der Lernende durch sein Handy abgelenkt werden könnte.\
            4. Plan: Im Plan definiert der Lernende für jedes Obstacle einen Plan oder eine Gegenmaßnahme, welche ihm beim überwinden des Obstacles hilft Ein Beispiel wäre, dass der Lernende sein Handy in einem anderen Raum lässt.\
            \
            Der Nutzer wird dir im folgenden seine Durchführung der WOOP Methode zeigen. Deine Aufgabe ist es, Feedback dazu zu geben.\
            Halte dich bitte an den Tool Call und antworte in dem spezifizierten Format.\
            Hier eine Erklärung zu deiner Antwort:\
            coherence: Antworte hier bitte nur niedrig, mittel oder hoch, abhängig davon, wie die Kohärenz der WOOP Durchführung ist.\
            summary: Erkläre hier, wie deine coherence Einschätzung zustande gekommen ist.\
            positive: Erkläre hier, was der Nutzer bei der Durchführung der WOOP Methode gut gemacht hat. Hinweis: Das Frontend, in welchem der Lernende die WOOP Methode durchführt, forciert eine korrekte WOOP Struktur. Bewerte also nur die Inhalte der einzelnen Schritte.\
            obstacles: Bewerte hier die obstacles und plans, welche der Nutzer gewählt hat und überprüfe, ob die plans die obstacles effectiv vermeiden.\
            suggestedOutcomes: Empfehle hier dem Nutzer weitere Outcomes, welche er in seine Zielsetzung übernehmen kann. Empfehle immer mindestens ein neues Outcome. Schreibe dabei nur das/ die Outcomes auf und nichts weiteres\
            sugesstedPairs: Empehle hier dem Nutzer weitere obstacle plan paare, welche er bei der Zielsetzung vielleicht nicht im Blick hatte. Antworte nach Schema nur in den Obstacles und Plans und schreibe nichts anderes auf\
            Achte darauf, dich genau and das Schema zu halten.\
            .\
            \
             ")),
            name: None,
        }),
        ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
            content: ChatCompletionRequestUserMessageContent::Text(format!("Hallo Amsl. Hier ist das Ergebnis meiner WOOP Durchführung. Kannst du mir Feedback geben?\
            Die Vorlesungen, welche ich lernen will: {session_ids}\
            Ich will für die Vorlesungen ein {target_mode} Ziel setzen:\
            {level}, {deadline}\
            {lecture_targets}\
            Darum möchte ich das Ziel erreichen: {outcome}\
            Diese Sachen werden mir vermutlich dabei im Weg stehen: {obstacles}\
            Damit will ich die Hindernisse überwinden: {plans}", session_ids = session_ids, target_mode=target_mode, level=level, deadline=deadline, lecture_targets=lecture_targets, outcome=outcome, obstacles=obstacles, plans=plans)),
            name: None,
        }),
    ];

    let openai_config = llm_config.get_quiz_openai_config();
    let model = llm_config.get_quiz_model();

    let (feedback, tokens) = openai_single_tool_call::<GoalFeedback>(
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
        add_usage(conn, user_id, usage, "goal_feedback_generation").await?;
    }

    Ok(feedback)
}
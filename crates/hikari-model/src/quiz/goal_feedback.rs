use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedOutcome {
    pub title: String,
    pub outcome: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedPair {
    pub obstacle: String,
    pub plan: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackObstacle {
    pub problem: String,
    pub plan: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Coherence {
    Hoch,
    Mittel,
    Niedrig,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, JsonSchema)]
#[schemars(description = "Strukturiertes Feedback zu einem WOOP-Lernziel.")]
#[serde(rename_all = "camelCase")]
pub struct GoalFeedback {
    pub coherence: Coherence,
    pub summary: String,
    pub positive: Vec<String>,
    pub obstacles: Vec<FeedbackObstacle>,
    pub suggested_outcomes: Vec<SuggestedOutcome>,
    pub suggested_pairs: Vec<SuggestedPair>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackInput {
    pub session_ids: Vec<String>,
    pub target_mode: String,
    pub level: String,
    pub deadline: String,
    pub outcome: String,
    pub obstacles: Vec<String>,
    pub plans: Vec<String>,
    pub lecture_targets: crate::quiz::goal::LectureTargets,
}
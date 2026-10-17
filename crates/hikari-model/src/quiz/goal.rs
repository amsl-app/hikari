use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub struct Goal {
    pub id: Uuid,
    pub module_id: String,
    pub quiz_id: Uuid,
    pub session_ids: Vec<String>,

    pub target_mode: String,
    pub level: String,
    pub deadline: chrono::NaiveDateTime,
    pub outcome: String,

    pub additional_outcomes: Vec<String>,
    pub obstacles: Vec<String>,
    pub plans: Vec<String>,

    pub active: bool,
    pub lecture_targets: LectureTargets,
}

#[derive(Debug, Deserialize, Serialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LectureTarget {
    pub bloom_level: String,
    pub deadline: String,
}

pub type LectureTargets = HashMap<String, LectureTarget>;
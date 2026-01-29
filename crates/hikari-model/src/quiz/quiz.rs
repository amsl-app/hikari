use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::quiz::question::Question;

#[derive(Deserialize, ToSchema, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum QuizStatus {
    Open,
    Closed,
}

#[derive(Deserialize, ToSchema)]
pub struct Quiz {
    pub id: Uuid,
    pub module_id: String,
    pub status: QuizStatus,
    pub created_at: chrono::NaiveDateTime,
}

impl Quiz {
    #[must_use]
    pub fn as_quiz_full<'a>(
        &'a self,
        deep: bool,
        questions: Vec<&'a Question>,
        session_ids: Vec<&'a str>,
    ) -> QuizFull<'a> {
        QuizFull {
            id: &self.id,
            module_id: &self.module_id,
            questions: if deep { questions } else { Vec::new() },
            session_ids,
            status: self.status,
            created_at: self.created_at,
        }
    }
}

#[derive(ToSchema, Serialize, Clone)]
pub struct QuizFull<'a> {
    pub id: &'a Uuid,
    pub module_id: &'a str,
    pub session_ids: Vec<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub questions: Vec<&'a Question>,
    pub status: QuizStatus,
    pub created_at: chrono::NaiveDateTime,
}

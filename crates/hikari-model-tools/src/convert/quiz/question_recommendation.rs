use hikari_model::quiz::question::{Question, QuestionOption};
use hikari_entity::quiz::question_recommendation::{Model as RecommendationModel};
use crate::convert::FromDbModel;
use hikari_model::quiz::question_recommendation::QuestionRecommendation;

impl FromDbModel<RecommendationModel> for QuestionRecommendation {
    fn from_db_model(model: RecommendationModel) -> Self {
        Self {
            id: model.id,
            quiz_id: model.quiz_id,
            question_id: model.question_id,
            recommended_at: model.recommended_at,
            used: model.used,
            used_at: model.used_at,
        }
    }
}
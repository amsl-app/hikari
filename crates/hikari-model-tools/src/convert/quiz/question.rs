use hikari_config::module::content::QuestionBloomLevel;
use hikari_entity::quiz::question::BloomLevel as QuestionBloomLevelModel;
use hikari_entity::quiz::question::Feedback as QuestionFeedbackModel;
use hikari_entity::quiz::question::Model as QuestionModel;
use hikari_entity::quiz::question::QuestionType as QuestionTypeModel;
use hikari_entity::quiz::question::Status as QuestionStatusModel;
use hikari_model::quiz::question::Question;
use hikari_model::quiz::question::QuestionFeedback;
use hikari_model::quiz::question::QuestionOption;
use hikari_model::quiz::question::QuestionStatus;

use crate::convert::FromDbModel;
use crate::convert::IntoDbModel;

impl FromDbModel<QuestionModel> for Question {
    fn from_db_model(model: QuestionModel) -> Self {
        let options: Vec<QuestionOption> = if let Some(options_json) = model.options {
            serde_json::from_str(&options_json).unwrap_or_default()
        } else {
            Vec::new()
        };

        Self {
            id: model.id,
            quiz_id: model.quiz_id,
            session_id: model.session_id,
            topic: model.topic,
            content: model.content,
            question: model.question,
            r#type: FromDbModel::from_db_model(model.r#type),
            options,
            level: FromDbModel::from_db_model(model.level),
            answer: model.answer,
            evaluation: model.evaluation,
            grade: model.grade,
            ai_solution: model.ai_solution,
            status: FromDbModel::from_db_model(model.status),
            feedback: model.feedback.map(FromDbModel::from_db_model),
            feedback_explanation: model.feedback_explanation,
            created_at: model.created_at,
            answered_at: model.answered_at,
        }
    }
}

impl FromDbModel<QuestionTypeModel> for hikari_model::quiz::question::QuestionType {
    fn from_db_model(model: QuestionTypeModel) -> Self {
        match model {
            QuestionTypeModel::MultipleChoice => hikari_model::quiz::question::QuestionType::MultipleChoice,
            QuestionTypeModel::Text => hikari_model::quiz::question::QuestionType::Text,
        }
    }
}

impl FromDbModel<QuestionFeedbackModel> for QuestionFeedback {
    fn from_db_model(model: QuestionFeedbackModel) -> Self {
        match model {
            QuestionFeedbackModel::Good => QuestionFeedback::Good,
            QuestionFeedbackModel::Bad => QuestionFeedback::Bad,
        }
    }
}

impl IntoDbModel<QuestionFeedbackModel> for QuestionFeedback {
    fn into_db_model(self) -> QuestionFeedbackModel {
        match self {
            QuestionFeedback::Good => QuestionFeedbackModel::Good,
            QuestionFeedback::Bad => QuestionFeedbackModel::Bad,
        }
    }
}

impl FromDbModel<QuestionBloomLevelModel> for QuestionBloomLevel {
    fn from_db_model(model: QuestionBloomLevelModel) -> Self {
        match model {
            QuestionBloomLevelModel::Remember => QuestionBloomLevel::Remember,
            QuestionBloomLevelModel::Understand => QuestionBloomLevel::Understand,
            QuestionBloomLevelModel::Apply => QuestionBloomLevel::Apply,
            QuestionBloomLevelModel::Analyze => QuestionBloomLevel::Analyze,
            QuestionBloomLevelModel::Evaluate => QuestionBloomLevel::Evaluate,
            QuestionBloomLevelModel::Create => QuestionBloomLevel::Create,
        }
    }
}

impl IntoDbModel<QuestionBloomLevelModel> for QuestionBloomLevel {
    fn into_db_model(self) -> QuestionBloomLevelModel {
        match self {
            QuestionBloomLevel::Remember => QuestionBloomLevelModel::Remember,
            QuestionBloomLevel::Understand => QuestionBloomLevelModel::Understand,
            QuestionBloomLevel::Apply => QuestionBloomLevelModel::Apply,
            QuestionBloomLevel::Analyze => QuestionBloomLevelModel::Analyze,
            QuestionBloomLevel::Evaluate => QuestionBloomLevelModel::Evaluate,
            QuestionBloomLevel::Create => QuestionBloomLevelModel::Create,
        }
    }
}

impl FromDbModel<QuestionStatusModel> for QuestionStatus {
    fn from_db_model(model: QuestionStatusModel) -> Self {
        match model {
            QuestionStatusModel::Open => QuestionStatus::Open,
            QuestionStatusModel::Finished => QuestionStatus::Finished,
            QuestionStatusModel::Skipped => QuestionStatus::Skipped,
        }
    }
}

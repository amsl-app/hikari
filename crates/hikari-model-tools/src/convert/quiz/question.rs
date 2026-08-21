use hikari_config::module::content::QuestionBloomLevel;
use hikari_entity::quiz::question::BloomLevel as QuestionBloomLevelModel;
use hikari_entity::quiz::question::Model as QuestionModel;
use hikari_entity::quiz::question::QuestionType as QuestionTypeModel;
use hikari_model::quiz::question::Question;
use hikari_model::quiz::question::QuestionOption;

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
            topic: model.topic,
            content: model.content,
            question: model.question,
            r#type: FromDbModel::from_db_model(model.r#type),
            options,
            level: FromDbModel::from_db_model(model.level),
            ai_solution: model.ai_solution,
            created_at: model.created_at,
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

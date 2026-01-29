use hikari_entity::quiz::quiz::Model as QuizModel;
use hikari_entity::quiz::quiz::Status as QuizStatusModel;
use hikari_model::quiz::quiz::{Quiz, QuizStatus};

use crate::convert::FromDbModel;

impl FromDbModel<QuizStatusModel> for QuizStatus {
    fn from_db_model(model: QuizStatusModel) -> Self {
        match model {
            QuizStatusModel::Open => QuizStatus::Open,
            QuizStatusModel::Closed => QuizStatus::Closed,
        }
    }
}

impl FromDbModel<QuizModel> for Quiz {
    fn from_db_model(model: QuizModel) -> Self {
        Self {
            id: model.id,
            module_id: model.module_id,
            status: QuizStatus::from_db_model(model.status),
            created_at: model.created_at,
        }
    }
}

use hikari_entity::quiz::quiz_sessions::Model as QuestionSessionModel;
use hikari_model::quiz::quiz_sessions::QuizSession;

use crate::convert::FromDbModel;

impl FromDbModel<QuestionSessionModel> for QuizSession {
    fn from_db_model(model: QuestionSessionModel) -> Self {
        Self {
            quiz_id: model.quiz_id,
            session_id: model.session_id,
        }
    }
}

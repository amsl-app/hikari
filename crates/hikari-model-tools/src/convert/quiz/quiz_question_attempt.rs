use hikari_model::quiz::question::Question;
use hikari_model::quiz::quiz_question_attempt::QuizQuestionAttempt;
use hikari_entity::quiz::quiz_question_attempt:: Model as AttemptModel;
use crate::convert::{FromDbModel, IntoDbModel};
use hikari_model::quiz::quiz_question_attempt::QuestionStatus;
use hikari_model::quiz::quiz_question_attempt::QuestionFeedback;
use hikari_entity::quiz::quiz_question_attempt::Status as QuestionStatusModel;
use hikari_entity::quiz::quiz_question_attempt::Feedback as QuestionFeedbackModel;


impl FromDbModel<AttemptModel> for QuizQuestionAttempt {
    fn from_db_model(model: AttemptModel) -> Self {
        Self {
            quiz_id: model.quiz_id,
            question_id: model.question_id,
            attempt: model.attempt,
            session_id: model.session_id,
            answer: model.answer,
            evaluation: model.evaluation,
            grade: model.grade,
            status: FromDbModel::from_db_model(model.status),
            feedback: model.feedback.map(FromDbModel::from_db_model),
            feedback_explanation: model.feedback_explanation,
            asked_at: model.asked_at,
            answered_at: model.answered_at,
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
impl FromDbModel<QuestionStatusModel> for QuestionStatus {
    fn from_db_model(model: QuestionStatusModel) -> Self {
        match model {
            QuestionStatusModel::Open => QuestionStatus::Open,
            QuestionStatusModel::Finished => QuestionStatus::Finished,
            QuestionStatusModel::Skipped => QuestionStatus::Skipped,
        }
    }
}


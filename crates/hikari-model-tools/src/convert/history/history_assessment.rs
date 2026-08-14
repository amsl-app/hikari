use crate::convert::FromDbModel;
use hikari_entity::history::history_assessment::Model;
use hikari_model::history::HistoryAssessment;

impl FromDbModel<Model> for HistoryAssessment {
    fn from_db_model(model: Model) -> Self {
        Self {
            session_id: model.assessment_session_id,
        }
    }
}

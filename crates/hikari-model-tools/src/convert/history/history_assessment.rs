use crate::convert::FromDbModel;
use hikari_entity::history::history_assessment::Model;
use hikari_model::history::HistoryAssessment;

impl FromDbModel<Model> for HistoryAssessment {
    fn from_db_model(model: Model) -> Self {
        Self {
            assessment_type: "".to_string(), // Needed for old frontend but never actually used, so we can just return an empty string for now
            session_id: model.assessment_session_id,
        }
    }
}

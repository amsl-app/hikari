use crate::convert::FromDbModel;
use hikari_entity::history::history_assessment::Model;
use hikari_model::history::HistoryAssessment;

impl FromDbModel<Model> for HistoryAssessment {
    fn from_db_model(model: Model) -> Self {
        Self {
            #[allow(deprecated)]
            assessment_type: "".to_string(), // Needed for old frontend to parse the object. But it was never actually used, so we can just return an empty string for now
            session_id: model.assessment_session_id,
        }
    }
}

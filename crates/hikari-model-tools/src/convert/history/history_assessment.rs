use crate::convert::{FromDbModel, FromModel, IntoModel};
use hikari_entity::history::history_assessment::HistoryAssessmentType as HistoryAssessmentTypeModel;
use hikari_entity::history::history_assessment::Model;
use hikari_model::history::{HistoryAssessment, HistoryAssessmentType};

impl FromDbModel<HistoryAssessmentTypeModel> for HistoryAssessmentType {
    fn from_db_model(model: HistoryAssessmentTypeModel) -> Self {
        match model {
            HistoryAssessmentTypeModel::Pre => Self::Pre,
            HistoryAssessmentTypeModel::Post => Self::Post,
        }
    }
}

impl FromModel<HistoryAssessmentType> for HistoryAssessmentTypeModel {
    fn from_model(model: HistoryAssessmentType) -> Self {
        match model {
            HistoryAssessmentType::Pre => Self::Pre,
            HistoryAssessmentType::Post => Self::Post,
        }
    }
}

impl FromDbModel<Model> for HistoryAssessment {
    fn from_db_model(model: Model) -> Self {
        Self {
            assessment_type: model.type_id.into_model(),
            session_id: model.assessment_session_id,
        }
    }
}

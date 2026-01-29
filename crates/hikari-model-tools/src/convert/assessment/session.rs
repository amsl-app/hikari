use crate::convert::FromDbModel;
use hikari_entity::assessment::session::AssessmentStatus;

impl FromDbModel<AssessmentStatus> for hikari_model::assessment::session::Status {
    fn from_db_model(model: AssessmentStatus) -> Self {
        match model {
            AssessmentStatus::NotStarted => Self::NotStarted,
            AssessmentStatus::Running => Self::Running,
            AssessmentStatus::Finished => Self::Finished,
        }
    }
}

use hikari_entity::llm::conversation_state::{Model, Status as LlmStepStateModel};
use hikari_model::llm::state::{LlmConversationState, LlmStepStatus, StateValue};

use crate::convert::{FromDbModel, FromModel};

impl FromDbModel<LlmStepStateModel> for LlmStepStatus {
    fn from_db_model(model: LlmStepStateModel) -> Self {
        match model {
            LlmStepStateModel::Running => Self::Running,
            LlmStepStateModel::Completed => Self::Completed,
            LlmStepStateModel::WaitingForInput => Self::WaitingForInput,
            LlmStepStateModel::Error => Self::Error,
            LlmStepStateModel::NotStarted => Self::NotStarted,
        }
    }
}

impl FromModel<LlmStepStatus> for LlmStepStateModel {
    fn from_model(model: LlmStepStatus) -> Self {
        match model {
            LlmStepStatus::Running => Self::Running,
            LlmStepStatus::Completed => Self::Completed,
            LlmStepStatus::WaitingForInput => Self::WaitingForInput,
            LlmStepStatus::Error => Self::Error,
            LlmStepStatus::NotStarted => Self::NotStarted,
        }
    }
}

impl FromDbModel<Model> for LlmConversationState {
    fn from_db_model(model: Model) -> Self {
        let value = model.value.map(|v| serde_json::from_str::<StateValue>(v.as_str()));
        let value = match value {
            Some(Ok(v)) => v,
            _ => StateValue::default(),
        };
        Self {
            status: LlmStepStatus::from_db_model(model.step_state),
            current_step: model.current_step,
            value,
        }
    }
}

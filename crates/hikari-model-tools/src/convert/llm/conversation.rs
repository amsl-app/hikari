use hikari_entity::llm::conversation::Model;
use hikari_entity::llm::conversation::Status as ConversationStatusModel;
use hikari_model::llm::conversation::{ConversationStatus, LlmConversation};

use crate::convert::{FromDbModel, FromModel, IntoModel};

impl FromDbModel<ConversationStatusModel> for ConversationStatus {
    fn from_db_model(model: ConversationStatusModel) -> Self {
        match model {
            ConversationStatusModel::Open => Self::Open,
            ConversationStatusModel::Closed => Self::Closed,
            ConversationStatusModel::Completed => Self::Completed,
        }
    }
}

impl FromModel<ConversationStatus> for ConversationStatusModel {
    fn from_model(model: ConversationStatus) -> Self {
        match model {
            ConversationStatus::Open => Self::Open,
            ConversationStatus::Closed => Self::Closed,
            ConversationStatus::Completed => Self::Completed,
        }
    }
}
impl FromDbModel<Model> for LlmConversation {
    fn from_db_model(model: Model) -> Self {
        Self {
            conversation_id: model.conversation_id,
            status: model.status.into_model(),
        }
    }
}

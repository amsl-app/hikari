use hikari_entity::llm::slot::conversation_slot::Model as ConversationSlotModel;
use hikari_entity::llm::slot::global_slot::Model as GlobalSlotModel;
use hikari_entity::llm::slot::module_slot::Model as ModuleSlotModel;
use hikari_entity::llm::slot::session_slot::Model as SessionSlotModel;
use hikari_model::llm::slot::Slot;
use hikari_utils::values::ValueDecoder;
use serde_yml::Value;

use crate::convert::FromDbModel;

impl FromDbModel<ConversationSlotModel> for Slot {
    fn from_db_model(model: ConversationSlotModel) -> Self {
        Self {
            name: model.slot,
            value: Value::decode(&model.value),
        }
    }
}

impl FromDbModel<GlobalSlotModel> for Slot {
    fn from_db_model(model: GlobalSlotModel) -> Self {
        Self {
            name: model.slot,
            value: Value::decode(&model.value),
        }
    }
}

impl FromDbModel<SessionSlotModel> for Slot {
    fn from_db_model(model: SessionSlotModel) -> Self {
        Self {
            name: model.slot,
            value: Value::decode(&model.value),
        }
    }
}

impl FromDbModel<ModuleSlotModel> for Slot {
    fn from_db_model(model: ModuleSlotModel) -> Self {
        Self {
            name: model.slot,
            value: Value::decode(&model.value),
        }
    }
}

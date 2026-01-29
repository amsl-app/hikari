use chrono::NaiveDateTime;
use hikari_entity::module::status::{Model as ModuleInstanceModel, Status as ModuleStatusModel};
use hikari_model::module::instance::{ModuleInstance, ModuleInstanceStatus};

use crate::convert::{FromDbModel, IntoModel};

impl FromDbModel<ModuleStatusModel> for ModuleInstanceStatus {
    fn from_db_model(model: ModuleStatusModel) -> Self {
        match model {
            ModuleStatusModel::NotStarted => Self::NotStarted,
            ModuleStatusModel::Started => Self::Started,
            ModuleStatusModel::Finished => Self::Finished,
        }
    }
}

impl FromDbModel<ModuleInstanceModel> for ModuleInstance {
    fn from_db_model(model: ModuleInstanceModel) -> Self {
        Self {
            user_id: model.user_id,
            module: model.module,
            status: model.status.into_model(),
            completion: model.completion.as_ref().map(NaiveDateTime::and_utc),
        }
    }
}

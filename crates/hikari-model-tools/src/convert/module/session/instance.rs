use chrono::NaiveDateTime;
use hikari_entity::module::session::status::{Model as SessionInstanceModel, Status as SessionStatusModel};
use hikari_model::module::session::instance::{SessionInstance, SessionInstanceStatus};

use crate::convert::{FromDbModel, FromModel, IntoModel};

impl FromDbModel<SessionStatusModel> for SessionInstanceStatus {
    fn from_db_model(model: SessionStatusModel) -> Self {
        match model {
            SessionStatusModel::NotStarted => Self::NotStarted,
            SessionStatusModel::Started => Self::Started,
            SessionStatusModel::Finished => Self::Finished,
        }
    }
}

impl FromModel<SessionInstanceStatus> for SessionStatusModel {
    fn from_model(status: SessionInstanceStatus) -> Self {
        match status {
            SessionInstanceStatus::NotStarted => Self::NotStarted,
            SessionInstanceStatus::Started => Self::Started,
            SessionInstanceStatus::Finished => Self::Finished,
        }
    }
}

impl FromDbModel<SessionInstanceModel> for SessionInstance {
    fn from_db_model(model: SessionInstanceModel) -> Self {
        Self {
            user_id: model.user_id,
            module: model.module,
            session: model.session,
            status: model.status.into_model(),
            bot_id: model.bot_id,
            last_conv_id: model.last_conv_id,
            completion: model.completion.as_ref().map(NaiveDateTime::and_utc),
        }
    }
}

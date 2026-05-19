use crate::convert::FromDbModel;
use hikari_entity::user_context_logs::Model;
use hikari_model::user_context_log::UserContextLog;

impl FromDbModel<Model> for UserContextLog {
    fn from_db_model(model: Model) -> Self {
        Self {
            user_id: model.user_id,
            created_at: model.created_at,
            r#type: model.r#type,
            data: model.data,
        }
    }
}

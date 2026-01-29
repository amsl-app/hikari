use crate::convert::FromDbModel;
use base64::Engine;
use hikari_entity::user_handle::Model as UserHandleModel;
use hikari_model::user_handle::UserHandle;

impl FromDbModel<UserHandleModel> for UserHandle {
    fn from_db_model(model: UserHandleModel) -> Self {
        Self {
            handle: base64::engine::general_purpose::STANDARD.encode(model.handle),
            user_id: model.user_id,
        }
    }
}

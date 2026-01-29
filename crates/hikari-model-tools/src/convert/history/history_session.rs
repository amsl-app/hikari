use crate::convert::FromDbModel;
use hikari_entity::history::history_session::Model;
use hikari_model::history::HistorySession;

impl FromDbModel<Model> for HistorySession {
    fn from_db_model(model: Model) -> Self {
        Self {
            module: model.module,
            session: model.session,
        }
    }
}

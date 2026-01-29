use crate::convert::FromDbModel;
use hikari_entity::history::history_module::Model;
use hikari_model::history::HistoryModule;

impl FromDbModel<Model> for HistoryModule {
    fn from_db_model(model: Model) -> Self {
        Self { module: model.module }
    }
}

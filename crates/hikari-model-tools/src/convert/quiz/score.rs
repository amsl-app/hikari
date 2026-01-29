use hikari_entity::quiz::score::Model as ScoreModel;
use hikari_model::quiz::score::Score;

use crate::convert::FromDbModel;

impl FromDbModel<ScoreModel> for Score {
    fn from_db_model(model: ScoreModel) -> Self {
        Self {
            user_id: model.user_id,
            module_id: model.module_id,
            session_id: model.session_id,
            topic: model.topic,
            score: model.score,
        }
    }
}

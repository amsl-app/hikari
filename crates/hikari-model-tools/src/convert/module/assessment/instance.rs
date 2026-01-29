use crate::convert::FromDbModel;
use hikari_entity::module::assessment::Model as ModuleAssessmentModel;
use hikari_model::module::assessment::instance::ModuleAssessmentInstance;

impl FromDbModel<ModuleAssessmentModel> for ModuleAssessmentInstance {
    fn from_db_model(model: ModuleAssessmentModel) -> Self {
        Self {
            user_id: model.user_id,
            module: model.module,
            last_pre: model.last_pre,
            last_post: model.last_post,
        }
    }
}

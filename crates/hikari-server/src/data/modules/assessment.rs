use crate::data::modules::error::ModuleError;

use hikari_config::module::{Module, ModuleConfig, assessment::ModuleAssessment};

pub(crate) fn get_module_assessment<'a, S: AsRef<str>>(
    module_config: &'a ModuleConfig,
    module_id: &str,
    permissions: &[S],
) -> Result<(&'a Module<'a>, &'a ModuleAssessment<'a>), ModuleError> {
    let module = module_config
        .get_for_group(module_id, permissions)
        .ok_or(ModuleError::ModuleNotFound)?;
    let assessment = module.assessment().ok_or(ModuleError::AssessmentNotConfigured)?;
    Ok((module, assessment))
}

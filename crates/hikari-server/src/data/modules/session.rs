use crate::data::modules::error::ModuleError;

use hikari_config::module::{Module, ModuleConfig, session::Session};

pub(crate) fn get_session<'a, S: AsRef<str>>(
    module_id: &str,
    session_id: &str,
    module_config: &'a ModuleConfig,
    permissions: &[S],
) -> Result<(&'a Module<'a>, &'a Session), ModuleError> {
    let module = module_config
        .get_for_group(module_id, permissions)
        .ok_or(ModuleError::ModuleNotFound)?;
    let session = module.get(session_id).ok_or(ModuleError::SessionNotFound)?;
    Ok((module, session))
}

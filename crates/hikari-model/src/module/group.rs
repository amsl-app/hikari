use std::collections::HashSet;

use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, Clone, Debug, ToSchema)]
pub struct ModuleGroupeFull<'a> {
    pub key: &'a str,
    pub label: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<usize>,
    pub modules: HashSet<&'a str>,
}

impl<'a> ModuleGroupeFull<'a> {
    #[must_use]
    pub fn from_config(
        group: &'a hikari_config::global::modules::ModuleGroup,
        module_config: &'a hikari_config::module::ModuleConfig,
    ) -> Self {
        let modules = module_config
            .modules()
            .values()
            .filter(|module| module.module_groups.contains(&group.key))
            .map(|module| module.id.as_str())
            .collect();

        Self {
            key: &group.key,
            label: &group.label,
            weight: group.weight,
            modules,
        }
    }
}

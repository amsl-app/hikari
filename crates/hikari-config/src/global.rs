use crate::global::{
    access::AccessConfig, frontend::FrontendConfig, journal::JournalConfig, modules::ModuleConfig,
    onboarding::OnboardingConfig, user::UserConfig, v01::config::GlobalConfigV01,
};
use hikari_utils::loader::{Loader, LoaderTrait, error::LoadingError};
use schemars::JsonSchema;
use serde_derive::Deserialize;
use std::fmt::Debug;

pub mod access;
pub mod frontend;
pub mod journal;
pub mod modules;
pub mod onboarding;
pub mod user;
pub mod v01;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[serde(tag = "version")]
pub enum VersionConfig {
    #[serde(rename = "0.1")]
    V01 { hikari: v01::config::GlobalConfigV01 },
}

#[derive(Deserialize, Clone, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApprovalConfigEntry {
    pub id: String,
    pub version: String,
}

#[derive(Clone, Debug, Default)]
pub struct GlobalConfig {
    pub onboarding: OnboardingConfig,
    pub frontend: FrontendConfig,
    pub module: ModuleConfig,
    pub approvals: Vec<ApprovalConfigEntry>,
    pub user: UserConfig,
    pub journal: JournalConfig,
    pub access: Vec<AccessConfig>,
}

impl From<GlobalConfigV01> for GlobalConfig {
    fn from(value: GlobalConfigV01) -> Self {
        Self {
            onboarding: value.onboarding,
            frontend: value.frontend.into(),
            module: value.module,
            approvals: value.approvals,
            user: value.config,
            journal: value.journal,
            access: value.access,
        }
    }
}

impl GlobalConfig {
    #[must_use]
    pub fn onboarding(&self) -> &OnboardingConfig {
        &self.onboarding
    }
    #[must_use]
    pub fn frontend(&self) -> &FrontendConfig {
        &self.frontend
    }
    #[must_use]
    pub fn approvals(&self) -> &Vec<ApprovalConfigEntry> {
        &self.approvals
    }

    #[must_use]
    pub fn config(&self) -> &UserConfig {
        &self.user
    }

    #[must_use]
    pub fn journal(&self) -> &JournalConfig {
        &self.journal
    }

    #[must_use]
    pub fn module(&self) -> &ModuleConfig {
        &self.module
    }

    #[must_use]
    pub fn access(&self) -> &Vec<AccessConfig> {
        &self.access
    }
}

pub async fn load(loader: Loader) -> Result<GlobalConfig, LoadingError> {
    tracing::debug!("Loading config");
    let file = loader.load_file("").await?;
    let VersionConfig::V01 { hikari } = serde_yml::from_slice::<VersionConfig>(&file.content)?;

    Ok(hikari.into())
}

#[cfg(test)]
mod tests {
    use crate::global::modules::ModuleGroup;
    use std::fs::read_to_string;

    use super::*;

    #[test]
    fn test_config_loading() {
        let global_config_file = read_to_string("test_configs/test.global.yaml").unwrap();
        let VersionConfig::V01 { hikari: config_v01 } =
            serde_yml::from_str::<VersionConfig>(&global_config_file).unwrap();
        let config: GlobalConfig = config_v01.into();

        assert_eq!(config.onboarding.module, Some("onboarding".to_string()));
        assert_eq!(config.frontend.frontend.min, "0.0.0");
        assert!(!config.module.groups.is_empty());
        assert!(!config.approvals.is_empty());
        assert!(!config.user.allowed_keys.is_empty());
        assert!(!config.journal.focus.is_empty());
    }

    #[test]
    fn test_module_group_deserialization() {
        let module_group: ModuleGroup = serde_json::from_str(
            r#"{
                "key": "test",
                "label": "Test"
            }"#,
        )
        .unwrap();
        assert_eq!(module_group.key, "test");
        assert_eq!(module_group.label, "Test");
    }
}

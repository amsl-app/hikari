use schemars::JsonSchema;
use serde::Deserialize;

use crate::global::{
    ApprovalConfigEntry,
    v01::{
        access::AccessConfigV01, frontend::FrontendConfigV01, journal::JournalConfigV01, modules::ModuleConfigV01,
        onboarding::OnboardingConfigV01, user::UserConfigV01,
    },
};

#[derive(Deserialize, Clone, Debug, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct GlobalConfigV01 {
    /// # Configuration for the onboarding process
    pub(crate) onboarding: OnboardingConfigV01,
    /// # Configuration for frontend
    pub(crate) frontend: FrontendConfigV01,
    #[serde(default)]
    /// # Global configuration for the modules
    /// Defines which `module_groups` axists. Module groups are other groups than permission groups
    /// Module groups are used to group modules for display purposes in the frontend
    pub(crate) module: ModuleConfigV01,
    /// # Approval configurations
    pub(crate) approvals: Vec<ApprovalConfigEntry>,
    #[serde(default)]
    /// # User configuration
    pub(crate) config: UserConfigV01,
    #[serde(default)]
    /// # Journal configuration
    pub(crate) journal: JournalConfigV01,
    #[serde(default)]
    /// # Access configurations
    /// Defines which tokens are associated with which groups
    /// Tokens can be used to add groups to the user; groups define permissions for modules
    pub(crate) access: Vec<AccessConfigV01>,
}

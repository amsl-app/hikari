use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Default, Deserialize, Clone, Debug, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct OnboardingConfig {
    /// # Module which is shown as the first module during onboarding
    pub module: Option<String>,
}

impl OnboardingConfig {
    #[must_use]
    pub fn module(&self) -> Option<&str> {
        self.module.as_deref()
    }
}

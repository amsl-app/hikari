use std::borrow::Cow;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ModuleAssessment<'a> {
    /// # ID of an assessment which is mandatory before starting the module
    /// Assessments are defined in separate *.assessment.yaml files
    pub pre: Cow<'a, str>,
    /// # ID of an assessment which is mandatory to finishing the module
    /// Assessments are defined in separate *.assessment.yaml files
    pub post: Cow<'a, str>,
}

impl<'a> ModuleAssessment<'a> {
    #[must_use]
    pub fn borrowed(&'a self) -> ModuleAssessment<'a> {
        ModuleAssessment {
            pre: Cow::Borrowed(&self.pre),
            post: Cow::Borrowed(&self.post),
        }
    }
}

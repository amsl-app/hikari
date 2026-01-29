use crate::assessment::v01::{question::QuestionV01, scale::ScaleV01};
use hikari_utils::id_map::id_map;
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct AssessmentV01 {
    /// # Unique identifier for the assessment
    /// This ID is used to reference the assessment within the system.
    pub id: String,
    /// # Title of the assessment
    /// A human-readable title for the assessment.
    pub title: String,
    #[serde(default)]
    #[serde(with = "id_map")]
    #[schemars(with = "Vec::<QuestionV01>")]
    /// # Questions included in the assessment
    pub questions: IndexMap<String, QuestionV01>,
    #[serde(default)]
    #[serde(with = "id_map")]
    #[schemars(with = "Vec::<ScaleV01>")]
    /// # Scales used in the assessment
    /// Scales are used to generate scores based on user responses.
    pub scales: IndexMap<String, ScaleV01>,
}

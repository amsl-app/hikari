use crate::assessment::{question::Question, scale::Scale, v01::assessment::AssessmentV01};
use futures::StreamExt;
use hikari_utils::id_map::id_map;
use hikari_utils::loader::{Filter, Loader, LoaderTrait, error::LoadingError};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::Debug;
use utoipa::ToSchema;

pub mod error;
pub mod question;
pub mod scale;
pub mod v01;

#[derive(Deserialize, Debug, JsonSchema)]
#[serde(tag = "version")]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum VersionConfig {
    #[serde(rename = "0.1")]
    V01 { assessment: AssessmentV01 },
}

#[derive(Serialize, ToSchema, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Assessment {
    pub assessment_id: String,
    pub title: String,
    #[serde(default)]
    #[serde(with = "id_map")]
    pub questions: IndexMap<String, Question>,
    #[serde(default)]
    #[serde(with = "id_map")]
    pub scales: IndexMap<String, Scale>,
}

impl From<AssessmentV01> for Assessment {
    fn from(v01: AssessmentV01) -> Self {
        Self {
            assessment_id: v01.id,
            title: v01.title,
            questions: v01.questions,
            scales: v01.scales,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AssessmentConfig {
    pub assessments: IndexMap<String, Assessment>,
}

impl AssessmentConfig {
    #[must_use]
    pub fn get(&self, assessment_id: &str) -> Option<&Assessment> {
        self.assessments.get(assessment_id)
    }

    #[must_use]
    pub fn assessments(&self) -> &IndexMap<String, Assessment> {
        &self.assessments
    }

    #[must_use]
    pub fn ids(&self) -> HashSet<&String> {
        self.assessments.keys().collect()
    }
}

pub async fn load(loader: Loader) -> Result<AssessmentConfig, LoadingError> {
    tracing::debug!("Loading assessments");
    let mut res = IndexMap::new();
    let mut stream = loader.load_dir("", Filter::Yaml);
    while let Some(Ok(file)) = stream.next().await {
        let VersionConfig::V01 { assessment } = serde_yml::from_slice::<VersionConfig>(&file.content)?;
        let assessment: Assessment = assessment.into();
        res.insert(assessment.assessment_id.clone(), assessment);
    }
    tracing::debug!(?res, "loaded assessment configuration");
    Ok(AssessmentConfig { assessments: res })
}

#[cfg(test)]
mod tests {
    use std::fs::read_to_string;

    use super::*;

    #[test]
    fn test_assessment_loading() {
        let assessment_file = read_to_string("test_configs/test.assessment.yaml").unwrap();
        let VersionConfig::V01 { assessment } = serde_yml::from_str::<VersionConfig>(&assessment_file).unwrap();
        let assessment: Assessment = assessment.into();
        assert_eq!(assessment.questions.len(), 1);
    }
}

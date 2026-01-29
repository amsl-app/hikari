use std::collections::HashMap;

use crate::{
    builder::{
        error::LlmBuildingError,
        slot::{SaveTarget, paths::SlotPath},
        steps::Documents,
    },
    execution::steps::{LlmStep, vector_db_extractor::VectorDBExtractor},
};

use super::{Condition, IntoLlmStep, ParentStep};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_yml::Value;

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RetrieverBuilder {
    /// # A Slot path containing the query to retrieve documents for
    /// Can be a list of strings for multiple queries
    pub query: SlotPath,
    pub target: SaveTarget,
    /// # Limit of documents to retrieve
    /// Default: 4
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// # Whether to use secondary document stores
    /// Default: true
    /// Can be usefull to deactivate if too many documents are referenced in the session
    #[serde(default = "default_secondary")]
    pub secondary: bool,
}

fn default_limit() -> u32 {
    4
}

fn default_secondary() -> bool {
    true
}

impl IntoLlmStep for RetrieverBuilder {
    fn into_llm_step(
        self,
        parent_steps: Vec<ParentStep>,
        mut conditions: Vec<Condition>,
        id: String,
        _constants: HashMap<String, Value>,
        documents: Documents,
    ) -> Result<LlmStep, LlmBuildingError> {
        for step in parent_steps {
            conditions.extend(step.conditions);
        }

        let secondary_documents = if self.secondary { documents.secondary } else { vec![] };

        Ok(LlmStep::VectorDBExtractor(VectorDBExtractor::new(
            id,
            self.target,
            documents.primary,
            secondary_documents,
            self.limit,
            self.query,
            conditions,
        )))
    }
}

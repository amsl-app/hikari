use crate::builder::error::LlmBuildingError;
use crate::builder::slot::LoadToSlot;
use crate::builder::steps::llm::MemorySelector;
use crate::builder::steps::{Documents, Flow, Next, ParentStep, StepBuilder};
use crate::execution::steps::LlmStep;
use futures_util::StreamExt;
use hikari_utils::loader::{Filter, Loader, LoaderTrait, error::LoadingError};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_yml::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod slot;
pub mod steps;

pub mod error;
pub mod tools;

#[derive(Default, Debug)]
pub struct LlmStructureConfig {
    pub structures: IndexMap<String, LlmStructureBuilder>,
}

impl LlmStructureConfig {
    #[must_use]
    pub fn ids(&self) -> HashSet<&String> {
        self.structures.keys().collect()
    }
}

#[derive(Deserialize, Debug, JsonSchema)]
#[serde(tag = "version")]
#[serde(deny_unknown_fields)]
pub enum VersionConfig {
    #[serde(rename = "0.1")]
    V01 { structure: LlmStructureBuilder },
}

pub async fn load(loader: Loader) -> Result<LlmStructureConfig, LoadingError> {
    tracing::debug!("Loading llm structures");
    let mut res = IndexMap::new();
    let mut stream = loader.load_dir("", Filter::Yaml);
    while let Some(Ok(file)) = stream.next().await {
        let config = serde_yml::from_slice::<VersionConfig>(&file.content);
        match config {
            Ok(VersionConfig::V01 { structure }) => {
                if res.contains_key(&structure.id) {
                    tracing::warn!(
                        "Duplicate llm structure id found: {}. Overwriting previous definition.",
                        structure.id
                    );
                }
                res.insert(structure.id.clone(), structure);
            }
            Err(error) => {
                return Err(LoadingError::from(error));
            }
        }
    }
    Ok(LlmStructureConfig { structures: res })
}

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct LlmStructureBuilder {
    /// # Unique identifier for the LLM structure
    /// LLM structure corrispond to the `llm_agent` in sessions
    pub id: String,
    /// # The root action step of the LLM structure
    /// Often a chain to execute multiple steps
    pub action: StepBuilder,
    #[serde(default)]
    /// # Slots to load data into before executing
    pub slots: Vec<LoadToSlot>,
    #[serde(default)]
    #[schemars(with = "HashMap::<String, serde_json::Value>")]
    /// # Constants available to all steps in the structure
    pub constants: HashMap<String, Value>,
    #[serde(skip, default)]
    pub documents: Documents,
}

impl LlmStructureBuilder {
    pub(crate) fn build(self) -> Result<IndexMap<String, Arc<Mutex<LlmStep>>>, LlmBuildingError> {
        self.action.into_llm_step(Vec::new(), self.constants, self.documents)
    }

    pub fn with_constants(&mut self, constants: &HashMap<String, Value>, overwrite: bool) {
        for (key, value) in constants {
            if overwrite {
                self.constants.insert(key.to_owned(), value.to_owned());
            } else {
                self.constants.entry(key.to_owned()).or_insert(value.to_owned());
            }
        }
    }

    pub fn with_documents(&mut self, documents: Documents, overwrite: bool) {
        if overwrite {
            self.documents = documents;
        } else {
            self.documents.extend(documents);
        }
    }
}

#[derive(Deserialize, Debug, Clone, Serialize, Eq, PartialEq, Hash, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Selection<T> {
    Selector(T),
    From(String),
}
impl<T> Default for Selection<T>
where
    T: Default,
{
    fn default() -> Self {
        Selection::Selector(T::default())
    }
}
#[must_use]
pub fn build_memory_filter(memory_selection: &Vec<Selection<MemorySelector>>, own_id: &str) -> Option<Vec<String>> {
    let mut memory_filter = Vec::new();

    for selector in memory_selection {
        match selector {
            // If we want all, we return None for no filtering
            Selection::Selector(MemorySelector::All) => return None,
            Selection::From(id) => memory_filter.push(id.to_owned()),
            Selection::Selector(MemorySelector::Current) => memory_filter.push(own_id.to_owned()),
        }
    }
    Some(memory_filter)
}

#[must_use]
pub fn step_id_from_flow(flow: Flow, parent_steps: &[ParentStep]) -> Option<String> {
    tracing::trace!(?parent_steps, ?flow, "step_id_from_flow");
    match flow {
        Flow::Action(Next::Continue) => None,
        Flow::Action(Next::Repeat) => {
            if let [.., parent] = parent_steps {
                parent.steps.first().cloned()
            } else {
                None
            }
        }
        Flow::Goto(id) => Some(id),
    }
}

#[must_use]
pub fn default_temperature() -> f32 {
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::read_to_string;

    #[test]
    fn test_structure_loading() {
        let structure_file = read_to_string("test_configs/test.agemt.yaml").unwrap();
        let VersionConfig::V01 { structure } = serde_yml::from_str::<VersionConfig>(&structure_file).unwrap();
        assert_eq!(structure.slots.len(), 0);
        assert_eq!(structure.constants.len(), 1);
    }
}

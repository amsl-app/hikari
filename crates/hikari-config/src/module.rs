use std::collections::{HashMap, HashSet};

use crate::documents::collection::DocumentCollection;
use crate::generic::{Metadata, Theme};
use crate::module::assessment::ModuleAssessment;
use crate::module::content::Content;
use crate::module::error::ModuleError;
use crate::module::llm_agent::{LlmAgent, LlmService};
use crate::module::session::Session;
use futures::StreamExt;
use hikari_utils::loader::{Filter, Loader, LoaderTrait};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::Serialize;
use serde_derive::Deserialize;
use utoipa::ToSchema;

pub mod assessment;
pub mod content;
pub mod error;
pub mod llm_agent;
pub mod session;
pub mod unlock;
mod v01;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[serde(tag = "version")]
pub enum VersionConfig {
    #[serde(rename = "0.1")]
    V01 { module: v01::module::ModuleV01 },
}

#[derive(Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ModuleCategory {
    Onboarding,
    #[default]
    Learning,
    Course,
    Journal,
}

#[derive(Debug, Clone, Serialize)]
pub struct Module<'a> {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub banner: Option<String>,
    pub default_session: Option<String>,
    pub hidden: bool,
    pub sessions: IndexMap<String, Session>,
    pub theme: Option<Theme>,
    pub weight: Option<usize>,
    pub assessment: Option<ModuleAssessment<'a>>,
    pub category: ModuleCategory,
    pub module_groups: HashSet<String>,
    pub metadata: Option<Metadata>,
    pub groups_whitelist: Vec<String>,
    pub groups_blacklist: Vec<String>,
    pub custom: Option<HashMap<String, serde_yml::Value>>,
    pub self_learning: bool,
    pub quizzable: bool,
}

impl Module<'_> {
    pub(crate) fn from_v01(
        module: v01::module::ModuleV01,
        llm_rag_documents: &DocumentCollection,
    ) -> Result<Self, ModuleError> {
        let contents: Result<Vec<Content>, ModuleError> = module
            .contents
            .iter()
            .map(|content| Content::from_v01(content.to_owned(), llm_rag_documents))
            .collect();

        let contents = contents?;

        let self_learning_sessions: Option<Session> = build_self_learning(&module, &contents);

        let mut sessions = module
            .sessions
            .into_iter()
            .map(|session| Session::from_v01(session, contents.as_ref()).map(|s| (s.id.clone(), s)))
            .collect::<Result<IndexMap<_, _>, _>>()?;

        if let Some(self_learning) = self_learning_sessions {
            tracing::debug!("Adding self-learning session to module {}", module.id);
            sessions.insert(self_learning.id.clone(), self_learning);
        } else {
            tracing::debug!("No self-learning session for module {}", module.id);
        }

        Ok(Self {
            id: module.id,
            title: module.title,
            subtitle: module.subtitle,
            description: module.description,
            icon: module.icon,
            banner: module.banner,
            default_session: module.default_session,
            sessions,
            hidden: module.hidden,
            theme: module.theme,
            assessment: module.assessment,
            category: module.category,
            module_groups: module.module_groups,
            weight: module.weight,
            metadata: module.metadata,
            groups_whitelist: module.groups_whitelist,
            groups_blacklist: module.groups_blacklist,
            self_learning: module.self_learning.enabled,
            quizzable: module.quizzable,
            custom: module.custom,
        })
    }

    pub fn has_access<S: AsRef<str>>(&self, user_groups: &[S]) -> bool {
        let user_groups: Vec<String> = user_groups.iter().map(|g| g.as_ref().to_string()).collect();

        if user_groups.iter().any(|group| self.groups_blacklist.contains(group)) {
            false
        } else {
            self.groups_whitelist.is_empty() || self.groups_whitelist.iter().all(|group| user_groups.contains(group))
        }
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }

    #[must_use]
    pub fn assessment(&self) -> Option<&ModuleAssessment<'_>> {
        self.assessment.as_ref()
    }

    pub fn validate(
        &self,
        assessments: &HashSet<&String>,
        bots: &HashMap<&String, Vec<&String>>,
        llm_agents: &HashSet<&String>,
        module_groups: &HashSet<&String>,
    ) -> Result<(), ModuleError> {
        if let Some(assessment) = &self.assessment {
            let pre = assessment.pre.as_ref().to_owned();
            let post = assessment.post.as_ref().to_owned();

            if !assessments.contains(&pre) {
                tracing::error!(assessment_id = self.id, kind = %pre, "can't find assessment for module");
                return Err(ModuleError::AssessmentNotFound);
            }
            if !assessments.contains(&post) {
                tracing::error!(assessment_id = self.id, kind = %post, "can't find assessment for module");
                return Err(ModuleError::AssessmentNotFound);
            }
        }
        if !self.module_groups.iter().all(|group| module_groups.contains(&group)) {
            tracing::error!(module_id = self.id, module_groups = ?self.module_groups, "module group not found");
            return Err(ModuleError::ModuleGroupNotFound);
        }
        self.sessions.values().try_for_each(|s| s.validate(bots, llm_agents))
    }
}

fn build_self_learning(module: &v01::module::ModuleV01, contents: &[Content]) -> Option<Session> {
    if module.self_learning.enabled {
        let llm_agent = module.self_learning.llm_agent.clone().unwrap_or(LlmAgent {
            llm_agent: "self-learning".to_string(),
            provider: LlmService::OpenAI,
        });

        Some(Session {
            id: "self-learning".to_string(),
            title: "Self Learning".to_string(),
            subtitle: Some("Learn the contents by yourself.".to_string()),
            description: Some("Enable self learning for this module".to_string()),
            icon: None,
            banner: None,
            bot: None,
            next_session: None,
            theme: module.self_learning.theme.clone(),
            time: None,
            unlock: module.self_learning.unlock.clone(),
            metadata: None,
            contents: contents.to_owned(),
            llm_agent: Some(llm_agent),
            custom: None,
            hidden: true,
            quizzable: false,
        })
    } else {
        None
    }
}

#[derive(Debug, Clone)]
pub struct ModuleConfig {
    modules: IndexMap<String, Module<'static>>,
}

impl ModuleConfig {
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Module<'_>> {
        self.modules.get(id)
    }

    pub fn get_for_group<S: AsRef<str>>(&self, id: &str, user_groups: &[S]) -> Option<&Module<'_>> {
        // Filter all blacklisted groups and then all groups with empty whitelist or groups that contain the user group
        self.modules.get(id).and_then(|module| {
            if module.has_access(user_groups) {
                Some(module)
            } else {
                None
            }
        })
    }

    #[must_use]
    pub fn modules(&self) -> &IndexMap<String, Module<'_>> {
        &self.modules
    }

    #[must_use]
    pub fn modules_filtered(&self, user_groups: &[String]) -> Vec<&Module<'_>> {
        self.modules
            .values()
            .filter(|module| module.has_access(user_groups))
            .collect()
    }

    pub fn validate(
        &self,
        assessments: &HashSet<&String>,
        bots: &HashMap<&String, Vec<&String>>,
        llm_agents: &HashSet<&String>,
        module_groups: &HashSet<&String>,
    ) -> Result<(), ModuleError> {
        for (_, module) in self.modules() {
            module.validate(assessments, bots, llm_agents, module_groups)?;
        }
        Ok(())
    }
}

impl From<Vec<Module<'static>>> for ModuleConfig {
    fn from(modules: Vec<Module<'static>>) -> Self {
        Self {
            modules: modules.into_iter().map(|module| (module.id.clone(), module)).collect(),
        }
    }
}

pub async fn load_config(loader: Loader, llm_rag_documents: &DocumentCollection) -> Result<ModuleConfig, ModuleError> {
    tracing::debug!("Loading modules");
    let mut res = vec![];
    let mut stream = loader.load_dir("", Filter::Yaml);
    while let Some(Ok(file)) = stream.next().await {
        res.push(load(&file.content, llm_rag_documents)?);
    }
    Ok(res.into())
}

fn load(content: &[u8], llm_rag_documents: &DocumentCollection) -> Result<Module<'static>, ModuleError> {
    let VersionConfig::V01 { module } = serde_yml::from_slice::<VersionConfig>(content)?;

    let module = Module::from_v01(module, llm_rag_documents)?;
    tracing::debug!("Loaded Module is {:?}", module);
    Ok(module)
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::module::assessment::ModuleAssessment;
    use std::fs::read_to_string;

    #[test]
    fn test_module_loading() {
        let module_file = read_to_string("test_configs/test.module.yaml").unwrap();
        let VersionConfig::V01 { module } = serde_yml::from_str::<VersionConfig>(&module_file).unwrap();
        assert_eq!(module.id, "test");
        let ModuleAssessment { pre, post } = module.assessment.clone().unwrap();
        assert_eq!(pre, "pre-id");
        assert_eq!(post, "post-id");
        let rag = DocumentCollection {
            documents: HashMap::new(),
        };

        Module::from_v01(module, &rag).unwrap();
        assert_eq!(pre, "pre-id");
        assert_eq!(post, "post-id");
    }

    #[test]
    fn text_generate_json_schema() {
        let _schema = serde_json::to_string_pretty(&schemars::schema_for!(VersionConfig)).unwrap();
        println!("{}", _schema);
    }
}

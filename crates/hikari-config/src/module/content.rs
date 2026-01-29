use std::fmt::Display;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    documents::collection::DocumentCollection,
    module::{
        error::ModuleError,
        unlock::Unlock,
        v01::content::{ContentSourcesV01, ContentV01},
    },
};

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Content {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub unlock: Option<Unlock>,
    pub contents: Vec<String>,
    pub goal: Option<String>,
    pub sources: ContentSources,
    pub exams: Vec<ContentExam>,
}

impl Content {
    pub(crate) fn from_v01(content: ContentV01, llm_rag_documents: &DocumentCollection) -> Result<Self, ModuleError> {
        let sources = ContentSources::from_v01(content.sources, llm_rag_documents)?;

        Ok(Self {
            id: content.id,
            title: content.title,
            unlock: content.unlock,
            contents: content.contents,
            goal: content.goal,
            sources,
            exams: content.exams,
        })
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ContentSources {
    pub primary: Vec<ContentSource>,
    pub secondary: Vec<ContentSource>,
}

impl ContentSources {
    fn from_v01(sources: ContentSourcesV01, llm_rag_documents: &DocumentCollection) -> Result<Self, ModuleError> {
        let docs = &llm_rag_documents.documents;

        let primary: Result<Vec<ContentSource>, ModuleError> = sources
            .primary
            .into_iter()
            .map(|s| {
                let doc = docs.get(&s).ok_or(ModuleError::SourceNotFound(s.clone()))?;
                Ok(ContentSource {
                    file_id: s.clone(),
                    file_name: doc.metadata.name.clone(),
                })
            })
            .collect();

        let primary = primary?;

        let secondary: Result<Vec<ContentSource>, ModuleError> = sources
            .secondary
            .into_iter()
            .map(|s| {
                let doc = docs.get(&s).ok_or(ModuleError::SourceNotFound(s.clone()))?;
                Ok(ContentSource {
                    file_id: s.clone(),
                    file_name: doc.metadata.name.clone(),
                })
            })
            .collect();

        let secondary = secondary?;

        Ok(Self { primary, secondary })
    }

    #[must_use]
    pub fn primary(&self) -> &Vec<ContentSource> {
        &self.primary
    }

    #[must_use]
    pub fn secondary(&self) -> &Vec<ContentSource> {
        &self.secondary
    }
}

#[derive(Serialize, Debug, ToSchema, Clone, PartialEq, Eq, Hash)]
pub struct ContentSource {
    pub file_id: String,
    pub file_name: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema, Clone, Copy, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QuestionBloomLevel {
    Remember,
    Understand,
    Apply,
    Analyze,
    Evaluate,
    Create,
}

impl Display for QuestionBloomLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level_str = match self {
            QuestionBloomLevel::Remember => "remember",
            QuestionBloomLevel::Understand => "understand",
            QuestionBloomLevel::Apply => "apply",
            QuestionBloomLevel::Analyze => "analyze",
            QuestionBloomLevel::Evaluate => "evaluate",
            QuestionBloomLevel::Create => "create",
        };
        write!(f, "{level_str}")
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, JsonSchema)]
pub struct ContentExamOption {
    pub option: String,
    pub is_correct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ContentExam {
    /// # Bloom's taxonomy level of the question
    pub level: QuestionBloomLevel,
    /// # The question text
    pub question: String,
    #[serde(default)]
    /// # Solution or explanation for the question
    pub solution: Option<String>,
    #[serde(default)]
    /// # Options for multiple-choice questions
    /// Only if the question is multiple choice
    pub options: Vec<ContentExamOption>,
}

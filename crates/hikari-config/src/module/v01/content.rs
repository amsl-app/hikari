use hikari_utils::id_map::ItemId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::module::{content::ContentExam, v01::unlock::UnlockV01};

pub(crate) type ContentExamV01 = ContentExam;

#[derive(Deserialize, JsonSchema, Serialize, Clone)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct ContentV01 {
    /// # Unique identifier of the content
    pub(crate) id: String,
    /// # Title of the content
    pub(crate) title: String,
    #[serde(default)]
    /// # Mechanism to unlock the content after certain conditions are met
    pub(crate) unlock: Option<UnlockV01>,
    /// # List of content descriptions which should be covered in sessions with this content assigned
    pub(crate) contents: Vec<String>,
    #[serde(default)]
    /// # A goal describing what the user should achieve with this content
    pub(crate) goal: Option<String>,
    #[serde(default)]
    /// # Sources for the content
    /// Used for RAG purposes and to show references to the user
    pub(crate) sources: ContentSourcesV01,
    #[serde(default)]
    /// # Exams associated with the content
    /// Exams are used to prompt the quiz tool to generate quizzes about this contents
    /// Exams are not shown to the user directly
    pub(crate) exams: Vec<ContentExamV01>,
}

impl ItemId for ContentV01 {
    type IdType = String;

    fn id(&self) -> Self::IdType {
        self.id.clone()
    }
}

#[derive(Deserialize, Default, Clone, JsonSchema, Serialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct ContentSourcesV01 {
    #[serde(default)]
    /// # Primary sources for the content
    /// IDs to the documents defined in .*collection.yaml files
    /// Primary sources are the main references for the content and shown to the user
    /// Mostly lecture slides ore papers
    pub(crate) primary: Vec<String>,
    #[serde(default)]
    /// # Secondary sources for the content
    /// IDs to the documents defined in .*collection.yaml files
    /// Secondary sources are additional references for the content and not shown to the user
    /// Mostly further reading material like books
    pub(crate) secondary: Vec<String>,
}

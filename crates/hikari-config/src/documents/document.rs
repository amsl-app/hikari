use hikari_utils::loader::file::FileMetadata;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Copy, Default, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum DocumentType {
    #[default]
    Slides,
    Book,
    Paper,
    Text,
}

#[derive(Debug, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct DocumentConfig {
    /// # The unique identifier for the document.
    /// The ID must be unique across all documents and will be used to reference the document in other configurations.
    pub id: String,
    /// # The path to the document file.
    pub file: String,
    #[serde(default)]
    /// # The type of the document.
    /// The type is used for chunking strategy.
    pub r#type: DocumentType,
    /// # Metadata for the document.
    pub metadata: DocumentMetadata,
    #[serde(default)]
    /// # Pages to exclude from processing.
    pub exclude: Vec<usize>, // Pages to exclude
    #[serde(default, skip_serializing, skip_deserializing)]
    pub file_metadata: Option<FileMetadata>,
}

impl DocumentConfig {
    pub fn set_file_metadata(&mut self, data: FileMetadata) {
        self.file_metadata = Some(data);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct DocumentMetadata {
    /// # A link associated with the document.
    pub link: String,
    /// # The name of the document.
    /// The name is used for display purposes.
    pub name: String,
}

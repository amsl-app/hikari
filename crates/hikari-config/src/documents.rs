use crate::documents::collection::DocumentCollection;
use futures::StreamExt;
use hikari_utils::loader::{Filter, Loader, LoaderTrait, error::LoadingError};
use schemars::JsonSchema;
use serde::Deserialize;

pub mod collection;
pub mod document;
pub mod v01;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[serde(tag = "version")]
pub enum VersionConfig {
    #[serde(rename = "0.1")]
    V01 {
        #[serde(flatten)]
        documents: v01::collection::DocumentCollectionV01,
    },
}

pub async fn load(loader: Loader) -> Result<DocumentCollection, LoadingError> {
    tracing::debug!("Loading  documents");
    let mut stream = loader.load_dir("", Filter::Yaml);
    let mut all_documents = DocumentCollection::default();
    while let Some(Ok(file)) = stream.next().await {
        let VersionConfig::V01 { documents } = serde_yml::from_slice::<VersionConfig>(&file.content)?;
        let collection: DocumentCollection = documents.into();
        for (id, mut doc) in collection.documents {
            let file = loader.get_file_metadata(&doc.file).await?;
            doc.set_file_metadata(file);
            all_documents.documents.insert(id, doc);
        }
    }
    Ok(all_documents)
}

#[cfg(test)]
mod tests {
    use std::fs::read_to_string;

    use super::*;

    #[test]
    fn test_collection_loading() {
        let collection_file = read_to_string("test_configs/test.collection.yaml").unwrap();
        let VersionConfig::V01 { documents } = serde_yml::from_str::<VersionConfig>(&collection_file).unwrap();
        let collection: DocumentCollection = documents.into();
        assert_eq!(collection.documents.len(), 1);
    }
}

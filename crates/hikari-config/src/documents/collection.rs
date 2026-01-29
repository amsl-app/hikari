use std::collections::HashMap;

use serde::Deserialize;

use crate::documents::{document::DocumentConfig, v01::collection::DocumentCollectionV01};

#[derive(Default, Debug, Clone, Deserialize)]
pub struct DocumentCollection {
    pub documents: HashMap<String, DocumentConfig>,
}

impl From<DocumentCollectionV01> for DocumentCollection {
    fn from(value: DocumentCollectionV01) -> Self {
        let mut documents = HashMap::new();
        for doc in value.documents {
            documents.insert(doc.id.clone(), doc);
        }
        DocumentCollection { documents }
    }
}

impl DocumentCollection {
    #[must_use]
    pub fn get(&self, document_id: &str) -> Option<&DocumentConfig> {
        self.documents.get(document_id)
    }

    #[must_use]
    pub fn documents(&self) -> &HashMap<String, DocumentConfig> {
        &self.documents
    }
}

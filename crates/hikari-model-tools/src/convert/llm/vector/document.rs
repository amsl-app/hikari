use hikari_entity::llm::vector::document::Model as DocumentModel;
use hikari_model::llm::vector::document::LlmDocument;

use crate::convert::TryFromDbModel;

impl TryFromDbModel<DocumentModel> for LlmDocument {
    type Error = serde_json::Error;

    fn try_from_db_model(model: DocumentModel) -> Result<Self, Self::Error> {
        Ok(Self {
            id: model.id,
            hash: model.hash,
            hash_algorithm: model.hash_algorithm,
            name: model.name,
            link: model.link,
        })
    }
}

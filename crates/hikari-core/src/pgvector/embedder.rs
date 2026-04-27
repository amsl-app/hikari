use async_openai::{
    Client,
    config::OpenAIConfig,
    types::embeddings::{CreateEmbeddingRequestArgs, EmbeddingInput},
};
use tracing::instrument;

use crate::pgvector::error::PgVectorError;

pub struct Embedder {
    pub model: String,
    pub client: Client<OpenAIConfig>,
}

impl Embedder {
    #[must_use]
    pub fn new(model: String, client: Client<OpenAIConfig>) -> Self {
        Self { model, client }
    }

    #[instrument(skip_all, fields(model = %self.model))]
    pub async fn embed<T: Into<Vec<String>>>(&self, texts: T) -> Result<Vec<Vec<f64>>, PgVectorError> {
        let text_vec = texts.into();
        if text_vec.is_empty() {
            tracing::warn!("no texts provided for embedding. Returning empty vector.");
            return Ok(vec![]);
        }

        let req = CreateEmbeddingRequestArgs::default()
            .model(&self.model)
            .input(EmbeddingInput::StringArray(text_vec))
            .build()?;

        let response = self.client.embeddings().create(req).await?;

        let embeddings = response
            .data
            .into_iter()
            .map(|item| item.embedding)
            .map(|embedding| embedding.into_iter().map(f64::from).collect::<Vec<f64>>())
            .collect();
        Ok(embeddings)
    }
}

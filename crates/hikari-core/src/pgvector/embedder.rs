use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{CreateEmbeddingRequestArgs, EmbeddingInput},
};

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

    pub async fn embed<T: Into<Vec<String>>>(&self, texts: T) -> Result<Vec<Vec<f64>>, PgVectorError> {
        let req = CreateEmbeddingRequestArgs::default()
            .model(&self.model)
            .input(EmbeddingInput::StringArray(texts.into()))
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

pub mod slides;
pub mod text;

use std::pin::Pin;

use futures::{FutureExt, future::BoxFuture};
use hikari_model::llm::vector::embedding_chunk::LlmEmbeddingChunk;
use hikari_utils::loader::{error::LoadingError, file::File};
use tokio::sync::{Mutex, OnceCell};
use tracing::instrument;

use crate::pgvector::{embedder::Embedder, error::PgVectorError};

pub type RagDocumentLoaderFn =
    Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = Result<File, LoadingError>> + Send>> + Send + Sync>;
pub trait PgVectorDocumentTrait: Send {
    fn id(&self) -> &str;

    fn name(&self) -> &str;

    fn link(&self) -> &str;

    fn chunks<'a>(&'a self, embedder: &'a Embedder) -> BoxFuture<'a, Result<Vec<LlmEmbeddingChunk>, PgVectorError>>;
}

#[derive(Debug, Clone, Copy)]
pub enum ChunkKind {
    Text,
    Slides,
}

pub struct PgVectorDocument {
    pub id: String,

    pub exclude: Vec<usize>, // Pages to exclude

    pub load_fn: Mutex<Option<RagDocumentLoaderFn>>,

    pub loaded_file: OnceCell<File>,

    pub name: String,

    pub link: String,

    pub kind: ChunkKind,
}

impl PgVectorDocument {
    fn file(&self) -> BoxFuture<'_, Result<&File, LoadingError>> {
        async move {
            self.loaded_file
                .get_or_try_init(|| async {
                    let mut load_fn_guard = self.load_fn.lock().await;
                    let Some(load_fn) = load_fn_guard.take() else {
                        return Err(LoadingError::FileAlreadyLoaded);
                    };
                    load_fn().await
                })
                .await
        }
        .boxed()
    }
}

impl PgVectorDocumentTrait for PgVectorDocument {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn link(&self) -> &str {
        &self.link
    }

    #[instrument(skip_all, fields(id = self.id, kind = ?self.kind))]
    fn chunks<'a>(&'a self, embedder: &'a Embedder) -> BoxFuture<'a, Result<Vec<LlmEmbeddingChunk>, PgVectorError>> {
        async move {
            let file = self.file().await?;
            let exclude = &self.exclude;

            match self.kind {
                ChunkKind::Text => text::chunks(file, exclude, embedder).await,
                ChunkKind::Slides => slides::chunks(file, exclude, embedder).await,
            }
        }
        .boxed()
    }
}

fn cosine_similarity(v1: &[f64], v2: &[f64]) -> f64 {
    let dot_product = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum::<f64>();
    let norm_v1 = v1.iter().map(|a| a * a).sum::<f64>().sqrt();
    let norm_v2 = v2.iter().map(|a| a * a).sum::<f64>().sqrt();

    if norm_v1 == 0.0 || norm_v2 == 0.0 {
        0.0
    } else {
        dot_product / (norm_v1 * norm_v2)
    }
}

const MIN_CHUNK_SIZE: usize = 300;
const LOOSE_MAX_CHUNK_SIZE: usize = 800;
#[cfg(test)]
mod test {
    #[test]
    fn test_cosine_similarity() {
        let v1 = vec![1.0, 2.0, 3.0];
        let v2 = vec![4.0, 5.0, 6.0];
        let similarity = super::cosine_similarity(&v1, &v2);
        assert!((similarity - 0.9746318461970762).abs() < 1e-10);

        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![0.0, 1.0, 0.0];
        let similarity = super::cosine_similarity(&v1, &v2);
        assert!((similarity - 0.0).abs() < 1e-10);

        let v1 = vec![1.0, 2.0, 3.0];
        let v2 = vec![1.0, 2.0, 3.0];
        let similarity = super::cosine_similarity(&v1, &v2);
        assert!((similarity - 1.0).abs() < 1e-10);
    }
}

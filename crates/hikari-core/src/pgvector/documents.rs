pub mod slides;
pub mod text;

use std::pin::Pin;

use futures::{FutureExt, future::BoxFuture};
use hikari_model::llm::vector::embedding_chunk::LlmEmbeddingChunk;
use hikari_utils::loader::{error::LoadingError, file::File};

use crate::pgvector::{embedder::Embedder, error::PgVectorError};

pub type RagDocumentLoaderFn =
    Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = Result<File, LoadingError>> + Send>> + Send + Sync>;
pub trait PgVectorDocumentTrait: Send {
    fn id(&self) -> &str;

    fn name(&self) -> &str;

    fn link(&self) -> &str;

    fn get_load_fn(&mut self) -> Option<RagDocumentLoaderFn>;

    fn get_loaded_file(&self) -> Option<&File>;

    fn set_loaded_file(&mut self, file: File) -> &File;

    fn chunks<'a>(&'a mut self, embedder: &'a Embedder)
    -> BoxFuture<'a, Result<Vec<LlmEmbeddingChunk>, PgVectorError>>;

    fn load_file(&mut self) -> BoxFuture<'_, Result<File, LoadingError>> {
        async move {
            if let Some(load_fn) = self.get_load_fn() {
                let file = load_fn().await?;
                Ok(self.set_loaded_file(file).clone())
            } else {
                Err(LoadingError::FileAlreadyLoaded)
            }
        }
        .boxed()
    }

    fn file(&mut self) -> BoxFuture<'_, Result<&File, LoadingError>> {
        async move {
            // Note: The nicer implementation (see below) is currently not possible because of the infamous Problem Case #3.
            // See https://rust-lang.github.io/rfcs/2094-nll.html#problem-case-3-conditional-control-flow-across-functions
            // let Some(loaded_file) = self.get_loaded_file() else {
            //     let file = self.load_file().await?;
            //     return Ok(self.set_loaded_file(file));
            // };
            // Ok(loaded_file)
            if self.get_loaded_file().is_none() {
                let file = self.load_file().await?;
                return Ok(self.set_loaded_file(file));
            }
            Ok(self
                .get_loaded_file()
                .expect("Option None that we just checked that it's not None"))
        }
        .boxed()
    }
}

pub enum PgVectorDocument {
    Text(text::TextDocument),
    Slides(slides::SlidesDocument),
}

impl PgVectorDocumentTrait for PgVectorDocument {
    fn id(&self) -> &str {
        match self {
            PgVectorDocument::Text(doc) => doc.id(),
            PgVectorDocument::Slides(doc) => doc.id(),
        }
    }

    fn name(&self) -> &str {
        match self {
            PgVectorDocument::Text(doc) => doc.name(),
            PgVectorDocument::Slides(doc) => doc.name(),
        }
    }

    fn link(&self) -> &str {
        match self {
            PgVectorDocument::Text(doc) => doc.link(),
            PgVectorDocument::Slides(doc) => doc.link(),
        }
    }

    fn get_load_fn(&mut self) -> Option<RagDocumentLoaderFn> {
        match self {
            PgVectorDocument::Text(doc) => doc.get_load_fn(),
            PgVectorDocument::Slides(doc) => doc.get_load_fn(),
        }
    }

    fn get_loaded_file(&self) -> Option<&File> {
        match self {
            PgVectorDocument::Text(doc) => doc.get_loaded_file(),
            PgVectorDocument::Slides(doc) => doc.get_loaded_file(),
        }
    }

    fn set_loaded_file(&mut self, file: File) -> &File {
        match self {
            PgVectorDocument::Text(doc) => doc.set_loaded_file(file),
            PgVectorDocument::Slides(doc) => doc.set_loaded_file(file),
        }
    }

    fn chunks<'a>(
        &'a mut self,
        embedder: &'a Embedder,
    ) -> BoxFuture<'a, Result<Vec<LlmEmbeddingChunk>, PgVectorError>> {
        match self {
            PgVectorDocument::Text(doc) => doc.chunks(embedder),
            PgVectorDocument::Slides(doc) => doc.chunks(embedder),
        }
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

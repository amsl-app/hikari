use futures::FutureExt;
use futures::future::BoxFuture;
use hikari_model::llm::vector::embedding_chunk::LlmEmbeddingChunk;
use hikari_utils::loader::{error::LoadingError, file::File};
use regex::Regex;
use std::collections::VecDeque;
use std::sync::LazyLock;
use unicode_segmentation::UnicodeSegmentation;

use crate::pgvector::documents::{
    LOOSE_MAX_CHUNK_SIZE, MIN_CHUNK_SIZE, PgVectorDocumentTrait, RagDocumentLoaderFn, cosine_similarity,
};
use crate::pgvector::embedder::Embedder;
use crate::pgvector::error::PgVectorError;

static ENDS_WITH_PUNCTUATION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[.!?][ \t\r\n]*$").expect("ends with punctuation regex is invalid"));

pub struct TextDocument {
    pub id: String,

    pub load_fn: Option<RagDocumentLoaderFn>,

    pub exclude: Vec<usize>, // Pages to exclude

    pub loaded_file: Option<File>,

    pub name: String,

    pub link: String,
}

impl PgVectorDocumentTrait for TextDocument {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn link(&self) -> &str {
        &self.link
    }

    fn get_load_fn(&mut self) -> Option<RagDocumentLoaderFn> {
        self.load_fn.take()
    }

    fn get_loaded_file(&self) -> Option<&File> {
        self.loaded_file.as_ref()
    }

    fn set_loaded_file(&mut self, file: File) -> &File {
        self.loaded_file.insert(file)
    }

    fn chunks<'a>(
        &'a mut self,
        embedder: &'a Embedder,
    ) -> BoxFuture<'a, Result<Vec<LlmEmbeddingChunk>, PgVectorError>> {
        async move {
            let file = self.file().await?;

            let pages = if file.metadata.key.ends_with("pdf") {
                tracing::debug!("Extracting text from PDF");
                pdf_extract::extract_text_from_mem_by_pages(&file.content)?
            } else {
                let content = String::from_utf8(file.content.clone())
                    .map_err(|e| PgVectorError::LoadingError(LoadingError::from(e)))?;
                vec![content]
            };

            let (all_sentences, all_indices): (Vec<String>, Vec<usize>) = pages
                .iter()
                .enumerate()
                .flat_map(|(i, page)| {
                    page.unicode_sentences()
                        .map(std::string::ToString::to_string)
                        .filter(|s| !s.is_empty())
                        .map(|s| (s, i))
                        .collect::<Vec<(String, usize)>>()
                })
                .collect();

            let all_embeddings = embedder.embed(all_sentences.as_slice()).await?;

            let sentences_embedded: Vec<(String, Vec<f64>, usize)> = all_indices
                .into_iter()
                .zip(all_embeddings)
                .zip(all_sentences)
                .map(|((i, emb), s)| (s, emb, i))
                .collect();

            let mut pages_embedded: Vec<VecDeque<(String, Vec<f64>, f64)>> = {
                let mut pages_embedded: Vec<VecDeque<(String, Vec<f64>, f64)>> = vec![VecDeque::new(); pages.len()];
                let mut prev_embeddings: Option<Vec<f64>> = None;
                for (sentence, embedding, index) in sentences_embedded {
                    let similarity = if let Some(prev) = &prev_embeddings {
                        cosine_similarity(prev, &embedding)
                    } else {
                        -1.0
                    };

                    pages_embedded
                        .get_mut(index)
                        .expect("paged index out of bounds")
                        .push_back((sentence, embedding.clone(), similarity));
                    prev_embeddings = Some(embedding);
                }
                pages_embedded
            };

            let (similarities_sum, similarities_count) = pages_embedded
                .iter()
                .flat_map(|page| page.iter().map(|(_, _, sim)| *sim))
                .filter(|&sim| sim >= 0.0)
                .fold((0.0, 0.0), |(sum, count), sim| (sum + sim, count + 1.0));

            let count = similarities_sum / similarities_count;
            let similarity_avg = if count.is_normal() { count } else { 0.0 };

            tracing::debug!("Average similarity: {}", similarity_avg);

            let mut chunks: Vec<LlmEmbeddingChunk> = Vec::new();

            let mut current_chunk: Option<LlmEmbeddingChunk> = None;

            for (page_number, sentences) in pages_embedded.iter_mut().enumerate() {
                if sentences.is_empty() || self.exclude.contains(&(page_number + 1)) {
                    let content = sentences
                        .iter()
                        .map(|(s, _, _)| s.as_str())
                        .collect::<Vec<&str>>()
                        .join(" ");

                    tracing::debug!(%content, page = page_number + 1, "Excluding page");
                    continue;
                }

                while let Some((sentence, _, similarity)) = sentences.pop_front() {
                    if let Some(mut current_chunk) = current_chunk.take() {
                        if (similarity > similarity_avg && current_chunk.content.len() < LOOSE_MAX_CHUNK_SIZE)
                            || current_chunk.content.len() < MIN_CHUNK_SIZE
                            || !ENDS_WITH_PUNCTUATION.is_match(&current_chunk.content)
                        {
                            // Continue the current chunk
                            current_chunk.push_sentence(&sentence, vec![u32::try_from(page_number + 1).unwrap_or(0)]);

                            continue;
                        }
                        // Finalize the current chunk and start a new one
                        chunks.push(current_chunk);
                    }

                    current_chunk = Some(LlmEmbeddingChunk::new(
                        sentence,
                        vec![u32::try_from(page_number + 1).unwrap_or(0)],
                    ));
                }
            }

            Ok(chunks)
        }
        .boxed()
    }
}

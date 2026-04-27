use futures::FutureExt;
use futures::future::BoxFuture;
use hikari_model::llm::vector::embedding_chunk::LlmEmbeddingChunk;
use hikari_utils::loader::error::LoadingError;
use regex::Regex;
use std::collections::VecDeque;
use std::sync::LazyLock;
use tracing::instrument;
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

    #[instrument(skip_all, fields(id = self.id))]
    fn chunks<'a>(
        &'a mut self,
        embedder: &'a Embedder,
    ) -> BoxFuture<'a, Result<Vec<LlmEmbeddingChunk>, PgVectorError>> {
        async move {
            let file = self.load_file().await?;

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
                        .filter(|s| !s.trim().is_empty())
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

            let pages_embedded: Vec<VecDeque<(String, Vec<f64>, f64)>> = {
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

            tracing::debug!(name: "average_similarity", similarity_avg);

            Ok(build_text_chunks(pages_embedded, &self.exclude, similarity_avg))
        }
        .boxed()
    }
}

/// Builds chunks from pre-computed per-page sentence embeddings, skipping excluded pages.
///
/// `pages_embedded` is 0-indexed; `exclude` contains 1-indexed page numbers.
pub(super) fn build_text_chunks(
    pages_embedded: Vec<VecDeque<(String, Vec<f64>, f64)>>,
    exclude: &[usize],
    similarity_avg: f64,
) -> Vec<LlmEmbeddingChunk> {
    let mut chunks: Vec<LlmEmbeddingChunk> = Vec::new();
    let mut current_chunk: Option<LlmEmbeddingChunk> = None;

    for (page_number, mut sentences) in pages_embedded.into_iter().enumerate() {
        if sentences.is_empty() || exclude.contains(&(page_number + 1)) {
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
        if let Some(current_chunk) = current_chunk.take() {
            chunks.push(current_chunk);
        }
    }

    chunks
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use hikari_model::llm::vector::embedding_chunk::LlmEmbeddingChunk;

    use super::build_text_chunks;

    fn page(sentences: Vec<(&str, f64)>) -> VecDeque<(String, Vec<f64>, f64)> {
        sentences
            .into_iter()
            .map(|(s, sim)| (s.to_string(), vec![0.0_f64], sim))
            .collect()
    }

    fn chunk_contents(chunks: &[LlmEmbeddingChunk]) -> Vec<String> {
        chunks.iter().map(|c| c.content.clone()).collect()
    }

    #[test]
    fn test_build_text_chunks_excludes_page() {
        // Page 2 (1-indexed) is in the exclude list — its content must not appear in any chunk.
        let pages = vec![
            page(vec![("Hello world.", -1.0)]),
            page(vec![("Secret content.", -1.0)]), // page 2, excluded
            page(vec![("Goodbye world.", -1.0)]),
        ];
        let result = build_text_chunks(pages, &[2], 0.0);
        let contents = chunk_contents(&result);
        assert_eq!(contents.len(), 2, "expected two chunks from the non-excluded pages");
        assert!(
            contents.iter().all(|c| !c.contains("Secret")),
            "excluded page content appeared in output: {contents:?}"
        );
    }

    #[test]
    fn test_build_text_chunks_empty_page_skipped() {
        let pages = vec![
            page(vec![("Hello world.", -1.0)]),
            VecDeque::new(), // empty page
        ];
        let result: Vec<LlmEmbeddingChunk> = build_text_chunks(pages, &[], 0.0);
        // Only the first page contributes; the empty page produces nothing.
        assert!(result.is_empty() || result.iter().all(|c| !c.content.is_empty()));
    }

    #[test]
    fn test_build_text_chunks_no_exclusions_keeps_all_pages() {
        let pages = vec![page(vec![("First page.", -1.0)]), page(vec![("Second page.", -1.0)])];
        let result = build_text_chunks(pages, &[], 0.0);
        println!("result: {result:#?}");
        let all_content = result
            .iter()
            .map(|c: &LlmEmbeddingChunk| c.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        print!("all_content: {all_content}");
        assert!(all_content.contains("First") || all_content.contains("Second"));
    }
}

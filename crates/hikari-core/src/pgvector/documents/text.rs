use std::collections::VecDeque;
use std::sync::LazyLock;

use futures::FutureExt;
use futures::future::BoxFuture;
use hikari_model::llm::vector::embedding_chunk::LlmEmbeddingChunk;
use hikari_utils::loader::{error::LoadingError, file::File};
use regex::Regex;
use tracing::instrument;
use unicode_segmentation::UnicodeSegmentation;

use crate::pgvector::documents::{LOOSE_MAX_CHUNK_SIZE, MIN_CHUNK_SIZE, cosine_similarity};
use crate::pgvector::embedder::Embedder;
use crate::pgvector::error::PgVectorError;

type EmbeddedSentence = (String, Vec<f64>, usize);
type EmbeddedPageSentence = (String, Vec<f64>, f64);

static ENDS_WITH_PUNCTUATION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[.!?][ \t\r\n]*$").expect("ends with punctuation regex is invalid"));

#[instrument(skip_all, fields(file_key = %file.metadata.key))]
fn extract_pages(file: &File) -> Result<Vec<String>, PgVectorError> {
    if file.metadata.key.ends_with("pdf") {
        tracing::debug!("Extracting text from PDF");
        pdf_extract::extract_text_from_mem_by_pages(&file.content).map_err(Into::into)
    } else {
        let content =
            String::from_utf8(file.content.clone()).map_err(|e| PgVectorError::LoadingError(LoadingError::from(e)))?;
        Ok(vec![content])
    }
}

#[instrument(skip_all, fields(page_count = pages.len()))]
fn collect_sentences_with_indices(pages: &[String]) -> (Vec<String>, Vec<usize>) {
    pages
        .iter()
        .enumerate()
        .flat_map(|(i, page)| {
            page.unicode_sentences()
                .map(std::string::ToString::to_string)
                .filter(|s| !s.is_empty())
                .map(|s| (s, i))
                .collect::<Vec<(String, usize)>>()
        })
        .collect()
}

#[instrument(skip_all, fields(sentence_count = sentences.len()))]
async fn embed_sentences(embedder: &Embedder, sentences: &[String]) -> Result<Vec<Vec<f64>>, PgVectorError> {
    embedder.embed(sentences).await
}

#[instrument(skip_all, fields(page_count, entry_count = sentences_embedded.len()))]
fn build_pages_embedded(
    page_count: usize,
    sentences_embedded: Vec<EmbeddedSentence>,
) -> Vec<VecDeque<EmbeddedPageSentence>> {
    let mut pages_embedded: Vec<VecDeque<EmbeddedPageSentence>> = vec![VecDeque::new(); page_count];
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
}

#[instrument(skip_all, fields(page_count = pages_embedded.len()))]
fn calculate_similarity_average(pages_embedded: &[VecDeque<EmbeddedPageSentence>]) -> f64 {
    let (similarities_sum, similarities_count) = pages_embedded
        .iter()
        .flat_map(|page| page.iter().map(|(_, _, sim)| *sim))
        .filter(|&sim| sim >= 0.0)
        .fold((0.0, 0.0), |(sum, count), sim| (sum + sim, count + 1.0));

    let count = similarities_sum / similarities_count;
    let similarity_avg = if count.is_normal() { count } else { 0.0 };
    tracing::debug!(name: "average_similarity", similarity_avg);
    similarity_avg
}

#[instrument(skip_all, fields(page_count = pages_embedded.len(), exclude_len = exclude.len()))]
fn build_chunks_from_pages(
    pages_embedded: &mut [VecDeque<EmbeddedPageSentence>],
    exclude: &[usize],
    similarity_avg: f64,
) -> Vec<LlmEmbeddingChunk> {
    let mut chunks: Vec<LlmEmbeddingChunk> = Vec::new();
    let mut current_chunk: Option<LlmEmbeddingChunk> = None;

    for (page_number, sentences) in pages_embedded.iter_mut().enumerate() {
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
    }

    chunks
}

#[instrument(skip_all, fields(file_key = %file.metadata.key, exclude_len = exclude.len()))]
pub fn chunks<'a>(
    file: &'a File,
    exclude: &'a [usize],
    embedder: &'a Embedder,
) -> BoxFuture<'a, Result<Vec<LlmEmbeddingChunk>, PgVectorError>> {
    async move {
        let pages = extract_pages(file)?;
        let (all_sentences, all_indices) = collect_sentences_with_indices(&pages);
        let all_embeddings = embed_sentences(embedder, &all_sentences).await?;

        let sentences_embedded: Vec<EmbeddedSentence> = all_indices
            .into_iter()
            .zip(all_embeddings)
            .zip(all_sentences)
            .map(|((i, emb), s)| (s, emb, i))
            .collect();

        let mut pages_embedded = build_pages_embedded(pages.len(), sentences_embedded);
        let similarity_avg = calculate_similarity_average(&pages_embedded);
        let chunks = build_chunks_from_pages(&mut pages_embedded, exclude, similarity_avg);
        Ok(chunks)
    }
    .boxed()
}

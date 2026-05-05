use futures::FutureExt;
use futures::future::BoxFuture;
use hikari_model::llm::vector::embedding_chunk::LlmEmbeddingChunk;
use hikari_utils::loader::{error::LoadingError, file::File};
use std::collections::VecDeque;
use tracing::instrument;
use unicode_segmentation::UnicodeSegmentation;

use crate::pgvector::documents::{LOOSE_MAX_CHUNK_SIZE, MIN_CHUNK_SIZE, cosine_similarity};
use crate::pgvector::embedder::Embedder;
use crate::pgvector::error::PgVectorError;

type EmbeddedSentence = (String, Vec<f64>, usize);
type EmbeddedPageSentence = (String, Vec<f64>, f64);

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
                .map(ToString::to_string)
                .filter(|s| !s.trim().is_empty())
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

    for (index, sentences) in pages_embedded.iter_mut().enumerate() {
        let page_number = index + 1;
        if sentences.is_empty() || exclude.contains(&(page_number)) {
            let content = sentences
                .iter()
                .map(|(s, _, _)| s.as_str())
                .collect::<Vec<&str>>()
                .join(" ");

            tracing::debug!(%content, page = page_number, "Excluding page");
            continue;
        }

        while let Some((sentence, _, similarity)) = sentences.pop_front() {
            let should_extend = current_chunk.as_ref().is_some_and(|chunk| {
                (similarity > similarity_avg && chunk.content.len() < LOOSE_MAX_CHUNK_SIZE)
                    || chunk.content.len() < MIN_CHUNK_SIZE
            });

            if should_extend {
                if let Some(ref mut chunk) = current_chunk {
                    chunk.push_sentence(&sentence, vec![u32::try_from(page_number).unwrap_or(0)]);
                }
                continue;
            }

            if let Some(finished) = current_chunk.take() {
                chunks.push(finished);
            }

            current_chunk = Some(LlmEmbeddingChunk::new(
                sentence,
                vec![u32::try_from(page_number).unwrap_or(0)],
            ));
        }
    }

    if let Some(current_chunk) = current_chunk {
        chunks.push(current_chunk);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    fn page(sentences: Vec<(&str, f64)>) -> VecDeque<EmbeddedPageSentence> {
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
        let mut pages = vec![
            page(vec![("Hello world.", -1.0)]),
            page(vec![("Secret content.", -1.0)]), // page 2, excluded
            page(vec![("Goodbye world.", -1.0)]),
        ];
        let result = build_chunks_from_pages(&mut pages, &[2], 0.0);
        let contents = chunk_contents(&result);
        assert_eq!(
            contents.len(),
            1,
            "expected one merged chunk, since they are too short to be separate"
        );
        assert!(
            contents.iter().all(|c| !c.contains("Secret")),
            "excluded page content appeared in output: {contents:?}"
        );
        assert!(
            contents.iter().any(|c| c.contains("Hello world.")),
            "expected content from page 1 to be included in the chunk"
        );
        assert!(
            contents.iter().any(|c| c.contains("Goodbye world.")),
            "expected content from page 3 to be included in the chunk"
        );
    }

    #[test]
    fn test_build_text_chunks_empty_page_skipped() {
        let mut pages = vec![
            page(vec![("Hello world.", -1.0)]),
            VecDeque::new(), // empty page
        ];
        let result: Vec<LlmEmbeddingChunk> = build_chunks_from_pages(&mut pages, &[], 0.0);

        assert_eq!(result.len(), 1, "expected one chunk for the non-empty page");
        assert_eq!(result[0].content, "Hello world.",);
    }

    #[test]
    fn test_build_text_chunks_no_exclusions_keeps_all_pages() {
        let mut pages = vec![page(vec![(
            "Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua. At vero eos et accusam et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus est Lorem ipsum dolor sit amet. Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua. At vero eos et accusam et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus est Lorem ipsum dolor sit amet. Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua. At vero eos et accusam et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus est Lorem ipsum dolor sit amet.  
            Duis autem vel eum iriure dolor in hendrerit in vulputate velit esse molestie consequat, vel illum dolore eu feugiat nulla facilisis at vero eros et accumsan et iusto odio dignissim qui blandit praesent luptatum zzril delenit augue duis dolore te feugait nulla facilisi. Lorem ipsum dolor sit amet, consectetuer adipiscing elit, sed diam nonummy nibh euismod tincidunt ut laoreet dolore magna aliquam erat volutpat.  
            Ut wisi enim ad minim veniam, quis nostrud exerci tation ullamcorper suscipit lobortis nisl ut aliquip ex ea commodo consequat. Duis autem vel eum iriure dolor in hendrerit in vulputate velit esse molestie consequat, vel illum dolore eu feugiat nulla facilisis at vero eros et accumsan et iusto odio dignissim qui blandit praesent luptatum zzril delenit augue duis dolore te feugait nulla facilisi.  
            Nam liber tempor cum soluta nobis eleifend option congue nihil imperdiet doming id quod mazim placerat facer possim assum. Lorem", -1.0)]), page(vec![("Second page.", -1.0)])];
        let result = build_chunks_from_pages(&mut pages, &[], 0.0);
        assert_eq!(
            result.len(),
            2,
            "expected two chunks for the two pages which are both long enough to be separate"
        );

        assert!(
            result.iter().any(|c| c.content.contains("Lorem ipsum dolor sit amet")),
            "expected content from page 1 to be included in a chunk"
        );
        assert!(
            result.iter().any(|c| c.content.contains("Second page.")),
            "expected content from page 2 to be included in a chunk"
        );
    }
}

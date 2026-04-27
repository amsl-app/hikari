use std::{collections::HashSet, vec};

use futures::{FutureExt, future::BoxFuture};
use hikari_model::llm::vector::embedding_chunk::LlmEmbeddingChunk;
use hikari_utils::loader::error::LoadingError;
use tracing::instrument;

use crate::pgvector::{
    documents::{MIN_CHUNK_SIZE, PgVectorDocumentTrait, RagDocumentLoaderFn, cosine_similarity},
    embedder::Embedder,
    error::PgVectorError,
};

pub struct SlidesDocument {
    pub id: String,

    pub exclude: Vec<usize>, // Pages to exclude

    pub load_fn: Option<RagDocumentLoaderFn>,

    pub name: String,

    pub link: String,
}

impl PgVectorDocumentTrait for SlidesDocument {
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
            let file_key = file.metadata.key.clone();

            if file_key.ends_with("pdf") {
                tracing::debug!("extracting text from PDF");
                let pages = {
                    let pages = pdf_extract::extract_text_from_mem_by_pages(&file.content)?;
                    drop(file);
                    pages
                };
                let pages_numbered = filter_excluded_pages(
                    pages.into_iter().enumerate().map(|(i, c)| (c, i + 1)).collect(),
                    &self.exclude,
                );

                let contents: Vec<String> = pages_numbered.iter().map(|(content, _)| content.clone()).collect();

                let embeddings = embedder.embed(contents).await?;

                tracing::debug!(exclude = ?self.exclude, "excluding pages");

                let pages_embeddings: Vec<(LlmEmbeddingChunk, Vec<f64>)> = pages_numbered
                    .into_iter()
                    .zip(embeddings)
                    .map(|((content, page_number), embedding)| {
                        (
                            LlmEmbeddingChunk::new(content, vec![u32::try_from(page_number).unwrap_or(0)]),
                            embedding,
                        )
                    })
                    .collect();

                Ok(merge_small_pages(pages_embeddings))
            } else {
                Err(PgVectorError::LoadingError(LoadingError::UnsupportedFileType(file_key)))
            }
        }
        .boxed()
    }
}

/// Removes pages that are empty or listed in `exclude` (1-indexed page numbers).
pub(super) fn filter_excluded_pages(pages: Vec<(String, usize)>, exclude: &[usize]) -> Vec<(String, usize)> {
    pages
        .into_iter()
        .filter(|(content, page_number)| !content.trim().is_empty() && !exclude.contains(page_number))
        .collect()
}

/// Merges pages whose content is shorter than `MIN_CHUNK_SIZE` into their most
/// similar neighbour, then returns the final list of chunks.
pub(super) fn merge_small_pages(mut pages_embeddings: Vec<(LlmEmbeddingChunk, Vec<f64>)>) -> Vec<LlmEmbeddingChunk> {
    let small_pages_idx: Vec<u32> = pages_embeddings
        .iter()
        .enumerate()
        .filter_map(|(idx, (c, _))| {
            if c.content.len() < MIN_CHUNK_SIZE {
                Some(u32::try_from(idx).unwrap_or(0))
            } else {
                None
            }
        })
        .collect();

    tracing::debug!(?small_pages_idx, "small pages idx");

    let mut merge_actions = Vec::new();
    let mut indices_to_remove = HashSet::new();

    for &position in small_pages_idx.iter().rev() {
        let position = usize::try_from(position).unwrap_or(0);

        let Some((_, current_emb)) = pages_embeddings.get(position) else {
            continue;
        };

        let prev_sim = if position == 0 {
            -1.0
        } else {
            pages_embeddings
                .get(position - 1)
                .map_or(-1.0, |(_, prev_emb)| cosine_similarity(prev_emb, current_emb))
        };

        let next_sim = pages_embeddings
            .get(position + 1)
            .map_or(-1.0, |(_, next_emb)| cosine_similarity(next_emb, current_emb));

        if prev_sim > next_sim {
            merge_actions.push((position, position - 1, prev_sim));
        } else {
            merge_actions.push((position, position + 1, next_sim));
        }
        indices_to_remove.insert(position);
    }

    // Sort by similarity descending so that the most similar merges happen first
    // (important when there are chains of small pages).
    merge_actions.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    // Follow merge chains to find the final target for each small page.
    let mut merge_map = Vec::new();
    for (from, to, _) in &merge_actions {
        let mut prev_targets = vec![*from];
        let mut target = *to;

        while let Some((_, new_target, _)) = merge_actions.iter().find(|&(f, _, _)| *f == target) {
            tracing::debug!("following merge chain: {} -> {}", target + 1, new_target + 1);

            if prev_targets.contains(new_target) {
                tracing::warn!(from = target + 1, to = new_target + 1, "skipping merge");
                break;
            }
            prev_targets.push(target);
            target = *new_target;
        }

        merge_map.push((*from, target));
    }

    // Perform forward merges first to avoid index-shifting issues.
    let forward_merges: Vec<&(usize, usize)> = merge_map.iter().filter(|(from, to)| from < to).collect();
    let backward_merges: Vec<&(usize, usize)> = merge_map.iter().filter(|(from, to)| from > to).collect();

    for (from, to) in forward_merges.iter().rev() {
        tracing::debug!(from = from + 1, to = to + 1, "merging page");
        #[allow(clippy::indexing_slicing)]
        let from = std::mem::take(&mut pages_embeddings[*from]).0;
        #[allow(clippy::indexing_slicing)]
        pages_embeddings[*to].0.push_sentence(&from.content, from.pages);
    }

    for (from, to) in &backward_merges {
        tracing::debug!(from = from + 1, to = to + 1, "merging page");
        #[allow(clippy::indexing_slicing)]
        let from = std::mem::take(&mut pages_embeddings[*from]).0;
        #[allow(clippy::indexing_slicing)]
        pages_embeddings[*to].0.push_sentence(&from.content, from.pages);
    }

    let mut indices_vec: Vec<&usize> = indices_to_remove.iter().collect();
    indices_vec.sort_by(|a, b| b.cmp(a));

    for index in indices_vec {
        tracing::debug!(page = index + 1, "removing page");
        pages_embeddings.remove(*index);
    }

    pages_embeddings.into_iter().map(|(chunk, _)| chunk).collect()
}

#[cfg(test)]
mod tests {
    use hikari_model::llm::vector::embedding_chunk::LlmEmbeddingChunk;

    use super::{filter_excluded_pages, merge_small_pages};
    use crate::pgvector::documents::MIN_CHUNK_SIZE;

    fn large_content() -> String {
        "a".repeat(MIN_CHUNK_SIZE)
    }

    fn small_content() -> String {
        "b".repeat(10)
    }

    fn chunk(content: String, pages: Vec<u32>) -> LlmEmbeddingChunk {
        LlmEmbeddingChunk::new(content, pages)
    }

    // --- filter_excluded_pages ---

    #[test]
    fn test_filter_excluded_pages_removes_listed() {
        let pages = vec![
            ("hello".to_string(), 1),
            ("world".to_string(), 2),
            ("foo".to_string(), 3),
        ];
        let result = filter_excluded_pages(pages, &[2]);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|(_, p)| *p != 2));
    }

    #[test]
    fn test_filter_excluded_pages_removes_empty() {
        let pages = vec![("hello".to_string(), 1), ("   ".to_string(), 2), ("foo".to_string(), 3)];
        let result = filter_excluded_pages(pages, &[]);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|(_, p)| *p != 2));
    }

    #[test]
    fn test_filter_excluded_pages_keeps_rest() {
        let pages = vec![("hello".to_string(), 1), ("world".to_string(), 2)];
        let result = filter_excluded_pages(pages, &[]);
        assert_eq!(result.len(), 2);
    }

    // --- merge_small_pages ---

    #[test]
    fn test_merge_small_pages_no_small_pages() {
        let input = vec![
            (chunk(large_content(), vec![1]), vec![1.0_f64, 0.0]),
            (chunk(large_content(), vec![2]), vec![1.0, 0.0]),
        ];
        let result = merge_small_pages(input);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].pages, vec![1]);
        assert_eq!(result[1].pages, vec![2]);
    }

    #[test]
    fn test_merge_small_pages_merges_to_prev() {
        // Page 2 (small) is most similar to page 1 → merges backward into page 1.
        let input = vec![
            (chunk(large_content(), vec![1]), vec![1.0_f64, 0.0]),
            (chunk(small_content(), vec![2]), vec![1.0, 0.0]), // similar to page 1
            (chunk(large_content(), vec![3]), vec![0.0, 1.0]), // different
        ];
        let result = merge_small_pages(input);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].pages, vec![1, 2]);
        assert_eq!(result[1].pages, vec![3]);
    }

    #[test]
    fn test_merge_small_pages_merges_to_next() {
        // Page 2 (small) is most similar to page 3 → merges forward into page 3.
        let input = vec![
            (chunk(large_content(), vec![1]), vec![0.0_f64, 1.0]), // different
            (chunk(small_content(), vec![2]), vec![1.0, 0.0]),     // similar to page 3
            (chunk(large_content(), vec![3]), vec![1.0, 0.0]),
        ];
        let result = merge_small_pages(input);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].pages, vec![1]);
        assert_eq!(result[1].pages, vec![2, 3]);
    }

    #[test]
    fn test_merge_small_pages_chain() {
        // Page 2 (small) is similar to page 1; page 3 (small) is similar to page 4.
        // Both small pages merge independently to their respective large neighbours.
        let input = vec![
            (chunk(large_content(), vec![1]), vec![1.0_f64, 0.0]),
            (chunk(small_content(), vec![2]), vec![1.0, 0.0]), // similar to page 1
            (chunk(small_content(), vec![3]), vec![0.0, 1.0]), // similar to page 4
            (chunk(large_content(), vec![4]), vec![0.0, 1.0]),
        ];
        let result = merge_small_pages(input);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].pages, vec![1, 2]);
        assert_eq!(result[1].pages, vec![3, 4]);
    }
}

use std::{collections::HashSet, vec};

use futures::{FutureExt, future::BoxFuture};
use hikari_model::llm::vector::embedding_chunk::LlmEmbeddingChunk;
use hikari_utils::loader::{error::LoadingError, file::File};
use tracing::instrument;

use crate::pgvector::{
    documents::{MIN_CHUNK_SIZE, cosine_similarity},
    embedder::Embedder,
    error::PgVectorError,
};

type EmbeddedPage = (LlmEmbeddingChunk, Vec<f64>);
type MergeAction = (usize, usize, f64);
type MergePair = (usize, usize);

#[instrument(skip_all, fields(file_key = %file.metadata.key))]
fn extract_pdf_pages(file: &File) -> Result<Vec<String>, PgVectorError> {
    tracing::debug!("Extracting text from PDF");
    let mut pages = pdf_extract::extract_text_from_mem_by_pages(&file.content)?;
    for page in &mut pages {
        if page.is_empty() {
            // Preserve empty pages with a space
            *page = " ".to_string();
        }
    }
    Ok(pages)
}

#[instrument(skip_all, fields(page_count = pages.len(), exclude_len = exclude.len()))]
fn build_pages_embeddings(pages: Vec<String>, embeddings: Vec<Vec<f64>>, exclude: &[usize]) -> Vec<EmbeddedPage> {
    tracing::debug!("Excluding pages {:?}", exclude);
    pages
        .into_iter()
        .zip(embeddings)
        .enumerate()
        .filter_map(|(i, (content, embedding))| {
            if exclude.contains(&(i + 1)) {
                tracing::debug!(%content, page = i + 1, "Excluding page");
                return None;
            }
            Some((
                LlmEmbeddingChunk::new(content, vec![u32::try_from(i + 1).unwrap_or(0)]),
                embedding,
            ))
        })
        .collect()
}

#[instrument(skip_all, fields(page_count = pages_embeddings.len()))]
fn small_page_indices(pages_embeddings: &[EmbeddedPage]) -> Vec<u32> {
    // We get pages which are short (< 100)
    // Check if append to previous or next page makes sense
    // Both are present => highest similarity wins (if > 0.5)
    // Just prev or next => merge if similarity > 0.5
    // If similarity is not > 0.5, remove the page => it is probably noise or title page
    let small_pages_idx = pages_embeddings
        .iter()
        .enumerate()
        .filter_map(|(idx, (c, _))| {
            if c.content.len() < MIN_CHUNK_SIZE {
                Some(u32::try_from(idx).unwrap_or(0))
            } else {
                None
            }
        })
        .collect::<Vec<u32>>();
    tracing::debug!("Small pages idx: {:?}", small_pages_idx);
    small_pages_idx
}

#[instrument(skip_all, fields(page_count = pages_embeddings.len(), small_count = small_pages_idx.len()))]
fn build_merge_actions(pages_embeddings: &[EmbeddedPage], small_pages_idx: &[u32]) -> (Vec<MergeAction>, HashSet<usize>) {
    // Calculate inital merge actions by checking similarity to previous and next page
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
            // Merge into the previous page.
            merge_actions.push((position, position - 1, prev_sim));
        } else {
            // Merge into the next page.
            merge_actions.push((position, position + 1, next_sim));
        }
        indices_to_remove.insert(position);
    }

    // Sort by similarity descending, so we merge the most similar ones first
    // This is important if there are chains of small pages
    merge_actions.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    (merge_actions, indices_to_remove)
}

#[instrument(skip_all, fields(action_count = merge_actions.len()))]
fn build_merge_map(merge_actions: &[MergeAction]) -> Vec<MergePair> {
    // Calculate the final merge map, by following chains of merges
    let mut merge_map = Vec::new();
    for (from, to, _) in merge_actions {
        let mut prev_targets = vec![*from];
        let mut target = *to;

        while let Some((_, new_target, _)) = merge_actions.iter().find(|&(f, _, _)| *f == target) {
            tracing::debug!("Following merge chain: {} -> {}", target + 1, new_target + 1);

            if prev_targets.contains(new_target) {
                // This would create a cycle, skip it
                tracing::warn!("Skipping merge from {} to {} to avoid cycle", target + 1, new_target + 1);
                break;
            }
            prev_targets.push(target);
            target = *new_target;
        }

        merge_map.push((*from, target));
    }
    merge_map
}

#[instrument(skip_all, fields(page_count = pages_embeddings.len(), merge_count = merge_map.len()))]
fn apply_merges(pages_embeddings: &mut [EmbeddedPage], merge_map: &[MergePair]) {
    // Perform merges, starting with forward merges (to avoid index shifting issues)
    let forward_merges = merge_map.iter().filter(|(from, to)| from < to).rev();
    let backward_merges = merge_map.iter().filter(|(from, to)| from > to);
    let merges = forward_merges.chain(backward_merges);

    for (from, to) in merges {
        if let (Some((from_chunk, _)), Some((to_chunk, _))) =
            (pages_embeddings.get(*from).cloned(), pages_embeddings.get_mut(*to))
        {
            tracing::debug!("Merging page {} into page {}", from + 1, to + 1);
            to_chunk.push_sentence(&from_chunk.content, from_chunk.pages);
        }
    }
}

#[instrument(skip_all, fields(page_count = pages_embeddings.len(), remove_count = indices_to_remove.len()))]
fn remove_merged_pages(pages_embeddings: &mut Vec<EmbeddedPage>, indices_to_remove: &HashSet<usize>) {
    let mut indices_vec: Vec<&usize> = indices_to_remove.iter().collect();
    indices_vec.sort_by(|a, b| b.cmp(a)); // Sort in descending order

    for index in indices_vec {
        tracing::debug!("Removing page {}", index + 1);
        pages_embeddings.remove(*index);
    }
}

#[instrument(skip_all, fields(file_key = %file.metadata.key, exclude_len = exclude.len()))]
pub fn chunks<'a>(
    file: &'a File,
    exclude: &'a [usize],
    embedder: &'a Embedder,
) -> BoxFuture<'a, Result<Vec<LlmEmbeddingChunk>, PgVectorError>> {
    async move {
        if !file.metadata.key.ends_with("pdf") {
            return Err(PgVectorError::LoadingError(LoadingError::UnsupportedFileType(
                file.metadata.key.clone(),
            )));
        }

        // Get pages and embeddings
        let pages = extract_pdf_pages(file)?;
        let embeddings = embedder.embed(pages.as_slice()).await?;

        let mut pages_embeddings = build_pages_embeddings(pages, embeddings, exclude);
        let small_pages_idx = small_page_indices(&pages_embeddings);
        let (merge_actions, indices_to_remove) = build_merge_actions(&pages_embeddings, &small_pages_idx);
        let merge_map = build_merge_map(&merge_actions);
        apply_merges(&mut pages_embeddings, &merge_map);
        remove_merged_pages(&mut pages_embeddings, &indices_to_remove);

        let chunks: Vec<LlmEmbeddingChunk> = pages_embeddings.into_iter().map(|(chunk, _)| chunk).collect();
        Ok(chunks)
    }
    .boxed()
}

#[cfg(test)]
mod test {
    use hikari_model::llm::vector::embedding_chunk::LlmEmbeddingChunk;

    use super::apply_merges;

    fn embedded_page(content: &str, page: u32) -> (LlmEmbeddingChunk, Vec<f64>) {
        (LlmEmbeddingChunk::new(content.to_string(), vec![page]), vec![])
    }

    #[test]
    fn test_apply_merges_forward() {
        let mut pages = vec![
            embedded_page("p0", 0),
            embedded_page("p1", 1),
            embedded_page("p2", 2),
            embedded_page("p3", 3),
        ];

        apply_merges(&mut pages, &[(0, 2), (1, 2)]);

        assert_eq!(pages[2].0.content, "p2 p1 p0");
        assert_eq!(pages[2].0.pages, vec![0, 1, 2]);
    }

    #[test]
    fn test_apply_merges_backward() {
        let mut pages = vec![
            embedded_page("p0", 0),
            embedded_page("p1", 1),
            embedded_page("p2", 2),
            embedded_page("p3", 3),
        ];

        apply_merges(&mut pages, &[(3, 1), (2, 1)]);

        assert_eq!(pages[1].0.content, "p1 p3 p2");
        assert_eq!(pages[1].0.pages, vec![1, 2, 3]);
    }

    #[test]
    fn test_apply_merges_mixed() {
        let mut pages = vec![
            embedded_page("p0", 0),
            embedded_page("p1", 1),
            embedded_page("p2", 2),
            embedded_page("p3", 3),
            embedded_page("p4", 4),
        ];

        apply_merges(&mut pages, &[(0, 2), (4, 2), (3, 1)]);

        assert_eq!(pages[2].0.content, "p2 p0 p4");
        assert_eq!(pages[2].0.pages, vec![0, 2, 4]);
        assert_eq!(pages[1].0.content, "p1 p3");
        assert_eq!(pages[1].0.pages, vec![1, 3]);
    }
}

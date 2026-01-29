use std::{collections::HashSet, vec};

use futures::{FutureExt, future::BoxFuture};
use hikari_model::llm::vector::embedding_chunk::LlmEmbeddingChunk;
use hikari_utils::loader::{error::LoadingError, file::File};

use crate::pgvector::{
    documents::{MIN_CHUNK_SIZE, PgVectorDocumentTrait, RagDocumentLoaderFn, cosine_similarity},
    embedder::Embedder,
    error::PgVectorError,
};

pub struct SlidesDocument {
    pub id: String,

    pub exclude: Vec<usize>, // Pages to exclude

    pub load_fn: Option<RagDocumentLoaderFn>,

    pub loaded_file: Option<File>,

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

            if file.metadata.key.ends_with("pdf") {
                // Get pages and embeddings
                tracing::debug!("Extracting text from PDF");
                let mut pages = pdf_extract::extract_text_from_mem_by_pages(&file.content)?;
                for page in &mut pages {
                    if page.is_empty() {
                        *page = " ".to_string(); // Preserve empty pages with a space
                    }
                }
                let embeddings = embedder.embed(pages.as_slice()).await?;

                tracing::debug!("Excluding pages {:?}", self.exclude);

                let mut pages_embeddings: Vec<(LlmEmbeddingChunk, Vec<f64>)> = pages
                    .into_iter()
                    .zip(embeddings)
                    .enumerate()
                    .filter_map(|(i, (content, embedding))| {
                        if self.exclude.contains(&(i + 1)) {
                            tracing::debug!(%content, page = i + 1, "Excluding page");
                            return None;
                        }
                        Some((
                            LlmEmbeddingChunk::new(content, vec![u32::try_from(i + 1).unwrap_or(0)]),
                            embedding,
                        ))
                    })
                    .collect();

                // We get pages which are short (< 100)

                // Check if append to previous or next page makes sense
                // Both are present => highest similarity wins (if > 0.5)
                // Just prev or next => merge if similarity > 0.5
                // If similarity is not > 0.5, remove the page => it is probably noise or title page

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

                tracing::debug!("Small pages idx: {:?}", small_pages_idx);

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

                // Calculate the final merge map, by following chains of merges
                let mut merge_map = Vec::new();
                for (from, to, _) in &merge_actions {
                    let mut prev_targets = vec![*from];
                    let mut target = *to;

                    while let Some((_, new_target, _)) = merge_actions.iter().find(|&(f, _, _)| *f == target) {
                        tracing::debug!("Following merge chain: {} -> {}", target + 1, new_target + 1);

                        if prev_targets.contains(new_target) {
                            // This would create a cycle, skip it
                            tracing::warn!(
                                "Skipping merge from {} to {} to avoid cycle",
                                target + 1,
                                new_target + 1
                            );
                            break;
                        }
                        prev_targets.push(target);
                        target = *new_target;
                    }

                    merge_map.push((*from, target));
                }

                // Perform merges, starting with forward merges (to avoid index shifting issues)
                let forward_merges: Vec<&(usize, usize)> = merge_map.iter().filter(|(from, to)| from < to).collect();
                let backward_merges: Vec<&(usize, usize)> = merge_map.iter().filter(|(from, to)| from > to).collect();

                for (from, to) in forward_merges.iter().rev() {
                    if let (Some((from_chunk, _)), Some((to_chunk, _))) =
                        (pages_embeddings.get(*from).cloned(), pages_embeddings.get_mut(*to))
                    {
                        tracing::debug!("Merging page {} into page {}", from + 1, to + 1);
                        to_chunk.push_sentence(&from_chunk.content, from_chunk.pages);
                    }
                }

                for (from, to) in &backward_merges {
                    if let (Some((from_chunk, _)), Some((to_chunk, _))) =
                        (pages_embeddings.get(*from).cloned(), pages_embeddings.get_mut(*to))
                    {
                        tracing::debug!("Merging page {} into page {}", from + 1, to + 1);
                        to_chunk.push_sentence(&from_chunk.content, from_chunk.pages);
                    }
                }

                let mut indices_vec: Vec<&usize> = indices_to_remove.iter().collect();
                indices_vec.sort_by(|a, b| b.cmp(a)); // Sort in descending order

                for index in indices_vec {
                    tracing::debug!("Removing page {}", index + 1);
                    pages_embeddings.remove(*index);
                }

                let chunks: Vec<LlmEmbeddingChunk> = pages_embeddings.into_iter().map(|(chunk, _)| chunk).collect();
                Ok(chunks)
            } else {
                Err(PgVectorError::LoadingError(LoadingError::UnsupportedFileType(
                    file.metadata.key.clone(),
                )))
            }
        }
        .boxed()
    }
}

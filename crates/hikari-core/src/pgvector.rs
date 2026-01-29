use crate::llm_config::LlmConfig;
use crate::pgvector::documents::{PgVectorDocument, PgVectorDocumentTrait};
use crate::pgvector::embedder::Embedder;
use crate::pgvector::error::PgVectorError;
use async_openai::Client;
use hikari_config::module::llm_agent::LlmService;
use hikari_db::llm::vector as vector_db;
use hikari_model::llm::vector::embedding_chunk::{LlmEmbeddingChunk, LlmEmbeddingQueryResult, Source};
use hikari_utils::loader::file::FileMetadata;
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;
use sea_orm::query::Value;
use sea_orm::{ConnectionTrait, DbBackend, QueryResult, Statement, TransactionTrait};
use std::vec;
use xxhash_rust::xxh3::xxh3_64;
pub mod documents;
pub mod embedder;
pub mod error;

pub(crate) const EMBEDDING_TABLE: &str = "llm_embeddings";
pub(crate) const DOCUMENT_TABLE: &str = "llm_documents";

pub struct PgVector<'a> {
    conn: &'a DatabaseConnection,
    chunking_embedder: Embedder,
    embedder: Embedder,
}

impl PgVector<'_> {
    #[must_use]
    pub fn new<'a>(llm_config: &'a LlmConfig, conn: &'a DatabaseConnection) -> PgVector<'a> {
        PgVector {
            conn,
            chunking_embedder: Embedder::new(
                "text-embedding-3-large".into(),
                Client::with_config(llm_config.get_openai_config(Some(&LlmService::OpenAI))),
            ),
            embedder: Embedder::new(
                llm_config.get_embedding_model().into(),
                Client::with_config(llm_config.get_embedding_openai_config()),
            ),
        }
    }

    pub async fn upsert_file(
        &self,
        mut document: PgVectorDocument,
        file_metadata: Option<FileMetadata>,
    ) -> Result<bool, PgVectorError> {
        let existing_file = vector_db::document::Query::get_file(self.conn, document.id()).await?;

        let existing_hash = existing_file.as_ref().and_then(|f| f.hash.as_ref());
        let existing_hash_algorithm = existing_file.as_ref().and_then(|f| f.hash_algorithm.as_ref());
        let existing_created_at = existing_file.as_ref().map(|f| f.created_at.and_utc());

        let new_hash_meta = file_metadata.as_ref().and_then(|m| m.hash.as_ref());
        let new_hash_algorithm = new_hash_meta.map(|m| m.algorithm.as_ref());
        let new_hash = new_hash_meta.map(|h| h.hash.as_ref());
        let new_last_modified = file_metadata.as_ref().and_then(|m| m.last_modified);

        // Vergleich der Hashes
        if let Some(old_hash) = existing_hash
            && let Some(new_hash) = new_hash
            && let Some(old_hash_algorithm) = existing_hash_algorithm
            && let Some(new_hash_algorithm) = new_hash_algorithm
        {
            if old_hash == new_hash && old_hash_algorithm == new_hash_algorithm {
                return Ok(false);
            }
        } else if let Some(new_created_at) = new_last_modified
            && let Some(existing_created_at) = existing_created_at
            && new_created_at <= existing_created_at
        {
            return Ok(false);
        }

        let chunks = document.chunks(&self.chunking_embedder).await?;

        self.insert_file(
            document.id(),
            document.name(),
            document.link(),
            new_hash,
            new_hash_algorithm,
            &chunks,
        )
        .await?;

        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    async fn insert_file(
        &self,
        file_id: &str,
        file_name: &str,
        file_link: &str,
        hash: Option<&str>,
        hash_algorithm: Option<&str>,
        chunks: &[LlmEmbeddingChunk],
    ) -> Result<(), PgVectorError> {
        let txn = self.conn.begin().await?;
        let hash = hash.map(std::string::ToString::to_string);
        let hash_algorithm = hash_algorithm.map(std::string::ToString::to_string);
        let file_name = file_name.to_string();
        let file_link = file_link.to_string();

        vector_db::document::Mutation::upsert_file(
            &txn,
            file_id.to_string(),
            hash,
            hash_algorithm,
            file_name,
            file_link,
        )
        .await?;

        let file_id = Value::from(file_id);

        // Split in content and metadata
        let (contents, pages): (Vec<String>, Vec<Vec<u32>>) =
            chunks.iter().map(|c| (c.content.clone(), c.pages.clone())).unzip();

        let embeddings = self.embedder.embed(contents.as_slice()).await?;

        if contents.len() != embeddings.len() || contents.len() != pages.len() {
            return Err(PgVectorError::VectorMissMatch);
        }

        for ((content, embedding), pages) in contents.iter().zip(embeddings).zip(pages) {
            let statement = Statement::from_sql_and_values(
                DbBackend::Postgres,
                format! {r"
                INSERT INTO {EMBEDDING_TABLE} (id, embedding, content, file_id, pages)
                VALUES ($1, $2, $3, $4, $5)",
                },
                vec![
                    Value::from(Uuid::new_v4()),
                    Value::from(embedding),
                    Value::from(content),
                    file_id.clone(),
                    Value::from(pages),
                ],
            );
            txn.execute(statement).await?;
        }
        txn.commit().await?;
        Ok(())
    }

    pub async fn search_by_documents(
        &self,
        query: &str,
        limit: u32,
        documents: &[String],
    ) -> Result<Vec<LlmEmbeddingQueryResult>, PgVectorError> {
        let query_vector = self.embedder.embed(&[query.to_owned()]).await?.swap_remove(0);
        let query_vector_string = query_vector
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<String>>()
            .join(",");
        let mut documents = documents.to_owned();
        documents.dedup();

        let documents_string = documents.join("','");

        let statement = Statement::from_string(
            DbBackend::Postgres,
            format! {r"
            WITH docs AS (
                SELECT id, name, link
                FROM {DOCUMENT_TABLE}
                WHERE id IN ('{documents_string}')
            )
            SELECT
                content,
                pages,
                name,
                link,
                distance
            FROM (
                SELECT
                    file_id,
                    content,
                    pages,
                    name,
                    link,
                    embedding <=> '[{query_vector_string}]' AS distance
                FROM
                     {EMBEDDING_TABLE} as embedding
                JOIN docs ON embedding.file_id = docs.id
            ) as chunks
            ORDER BY
                distance
            ASC
            LIMIT {limit}"
            },
        );

        let rows = self.conn.query_all(statement).await?;
        Self::handle_rows(&rows)
    }

    fn handle_rows(rows: &[QueryResult]) -> Result<Vec<LlmEmbeddingQueryResult>, PgVectorError> {
        rows.iter()
            .map(|row| {
                let content: String = row.try_get_by_index(0)?;
                //let pages: Vec<u32> = row.try_get_by_index(1)?; TODO: Enable pages again
                let pages = vec![];
                let name: String = row.try_get_by_index(2)?;
                let link: String = row.try_get_by_index(3)?;

                let entry = LlmEmbeddingQueryResult {
                    content,
                    source: Source { name, link, pages },
                };

                Ok(entry)
            })
            .collect()
    }
}

pub async fn upload_document(
    retriever: &PgVector<'_>,
    document: PgVectorDocument,
    file_metadata: Option<FileMetadata>,
) -> Result<(), PgVectorError> {
    retriever.upsert_file(document, file_metadata).await?;
    Ok(())
}

pub async fn search(
    llm_config: &LlmConfig,
    conn: &DatabaseConnection,
    query: &str,
    limit: u32,
    documents: &[String],
) -> Result<Vec<LlmEmbeddingQueryResult>, PgVectorError> {
    let retriever = PgVector::new(llm_config, conn);

    let results = async {
        for attempt in 0..3 {
            match tokio::time::timeout(
                std::time::Duration::from_secs(10),
                retriever.search_by_documents(query, limit, documents),
            )
            .await
            {
                Ok(Ok(results)) => return Ok(results),
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    if attempt < 2 {
                        tokio::time::sleep(std::time::Duration::from_millis(100 * (attempt + 1))).await;
                    }
                }
            }
        }
        Err(PgVectorError::Timeout)
    }
    .await?;

    Ok(results)
}

#[must_use]
pub fn hash_document(content: &str) -> String {
    let hash = xxh3_64(content.as_bytes());
    hex::encode(hash.to_le_bytes())
}

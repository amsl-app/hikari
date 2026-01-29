DELETE FROM llm_documents;

ALTER TABLE llm_embeddings
ALTER COLUMN embedding TYPE VECTOR (4096);
-- Your SQL goes here
ALTER TABLE llm_documents ALTER COLUMN hash DROP NOT NULL;
ALTER TABLE llm_documents ADD COLUMN IF NOT EXISTS hash_algorithm TEXT;
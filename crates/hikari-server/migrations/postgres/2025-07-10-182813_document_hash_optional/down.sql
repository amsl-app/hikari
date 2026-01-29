-- This file should undo anything in `up.sql`
ALTER TABLE llm_documents ALTER COLUMN hash SET NOT NULL;

ALTER TABLE llm_documents DROP COLUMN IF EXISTS hash_algorithm;
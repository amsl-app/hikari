CREATE TABLE llm_documents (
    id TEXT NOT NULL PRIMARY KEY,
    hash TEXT NOT NULL,
    collection TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL default current_timestamp,
    metadata JSONB
);

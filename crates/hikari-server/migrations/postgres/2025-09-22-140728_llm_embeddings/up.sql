CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE llm_embeddings (
    id uuid PRIMARY KEY,
    embedding VECTOR (4096) NOT NULL,
    content TEXT NOT NULL,
    file_id TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL default current_timestamp,
    pages INTEGER[] NOT NULL,
    
    foreign key (file_id) references llm_documents (id) on delete cascade
);
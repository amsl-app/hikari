-- Create the 'tags' table
CREATE TABLE llm_document_tags (
    name TEXT NOT NULL,
    file_id TEXT  NOT NULL,

    foreign key (file_id) references llm_documents (id) on delete cascade
);
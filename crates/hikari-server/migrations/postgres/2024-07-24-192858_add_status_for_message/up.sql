CREATE TYPE "message_status_enum" AS ENUM('generating', 'completed') ;

ALTER TABLE llm_message ADD status message_status_enum not null default 'generating';

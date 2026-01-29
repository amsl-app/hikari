CREATE TYPE "conversation_status_enum" AS ENUM('open', 'closed') ;
-- Create the conversation table
CREATE TABLE llm_conversation(
  conversation_id       uuid                            PRIMARY KEY NOT NULL,
  module_id             text                            NOT NULL,
  session_id            text                            NOT NULL,
  user_id               uuid                            NOT NULL,
  created_at            timestamp                       NOT NULL default current_timestamp,
  status                conversation_status_enum        NOT NULL default 'open',
  completed_at          timestamp,

  FOREIGN KEY(user_id) references USERS(id) ON DELETE cascade
);
CREATE INDEX idx_llm_conversation_session_id ON LLM_CONVERSATION(session_id) ;
CREATE INDEX idx_llm_conversation_user_id ON LLM_CONVERSATION(user_id)
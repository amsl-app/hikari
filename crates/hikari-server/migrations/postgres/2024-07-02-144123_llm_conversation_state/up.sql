CREATE TYPE "llm_step_state_enum" AS ENUM('running','waiting_for_input','completed','error', 'not_started'); ;
-- Create the conversation table
CREATE TABLE llm_conversation_state(
  conversation_id           uuid                                  PRIMARY KEY NOT NULL,
  step_state                llm_step_state_enum                   NOT NULL default 'not_started',
  current_step              text                                  NOT NULL,
  last_interaction_at       timestamp                             NOT NULL default current_timestamp,
  value                     text,

  FOREIGN KEY (conversation_id) REFERENCES llm_conversation(conversation_id) ON DELETE CASCADE
);
CREATE INDEX idx_llm_conversation_state_id ON LLM_CONVERSATION_STATE(conversation_id) ;

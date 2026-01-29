-- Create the slot table
CREATE TABLE llm_slot (
  conversation_id   uuid    NOT NULL,
  slot              text    NOT NULL,
  value             text    NOT NULL,

  PRIMARY KEY(conversation_id, slot),
  FOREIGN KEY(conversation_id) references llm_conversation(conversation_id) ON DELETE cascade
);

CREATE UNIQUE INDEX idx_llm_slot_conversation_id_slot ON llm_slot(conversation_id, slot);

-- Create the global slot table
CREATE TABLE llm_global_slot(
  user_id           uuid    NOT NULL,
  slot              text    NOT NULL,
  value             text    NOT NULL,

  PRIMARY KEY(user_id, slot),
  FOREIGN KEY(user_id) references "users"(id) ON DELETE cascade
);

CREATE UNIQUE INDEX idx_llm_global_slot_user_id_slot ON llm_global_slot(user_id, slot);

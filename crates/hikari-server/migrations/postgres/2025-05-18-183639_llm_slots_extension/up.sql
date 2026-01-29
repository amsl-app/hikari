CREATE TABLE llm_module_slot(
  user_id           uuid    NOT NULL,
  module_id         text    NOT NULL,
  slot              text    NOT NULL,
  value             text    NOT NULL,

  PRIMARY KEY(user_id, module_id, slot),
  FOREIGN KEY(user_id) references "users"(id) ON DELETE cascade
);

CREATE UNIQUE INDEX idx_llm_module_slot_user_id_slot ON llm_module_slot(user_id, module_id, slot);

CREATE TABLE llm_session_slot(
  user_id           uuid    NOT NULL,
  module_id         text    NOT NULL,
  session_id        text    NOT NULL,
  slot              text    NOT NULL,
  value             text    NOT NULL,

  PRIMARY KEY(user_id, module_id, session_id, slot),
  FOREIGN KEY(user_id) references "users"(id) ON DELETE cascade
);

CREATE UNIQUE INDEX idx_llm_session_slot_user_id_slot ON llm_session_slot(user_id, module_id, session_id, slot);

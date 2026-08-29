CREATE TABLE planner_entry (
    id         UUID PRIMARY KEY         NOT NULL,
    user_id    UUID                     NOT NULL,
    date       DATE                     NOT NULL,
    title      TEXT                     NOT NULL,
    completed  BOOLEAN                  NOT NULL DEFAULT false,
    priority   INTEGER                  NOT NULL,
    module_id  VARCHAR(255)                      DEFAULT NULL,
    session_id VARCHAR(255)                      DEFAULT NULL,
    created_at TIMESTAMP                NOT NULL DEFAULT now(),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX idx_planner_entry_user_id ON planner_entry(user_id);
CREATE INDEX idx_planner_entry_user_id_date ON planner_entry(user_id, date);

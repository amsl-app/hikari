CREATE TABLE planner_milestone (
    id          BLOB PRIMARY KEY NOT NULL,
    user_id     BLOB             NOT NULL,
    title       TEXT             NOT NULL,
    date        DATE             NOT NULL,
    description TEXT                      DEFAULT NULL,
    module_id   VARCHAR(255)              DEFAULT NULL,
    origin_id   VARCHAR(255)              DEFAULT NULL,
    created_at  TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX idx_planner_milestone_user_id ON planner_milestone(user_id);

ALTER TABLE planner_entry ADD COLUMN milestone_id BLOB DEFAULT NULL REFERENCES planner_milestone(id) ON DELETE SET NULL;
ALTER TABLE planner_entry DROP COLUMN module_id;
ALTER TABLE planner_entry DROP COLUMN session_id;

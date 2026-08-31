CREATE TABLE planner_goal (
    id BLOB PRIMARY KEY NOT NULL,
    user_id BLOB NOT NULL,
    name TEXT NOT NULL,
    description TEXT DEFAULT NULL,
    fulfilled BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX idx_planner_goal_user_id ON planner_goal(user_id);

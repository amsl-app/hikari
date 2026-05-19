CREATE TABLE user_context_logs (
    user_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    type TEXT NOT NULL,
    data JSONB NOT NULL,

    PRIMARY KEY (user_id, created_at, type),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX idx_user_context_logs_user_id ON user_context_logs(user_id);
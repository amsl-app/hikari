CREATE TABLE user_context_logs (
    user_id UUID NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    type TEXT NOT NULL,
    data JSONB NOT NULL,

    PRIMARY KEY (user_id, created_at, type),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX idx_user_context_logs_user_id_type ON user_context_logs(user_id, type);
CREATE INDEX idx_user_context_logs_created_at ON user_context_logs(user_id, type, created_at DESC);
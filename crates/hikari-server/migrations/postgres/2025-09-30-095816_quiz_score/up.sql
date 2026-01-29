CREATE TABLE quiz_score (
    user_id UUID NOT NULL,
    module_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    topic TEXT NOT NULL,
    score INTEGER NOT NULL,
    
    PRIMARY KEY (user_id, module_id, session_id, topic),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
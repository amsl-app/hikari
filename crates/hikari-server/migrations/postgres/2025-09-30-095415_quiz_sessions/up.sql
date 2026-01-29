CREATE TABLE quiz_sessions (
    quiz_id UUID NOT NULL,
    session_id TEXT NOT NULL,

    PRIMARY KEY (quiz_id, session_id),
    FOREIGN KEY (quiz_id) REFERENCES quiz(id) ON DELETE CASCADE
);
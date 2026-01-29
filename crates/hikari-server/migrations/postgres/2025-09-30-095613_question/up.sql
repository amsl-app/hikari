CREATE TABLE question (
    id UUID PRIMARY KEY,
    quiz_id UUID NOT NULL,
    topic TEXT NOT NULL,
    content TEXT NOT NULL,
    question TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    answered_at TIMESTAMP,
    answer TEXT,
    evaluation TEXT,
    grade INT,
    FOREIGN KEY (quiz_id) REFERENCES quiz (id) ON DELETE CASCADE
);

CREATE INDEX idx_question_quiz_id ON question (quiz_id);
CREATE TABLE quiz_question_attempt (
    quiz_id UUID NOT NULL,
    question_id UUID NOT NULL,
    attempt INTEGER NOT NULL DEFAULT 1,

    session_id VARCHAR NOT NULL,
    asked_at TIMESTAMP NULL,
    answered_at TIMESTAMP NULL,
    answer VARCHAR NULL,
    evaluation VARCHAR NULL,
    grade INTEGER NULL,
    status question_status_enum NOT NULL,
    feedback question_feedback_enum NULL,
    feedback_explanation VARCHAR NULL,

    PRIMARY KEY (quiz_id, question_id, attempt),

    FOREIGN KEY (quiz_id)
        REFERENCES quiz(id)
        ON DELETE CASCADE,

    FOREIGN KEY (question_id)
        REFERENCES question(id)
        ON DELETE CASCADE
);

INSERT INTO quiz_question_attempt (
    quiz_id,
    question_id,
    attempt,
    session_id,
    asked_at,
    answered_at,
    answer,
    evaluation,
    grade,
    status,
    feedback,
    feedback_explanation
)
SELECT
    quiz_id,
    id,
    1,
    session_id,
    created_at,
    answered_at,
    answer,
    evaluation,
    grade,
    status,
    feedback,
    feedback_explanation
FROM question;

ALTER TABLE question DROP COLUMN quiz_id;

ALTER TABLE question DROP COLUMN session_id;

ALTER TABLE question DROP COLUMN answered_at;

ALTER TABLE question DROP COLUMN answer;

ALTER TABLE question DROP COLUMN evaluation;

ALTER TABLE question DROP COLUMN grade;

ALTER TABLE question DROP COLUMN status;

ALTER TABLE question DROP COLUMN feedback;

ALTER TABLE question DROP COLUMN feedback_explanation



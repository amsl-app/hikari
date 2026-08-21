ALTER TABLE question
    ADD COLUMN quiz_id UUID;

ALTER TABLE question
    ADD COLUMN session_id VARCHAR;

ALTER TABLE question
    ADD COLUMN answered_at TIMESTAMP NULL;

ALTER TABLE question
    ADD COLUMN answer VARCHAR NULL;

ALTER TABLE question
    ADD COLUMN evaluation VARCHAR NULL;

ALTER TABLE question
    ADD COLUMN grade INTEGER NULL;

ALTER TABLE question
    ADD COLUMN status question_status_enum;

ALTER TABLE question
    ADD COLUMN feedback question_feedback_enum NULL;

ALTER TABLE question
    ADD COLUMN feedback_explanation VARCHAR NULL;

UPDATE question q
SET
    quiz_id = qqa.quiz_id,
    session_id = qqa.session_id,
    answered_at = qqa.answered_at,
    answer = qqa.answer,
    evaluation = qqa.evaluation,
    grade = qqa.grade,
    status = qqa.status,
    feedback = qqa.feedback,
    feedback_explanation = qqa.feedback_explanation
    FROM quiz_question_attempt qqa
WHERE q.id = qqa.question_id
    AND qqa.attempt = 1;

DROP TABLE quiz_question_attempt;

ALTER TABLE question
    ALTER COLUMN quiz_id SET NOT NULL;

ALTER TABLE question
    ALTER COLUMN session_id SET NOT NULL;

ALTER TABLE question
    ALTER COLUMN status SET NOT NULL;
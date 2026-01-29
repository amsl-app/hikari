ALTER TABLE question
DROP COLUMN status,
DROP COLUMN feedback,
DROP COLUMN feedback_explanation,
DROP COLUMN level;

DROP TYPE question_status_enum;

DROP TYPE question_feedback_enum;

DROP TYPE question_bloom_level_enum;
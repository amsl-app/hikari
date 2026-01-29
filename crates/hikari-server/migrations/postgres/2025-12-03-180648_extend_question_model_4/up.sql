CREATE TYPE question_type_enum AS ENUM ('text', 'multiplechoice');

ALTER TABLE question
ADD COLUMN type question_type_enum NOT NULL DEFAULT 'text',
ADD COlUMN options TEXT;
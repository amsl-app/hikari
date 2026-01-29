CREATE TYPE question_status_enum AS ENUM ('open', 'skipped', 'finished');

CREATE TYPE question_feedback_enum AS ENUM ('good', 'bad');

CREATE TYPE question_bloom_level_enum AS ENUM ('remember', 'understand', 'apply', 'analyze', 'evaluate', 'create');

ALTER TABLE question
ADD COLUMN status question_status_enum NOT NULL DEFAULT 'open',
ADD COLUMN feedback question_feedback_enum,
ADD COLUMN feedback_explanation TEXT,
ADD COLUMN level question_bloom_level_enum NOT NULL DEFAULT 'remember';
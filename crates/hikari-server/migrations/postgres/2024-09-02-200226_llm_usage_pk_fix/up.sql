ALTER TABLE llm_usage
DROP CONSTRAINT llm_usage_pkey;

ALTER TABLE llm_usage
ADD PRIMARY KEY (user_id, number);

ALTER TABLE llm_usage
ADD COLUMN step TEXT;

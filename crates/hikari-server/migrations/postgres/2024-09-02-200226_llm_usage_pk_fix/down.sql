ALTER TABLE llm_usage
DROP CONSTRAINT llm_usage_pkey;

ALTER TABLE llm_usage
ADD PRIMARY KEY (number);

ALTER TABLE llm_usage
DROP COLUMN step;

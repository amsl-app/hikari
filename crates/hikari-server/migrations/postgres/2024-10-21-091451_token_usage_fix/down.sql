ALTER TABLE llm_usage
DROP CONSTRAINT llm_usage_pkey;

ALTER TABLE llm_usage
ADD COLUMN number int;

ALTER TABLE llm_usage
ADD PRIMARY KEY (number);

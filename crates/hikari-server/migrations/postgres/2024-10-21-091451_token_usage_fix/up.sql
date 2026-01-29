ALTER TABLE llm_usage
DROP CONSTRAINT llm_usage_pkey;

ALTER TABLE llm_usage
DROP COLUMN number;

UPDATE llm_usage
SET step = 'Unknown'
WHERE step IS NULL;

ALTER TABLE llm_usage 
ALTER COLUMN step 
SET NOT NULL;



ALTER TABLE llm_usage
ADD PRIMARY KEY (user_id, step, time);



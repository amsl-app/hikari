ALTER TABLE history_session ADD COLUMN conversation_id varchar(255);
UPDATE history_session SET conversation_id = 'not set';
ALTER TABLE history_session ALTER COLUMN conversation_id SET NOT NULL;

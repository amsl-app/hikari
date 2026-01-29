ALTER TABLE history_assessment ADD COLUMN module varchar(255);
UPDATE history_assessment SET module = 'none';
ALTER TABLE history_assessment ALTER COLUMN module SET NOT NULL;

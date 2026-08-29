ALTER TABLE planner_entry ADD COLUMN module_id VARCHAR(255) DEFAULT NULL;
ALTER TABLE planner_entry ADD COLUMN session_id VARCHAR(255) DEFAULT NULL;
ALTER TABLE planner_entry DROP COLUMN milestone_id;
DROP TABLE planner_milestone;

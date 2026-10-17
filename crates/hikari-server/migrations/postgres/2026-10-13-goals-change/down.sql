ALTER TABLE goal
    ALTER COLUMN session_ids DROP DEFAULT;

ALTER TABLE goal
ALTER COLUMN session_ids TYPE UUID[]
    USING session_ids::UUID[];

ALTER TABLE goal
    ALTER COLUMN session_ids SET DEFAULT ARRAY[]::UUID[];
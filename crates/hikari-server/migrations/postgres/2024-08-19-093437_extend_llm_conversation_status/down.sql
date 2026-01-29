CREATE TYPE "conversation_status_enum_tmp" AS ENUM('open', 'closed');

ALTER TABLE llm_conversation ALTER COLUMN status DROP DEFAULT;

ALTER TABLE llm_conversation ALTER COLUMN status TYPE conversation_status_enum_tmp
    USING CASE
        WHEN status = 'closed' THEN 'closed'::conversation_status_enum_tmp
        WHEN status = 'completed' THEN 'closed'::conversation_status_enum_tmp
        ELSE 'open'::conversation_status_enum_tmp
    END;

ALTER TABLE llm_conversation ALTER COLUMN status SET DEFAULT 'open';

DROP TYPE conversation_status_enum;

ALTER TYPE conversation_status_enum_tmp RENAME TO conversation_status_enum;

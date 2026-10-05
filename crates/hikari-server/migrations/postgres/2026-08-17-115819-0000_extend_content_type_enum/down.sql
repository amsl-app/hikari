CREATE TYPE "content_type_enum_tmp" AS ENUM('text', 'payload', 'buttons');
ALTER TABLE llm_message
ALTER COLUMN content_type TYPE content_type_enum_tmp USING CASE
        WHEN content_type IN ('image', 'video') THEN 'payload'::content_type_enum_tmp
        ELSE content_type::text::content_type_enum_tmp
    END;
DROP TYPE content_type_enum;
ALTER TYPE content_type_enum_tmp
RENAME TO content_type_enum;
-- user table
-- @@@@@@@@@@@@@@
--
-- Drop all the foreign key constraints and change the type of the user_id column to UUID
ALTER TABLE access_tokens DROP CONSTRAINT access_tokens_user_id_fkey;
ALTER TABLE access_tokens ALTER COLUMN user_id TYPE VARCHAR(36) USING user_id::VARCHAR(36);

ALTER TABLE assessment_session DROP CONSTRAINT assessment_user_id_fkey;
ALTER TABLE assessment_session ALTER COLUMN user_id TYPE VARCHAR(36) USING user_id::VARCHAR(36);

-- The table was renamed
ALTER TABLE groups DROP CONSTRAINT groups_user_id_fkey;
ALTER TABLE groups ALTER COLUMN user_id TYPE VARCHAR(36) USING user_id::VARCHAR(36);

ALTER TABLE history DROP CONSTRAINT history_user_id_fkey;
ALTER TABLE history ALTER COLUMN user_id TYPE VARCHAR(36) USING user_id::VARCHAR(36);

ALTER TABLE journal_entry DROP CONSTRAINT journal_entry_user_id_fkey;
ALTER TABLE journal_entry ALTER COLUMN user_id TYPE VARCHAR(36) USING user_id::VARCHAR(36);

ALTER TABLE journal_summary DROP CONSTRAINT journal_summary_user_id_fkey;
ALTER TABLE journal_summary ALTER COLUMN user_id TYPE VARCHAR(36) USING user_id::VARCHAR(36);

ALTER TABLE module_assessment DROP CONSTRAINT module_assessment_user_id_fkey;
ALTER TABLE module_assessment ALTER COLUMN user_id TYPE VARCHAR(36) USING user_id::VARCHAR(36);

ALTER TABLE oidc_mapping DROP CONSTRAINT oidc_mapping_user_id_fkey;
ALTER TABLE oidc_mapping ALTER COLUMN user_id TYPE VARCHAR(36) USING user_id::VARCHAR(36);

ALTER TABLE reminders DROP CONSTRAINT reminders_user_id_fkey;
ALTER TABLE reminders ALTER COLUMN user_id TYPE VARCHAR(36) USING user_id::VARCHAR(36);

-- The table was renamed
ALTER TABLE tag DROP CONSTRAINT tag_user_id_fkey;
ALTER TABLE tag ALTER COLUMN user_id TYPE VARCHAR(36) USING user_id::VARCHAR(36);

ALTER TABLE user_configs DROP CONSTRAINT user_configs_user_id_fkey;
ALTER TABLE user_configs ALTER COLUMN user_id TYPE VARCHAR(36) USING user_id::VARCHAR(36);

ALTER TABLE user_handle DROP CONSTRAINT user_handle_user_id_fkey;
ALTER TABLE user_handle ALTER COLUMN user_id TYPE VARCHAR(36) USING user_id::VARCHAR(36);

ALTER TABLE user_modules DROP CONSTRAINT user_modules_user_id_fkey;
ALTER TABLE user_modules ALTER COLUMN user_id TYPE VARCHAR(36) USING user_id::VARCHAR(36);

-- Change the type of the actual id column on the users table
ALTER TABLE users ALTER COLUMN id TYPE VARCHAR(36) USING id::VARCHAR(36);

-- Add the foreign key constraints back
ALTER TABLE access_tokens ADD CONSTRAINT access_tokens_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE assessment_session ADD CONSTRAINT assessment_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE groups ADD CONSTRAINT affiliation_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE history ADD CONSTRAINT history_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE journal_entry ADD CONSTRAINT journal_entry_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE journal_summary ADD CONSTRAINT journal_summary_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE module_assessment ADD CONSTRAINT module_assessment_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE oidc_mapping ADD CONSTRAINT oidc_mapping_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE reminders ADD CONSTRAINT reminders_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE tag ADD CONSTRAINT journal_focus_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE user_configs ADD CONSTRAINT user_configs_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE user_handle ADD CONSTRAINT user_handle_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE user_modules ADD CONSTRAINT user_modules_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- Rename to user table to users because user is a reserved word in postgres
ALTER TABLE "users" RENAME TO "user";

-- answer table
-- @@@@@@@@@@@@@@
--
ALTER TABLE answer DROP CONSTRAINT answer_pkey;
ALTER TABLE answer RENAME COLUMN question TO answer_id;
ALTER TABLE answer RENAME COLUMN assessment_session_id TO assessment_id;
ALTER TABLE answer ADD COLUMN id VARCHAR(36);
UPDATE answer SET id = gen_random_uuid()::VARCHAR(36);
ALTER TABLE answer ADD CONSTRAINT answer_pkey PRIMARY KEY (id);
ALTER TABLE answer ADD CONSTRAINT answer_assessment_id_answer_id_key UNIQUE (assessment_id, answer_id);

-- assessment table
-- @@@@@@@@@@@@@@
--
ALTER TABLE assessment_session RENAME TO assessment;

-- Drop all the foreign key constraints and change the type of the assessment_id column to UUID
ALTER TABLE answer DROP CONSTRAINT answer_assessment_id_fkey;
ALTER TABLE answer ALTER COLUMN assessment_id TYPE VARCHAR(36) USING assessment_id::VARCHAR(36);

ALTER TABLE history_assessment DROP CONSTRAINT history_assessment_session_id_fkey;
ALTER TABLE history_assessment ALTER COLUMN assessment_session_id TYPE VARCHAR(36) USING assessment_session_id::VARCHAR(36);

ALTER TABLE module_assessment DROP CONSTRAINT module_assessment_last_pre_fkey;
ALTER TABLE module_assessment ALTER COLUMN last_pre TYPE VARCHAR(36) USING last_pre::VARCHAR(36);

ALTER TABLE module_assessment DROP CONSTRAINT module_assessment_last_post_fkey;
ALTER TABLE module_assessment ALTER COLUMN last_post TYPE VARCHAR(36) USING last_post::VARCHAR(36);

ALTER TABLE assessment RENAME assessment TO assessment_id;

-- Change the type of the actual id column on the assessment table
ALTER TABLE assessment ALTER COLUMN id TYPE VARCHAR(36) USING id::VARCHAR(36);

-- Add the foreign key constraints back
ALTER TABLE answer ADD CONSTRAINT answer_assessment_id_fkey FOREIGN KEY (assessment_id) REFERENCES assessment(id) ON DELETE CASCADE;
ALTER TABLE history_assessment ADD CONSTRAINT history_assessment_session_id_fkey FOREIGN KEY (assessment_session_id) REFERENCES assessment(id) ON DELETE CASCADE;
ALTER TABLE module_assessment ADD CONSTRAINT module_assessment_last_pre_fkey FOREIGN KEY (last_pre) REFERENCES assessment(id) ON DELETE SET NULL;
ALTER TABLE module_assessment ADD CONSTRAINT module_assessment_last_post_fkey FOREIGN KEY (last_post) REFERENCES assessment(id) ON DELETE SET NULL;

-- groups table
-- @@@@@@@@@@@@@@
--
ALTER TABLE groups DROP CONSTRAINT groups_pkey;
ALTER TABLE groups ADD COLUMN id VARCHAR(36);
UPDATE groups SET id = gen_random_uuid()::VARCHAR(36);
CREATE UNIQUE INDEX affiliation_user_id ON groups (user_id, value);
ALTER TABLE groups ADD CONSTRAINT affiliation_pkey PRIMARY KEY (id);

-- history table
-- @@@@@@@@@@@@@@
--
-- Drop all the foreign key constraints and change the type of the history_id column to UUID
ALTER TABLE history_assessment DROP CONSTRAINT history_assessment_history_id_fkey;
ALTER TABLE history_assessment ALTER COLUMN history_id TYPE VARCHAR(36) USING history_id::VARCHAR(36);

ALTER TABLE history_modules DROP CONSTRAINT history_modules_history_id_fkey;
ALTER TABLE history_modules ALTER COLUMN history_id TYPE VARCHAR(36) USING history_id::VARCHAR(36);

ALTER TABLE history_session DROP CONSTRAINT history_session_history_id_fkey;
ALTER TABLE history_session ALTER COLUMN history_id TYPE VARCHAR(36) USING history_id::VARCHAR(36);

-- Change the type of the actual id column on the history table
ALTER TABLE history ALTER COLUMN id TYPE VARCHAR(36) USING id::VARCHAR(36);

-- Add the foreign key constraints back
ALTER TABLE history_assessment ADD CONSTRAINT history_assessment_history_id_fkey FOREIGN KEY (history_id) REFERENCES history(id) ON DELETE CASCADE;
ALTER TABLE history_modules ADD CONSTRAINT history_modules_history_id_fkey FOREIGN KEY (history_id) REFERENCES history(id) ON DELETE CASCADE;
ALTER TABLE history_session ADD CONSTRAINT history_session_history_id_fkey FOREIGN KEY (history_id) REFERENCES history(id) ON DELETE CASCADE;

-- history_assessment table
-- @@@@@@@@@@@@@@
--
ALTER TABLE history_assessment DROP CONSTRAINT history_assessment_pkey;
ALTER TABLE history_assessment ADD COLUMN id VARCHAR(36);
UPDATE history_assessment SET id = gen_random_uuid()::VARCHAR(36);
ALTER TABLE history_session ALTER COLUMN conversation_id TYPE VARCHAR(255) USING conversation_id::VARCHAR(255);
ALTER TABLE history_assessment RENAME COLUMN assessment_session_id TO session_id;
ALTER TABLE history_assessment ADD CONSTRAINT history_assessment_pkey PRIMARY KEY (id);

-- history_modules table
-- @@@@@@@@@@@@@@
--
ALTER TABLE history_modules DROP CONSTRAINT history_modules_pkey;
ALTER TABLE history_modules ADD COLUMN id VARCHAR(36);
UPDATE history_modules SET id = gen_random_uuid()::VARCHAR(36);
ALTER TABLE history_modules ADD CONSTRAINT history_modules_pkey PRIMARY KEY (id);

-- history_session table
-- @@@@@@@@@@@@@@
--
ALTER TABLE history_session DROP CONSTRAINT history_session_pkey;
ALTER TABLE history_session ADD COLUMN id VARCHAR(36);
UPDATE history_session SET id = gen_random_uuid()::VARCHAR(36);
ALTER TABLE history_session ADD CONSTRAINT history_session_pkey PRIMARY KEY (id);

-- module_assessment table
-- @@@@@@@@@@@@@@
--
ALTER TABLE module_assessment DROP CONSTRAINT module_assessment_pkey;
ALTER TABLE module_assessment ADD COLUMN id VARCHAR(36);
UPDATE module_assessment SET id = gen_random_uuid()::VARCHAR(36);
ALTER TABLE module_assessment RENAME COLUMN module TO module_id;
ALTER TABLE module_assessment ADD CONSTRAINT module_assessment_pkey PRIMARY KEY ("id");
CREATE UNIQUE INDEX module_assessment_index ON module_assessment (user_id, module_id);

-- reminders table
-- @@@@@@@@@@@@@@
--
ALTER TABLE reminders ALTER COLUMN id TYPE VARCHAR(36) USING id::VARCHAR(36);

-- user_configs table
-- @@@@@@@@@@@@@@
--
ALTER TABLE user_configs DROP CONSTRAINT user_configs_pkey;
ALTER TABLE user_configs ADD COLUMN id VARCHAR(36);
UPDATE user_configs SET id = gen_random_uuid()::VARCHAR(36);
ALTER TABLE user_configs ADD CONSTRAINT uc_user_id_key UNIQUE (user_id, key);
ALTER TABLE user_configs ADD CONSTRAINT user_configs_pkey PRIMARY KEY (id);

-- user_modules table
-- @@@@@@@@@@@@@@
--
-- Alter the table
ALTER TABLE user_modules DROP CONSTRAINT user_modules_pkey;
ALTER TABLE user_modules ADD COLUMN id VARCHAR(36);
UPDATE user_modules SET id = gen_random_uuid()::VARCHAR(36);
ALTER TABLE user_modules ALTER COLUMN last_conv_id TYPE VARCHAR(36) USING last_conv_id::VARCHAR(36);
CREATE UNIQUE INDEX user_modules_index ON user_modules ("user_id", "module", "session");
ALTER TABLE user_modules ADD CONSTRAINT user_modules_pkey PRIMARY KEY (id);

-- Update user table accordingly
UPDATE "user" SET (current_module) = (
    SELECT id FROM user_modules WHERE "user".id = user_modules.user_id AND "user".current_module = user_modules.module AND "user".current_session = user_modules.session
);
ALTER TABLE "user" ALTER COLUMN current_module TYPE VARCHAR(36);
ALTER TABLE "user" DROP COLUMN current_session;

-- Add the foreign key constraints back
ALTER TABLE "user" ADD CONSTRAINT user_current_module_fkey FOREIGN KEY (current_module) REFERENCES user_modules (id) ON DELETE SET NULL;

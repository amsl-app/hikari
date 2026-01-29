-- user table
-- @@@@@@@@@@@@@@
--
-- Rename to user table to users because user is a reserved word in postgres
ALTER TABLE "user" RENAME TO users;

-- Drop all the foreign key constraints and change the type of the user_id column to UUID
ALTER TABLE access_tokens DROP CONSTRAINT access_tokens_user_id_fkey;
ALTER TABLE access_tokens ALTER COLUMN user_id TYPE UUID USING user_id::UUID;

ALTER TABLE assessment DROP CONSTRAINT assessment_user_id_fkey;
ALTER TABLE assessment ALTER COLUMN user_id TYPE UUID USING user_id::UUID;

-- The table was renamed
ALTER TABLE groups  DROP CONSTRAINT affiliation_user_id_fkey;
ALTER TABLE groups ALTER COLUMN user_id TYPE UUID USING user_id::UUID;

ALTER TABLE history DROP CONSTRAINT history_user_id_fkey;
ALTER TABLE history ALTER COLUMN user_id TYPE UUID USING user_id::UUID;

ALTER TABLE journal_entry DROP CONSTRAINT journal_entry_user_id_fkey;
ALTER TABLE journal_entry ALTER COLUMN user_id TYPE UUID USING user_id::UUID;

ALTER TABLE journal_summary DROP CONSTRAINT journal_summary_user_id_fkey;
ALTER TABLE journal_summary ALTER COLUMN user_id TYPE UUID USING user_id::UUID;

ALTER TABLE module_assessment DROP CONSTRAINT module_assessment_user_id_fkey;
ALTER TABLE module_assessment ALTER COLUMN user_id TYPE UUID USING user_id::UUID;

ALTER TABLE oidc_mapping DROP CONSTRAINT oidc_mapping_user_id_fkey;
ALTER TABLE oidc_mapping ALTER COLUMN user_id TYPE UUID USING user_id::UUID;

ALTER TABLE reminders DROP CONSTRAINT reminders_user_id_fkey;
ALTER TABLE reminders ALTER COLUMN user_id TYPE UUID USING user_id::UUID;

-- The table was renamed
ALTER TABLE tag DROP CONSTRAINT journal_focus_user_id_fkey;
ALTER TABLE tag ALTER COLUMN user_id TYPE UUID USING user_id::UUID;

ALTER TABLE user_configs DROP CONSTRAINT user_configs_user_id_fkey;
ALTER TABLE user_configs ALTER COLUMN user_id TYPE UUID USING user_id::UUID;

ALTER TABLE user_handle DROP CONSTRAINT user_handle_user_id_fkey;
ALTER TABLE user_handle ALTER COLUMN user_id TYPE UUID USING user_id::UUID;

ALTER TABLE user_modules DROP CONSTRAINT user_modules_user_id_fkey;
ALTER TABLE user_modules ALTER COLUMN user_id TYPE UUID USING user_id::UUID;

-- Change the type of the actual id column on the users table
ALTER TABLE users ALTER COLUMN id TYPE UUID USING id::UUID;

-- Add the foreign key constraints back
ALTER TABLE access_tokens ADD CONSTRAINT access_tokens_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE assessment ADD CONSTRAINT assessment_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE groups ADD CONSTRAINT groups_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE history ADD CONSTRAINT history_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE journal_entry ADD CONSTRAINT journal_entry_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE journal_summary ADD CONSTRAINT journal_summary_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE module_assessment ADD CONSTRAINT module_assessment_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE oidc_mapping ADD CONSTRAINT oidc_mapping_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE reminders ADD CONSTRAINT reminders_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE tag ADD CONSTRAINT tag_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE user_configs ADD CONSTRAINT user_configs_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE user_handle ADD CONSTRAINT user_handle_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE user_modules ADD CONSTRAINT user_modules_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;


-- answer table
-- @@@@@@@@@@@@@@
--
ALTER TABLE answer DROP CONSTRAINT answer_pkey;
ALTER TABLE answer DROP CONSTRAINT answer_assessment_id_answer_id_key;
ALTER TABLE answer RENAME COLUMN answer_id TO question;
ALTER TABLE answer RENAME COLUMN assessment_id TO assessment_session_id;
ALTER TABLE answer DROP COLUMN id;
ALTER TABLE answer ADD CONSTRAINT answer_pkey PRIMARY KEY (assessment_session_id, question);

-- assessment table
-- @@@@@@@@@@@@@@
--
-- Drop all the foreign key constraints and change the type of the assessment_id column to UUID
ALTER TABLE answer DROP CONSTRAINT answer_assessment_id_fkey;
ALTER TABLE answer ALTER COLUMN assessment_session_id TYPE UUID USING assessment_session_id::UUID;

ALTER TABLE history_assessment DROP CONSTRAINT history_assessment_session_id_fkey;
ALTER TABLE history_assessment ALTER COLUMN session_id TYPE UUID USING session_id::UUID;

ALTER TABLE module_assessment DROP CONSTRAINT module_assessment_last_pre_fkey;
ALTER TABLE module_assessment ALTER COLUMN last_pre TYPE UUID USING last_pre::UUID;

ALTER TABLE module_assessment DROP CONSTRAINT module_assessment_last_post_fkey;
ALTER TABLE module_assessment ALTER COLUMN last_post TYPE UUID USING last_post::UUID;

ALTER TABLE assessment RENAME assessment_id TO assessment;

-- Change the type of the actual id column on the assessment table
ALTER TABLE assessment ALTER COLUMN id TYPE UUID USING id::UUID;

-- Add the foreign key constraints back
ALTER TABLE answer ADD CONSTRAINT answer_assessment_id_fkey FOREIGN KEY (assessment_session_id) REFERENCES assessment(id) ON DELETE CASCADE;
ALTER TABLE history_assessment ADD CONSTRAINT history_assessment_session_id_fkey FOREIGN KEY (session_id) REFERENCES assessment(id) ON DELETE CASCADE;
ALTER TABLE module_assessment ADD CONSTRAINT module_assessment_last_pre_fkey FOREIGN KEY (last_pre) REFERENCES assessment(id) ON DELETE SET NULL;
ALTER TABLE module_assessment ADD CONSTRAINT module_assessment_last_post_fkey FOREIGN KEY (last_post) REFERENCES assessment(id) ON DELETE SET NULL;

ALTER TABLE assessment RENAME TO assessment_session;

-- groups table
-- @@@@@@@@@@@@@@
--
ALTER TABLE groups DROP CONSTRAINT affiliation_pkey;
ALTER TABLE groups DROP COLUMN id;
DROP INDEX affiliation_user_id;
ALTER TABLE groups ADD CONSTRAINT groups_pkey PRIMARY KEY (user_id, value);

-- history table
-- @@@@@@@@@@@@@@
--
-- Drop all the foreign key constraints and change the type of the history_id column to UUID
ALTER TABLE history_assessment DROP CONSTRAINT history_assessment_history_id_fkey;
ALTER TABLE history_assessment ALTER COLUMN history_id TYPE UUID USING history_id::UUID;

ALTER TABLE history_modules DROP CONSTRAINT history_modules_history_id_fkey;
ALTER TABLE history_modules ALTER COLUMN history_id TYPE UUID USING history_id::UUID;

ALTER TABLE history_session DROP CONSTRAINT history_session_history_id_fkey;
ALTER TABLE history_session ALTER COLUMN history_id TYPE UUID USING history_id::UUID;

-- Change the type of the actual id column on the history table
ALTER TABLE history ALTER COLUMN id TYPE UUID USING id::UUID;

-- Add the foreign key constraints back
ALTER TABLE history_assessment ADD CONSTRAINT history_assessment_history_id_fkey FOREIGN KEY (history_id) REFERENCES history(id) ON DELETE CASCADE;
ALTER TABLE history_modules ADD CONSTRAINT history_modules_history_id_fkey FOREIGN KEY (history_id) REFERENCES history(id) ON DELETE CASCADE;
ALTER TABLE history_session ADD CONSTRAINT history_session_history_id_fkey FOREIGN KEY (history_id) REFERENCES history(id) ON DELETE CASCADE;

-- history_assessment table
-- @@@@@@@@@@@@@@
--
ALTER TABLE history_assessment DROP CONSTRAINT history_assessment_pkey;
ALTER TABLE history_assessment DROP COLUMN id;
ALTER TABLE history_assessment RENAME COLUMN session_id TO assessment_session_id;
ALTER TABLE history_assessment ADD CONSTRAINT history_assessment_pkey PRIMARY KEY (history_id);

-- history_modules table
-- @@@@@@@@@@@@@@
--
ALTER TABLE history_modules DROP CONSTRAINT history_modules_pkey;
ALTER TABLE history_modules DROP COLUMN id;
ALTER TABLE history_modules ADD CONSTRAINT history_modules_pkey PRIMARY KEY (history_id);

-- history_session table
-- @@@@@@@@@@@@@@
--
ALTER TABLE history_session DROP CONSTRAINT history_session_pkey;
ALTER TABLE history_session DROP COLUMN id;
ALTER TABLE history_session ALTER COLUMN conversation_id TYPE UUID USING conversation_id::UUID;
ALTER TABLE history_session ADD CONSTRAINT history_session_pkey PRIMARY KEY (history_id);

-- module_assessment table
-- @@@@@@@@@@@@@@
--
ALTER TABLE module_assessment DROP CONSTRAINT module_assessment_pkey;
ALTER TABLE module_assessment DROP COLUMN id;
DROP INDEX module_assessment_index;
ALTER TABLE module_assessment RENAME COLUMN module_id TO module;
ALTER TABLE module_assessment ADD CONSTRAINT module_assessment_pkey PRIMARY KEY ("user_id", "module");

-- reminders table
-- @@@@@@@@@@@@@@
--
ALTER TABLE reminders ALTER COLUMN id TYPE UUID USING id::UUID;

-- user_configs table
-- @@@@@@@@@@@@@@
--
ALTER TABLE user_configs DROP CONSTRAINT user_configs_pkey;
ALTER TABLE user_configs DROP COLUMN id;
ALTER TABLE user_configs DROP CONSTRAINT uc_user_id_key;
ALTER TABLE user_configs ADD CONSTRAINT user_configs_pkey PRIMARY KEY ("user_id", "key");

-- user_modules table
-- @@@@@@@@@@@@@@
--
-- Drop all the foreign key constraints and change the type of the history_id column to UUID
ALTER TABLE users DROP CONSTRAINT user_current_module_fkey;
ALTER TABLE users ALTER COLUMN current_module TYPE VARCHAR(255);
ALTER TABLE users ADD COLUMN current_session VARCHAR(255);
UPDATE users SET (current_module, current_session) = (
    SELECT module, session FROM user_modules WHERE users.current_module = user_modules.id
);

-- Alter the table
ALTER TABLE user_modules DROP CONSTRAINT user_modules_pkey;
ALTER TABLE user_modules DROP COLUMN id;
DROP INDEX user_modules_index;
ALTER TABLE user_modules ALTER COLUMN last_conv_id TYPE UUID USING last_conv_id::UUID;
ALTER TABLE user_modules ADD CONSTRAINT user_modules_pkey PRIMARY KEY ("user_id", "module", "session");

-- We don't add the constraint back because a users current_session is not strictly related to the status of the session

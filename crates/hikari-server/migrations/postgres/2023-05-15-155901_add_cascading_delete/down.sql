ALTER TABLE user_configs
DROP CONSTRAINT user_configs_user_id_fkey,
ADD CONSTRAINT user_configs_user_id_fkey FOREIGN KEY (user_id) REFERENCES "user" (id);

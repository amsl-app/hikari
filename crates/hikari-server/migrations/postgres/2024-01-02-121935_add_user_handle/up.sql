CREATE TABLE user_handle
(
    handle     BYTEA PRIMARY KEY                                    NOT NULL,
    user_id    VARCHAR(36) REFERENCES "user" (id) ON DELETE CASCADE NOT NULL,
    created_at TIMESTAMP                                            NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX user_handle_user_id_idx ON user_handle (user_id);

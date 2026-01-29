CREATE TABLE user_modules
(
    id           varchar(36) PRIMARY KEY NOT NULL,
    user_id      varchar(36)             NOT NULL,
    module       varchar(255)            NOT NULL,
    session      varchar(255)            NOT NULL,
    status       smallint                NOT NULL DEFAULT 0,
    bot_id       varchar(255)            NOT NULL,
    last_conv_id varchar(36),

    FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE
);

ALTER TABLE "user" ADD COLUMN current_module varchar(36) REFERENCES user_modules(id) ON DELETE SET NULL;

CREATE UNIQUE INDEX user_modules_index ON user_modules (user_id, module, session);

DROP TABLE selected_module;
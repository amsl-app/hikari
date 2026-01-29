CREATE TABLE selected_module
(
    id              varchar(36) PRIMARY KEY NOT NULL,
    user_id         varchar(36)             NOT NULL UNIQUE,
    current_module  varchar(255),
    current_session varchar(255),

    FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE
);

ALTER TABLE "user" DROP COLUMN current_module;

/*INSERT INTO selected_module(id, user_id, current_module, current_session) SELECT id, user_id, module, session FROM user_modules;*/

DROP TABLE user_modules;
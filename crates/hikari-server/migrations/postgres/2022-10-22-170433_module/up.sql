CREATE TABLE selected_module
(
    id      varchar(36) PRIMARY KEY NOT NULL,
    user_id varchar(36)             NOT NULL UNIQUE,
    current_module  varchar(255),
    current_session varchar(255),

    FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE
);
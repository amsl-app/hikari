CREATE TABLE history
(
    id        varchar(36) PRIMARY KEY NOT NULL,
    user_id   varchar(36)             NOT NULL,
    completed timestamp               NOT NULL,
    FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE
);

CREATE INDEX history_user_id ON history (user_id);

CREATE TABLE history_modules
(
    id         varchar(36) PRIMARY KEY NOT NULL,
    history_id varchar(36)             NOT NULL,
    module     varchar(255)            NOT NULL UNIQUE,
    FOREIGN KEY (history_id) REFERENCES history (id) ON DELETE CASCADE
);

CREATE TABLE history_session
(
    id         varchar(36) PRIMARY KEY NOT NULL,
    history_id varchar(36)             NOT NULL,
    module     varchar(255)            NOT NULL,
    session    varchar(255)            NOT NULL,
    FOREIGN KEY (history_id) REFERENCES history (id) ON DELETE CASCADE
);

CREATE TABLE history_assessment
(
    id         varchar(36) PRIMARY KEY NOT NULL,
    history_id varchar(36)             NOT NULL,
    type_id    smallint                NOT NULL,
    session_id varchar(36)             NOT NULL,
    FOREIGN KEY (history_id) REFERENCES history (id) ON DELETE CASCADE,
    FOREIGN KEY (session_id) REFERENCES assessment (id) ON DELETE CASCADE
);

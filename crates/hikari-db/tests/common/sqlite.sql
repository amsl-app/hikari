--TODO in final deployment (in postgres migration) make primary keys autoincrement (sqlite does a_i for primary key)
--TODO change to serial change primary keys to serial instead of integer + autoincrement

-- User stuff

CREATE TABLE users
(
    id              BLOB PRIMARY KEY NOT NULL,
    /*email          varchar(255)            NOT NULL UNIQUE,*/
    name            varchar(255),
    birthday        DATE,
    subject         varchar(255),
    semester        smallint,
    gender          varchar(255),
    current_module  varchar(255),
    current_session varchar(255),
    onboarding      BOOLEAN          NOT NULL DEFAULT false
);

CREATE TABLE access_tokens
(
    id           INTEGER PRIMARY KEY NOT NULL,
    user_id      BLOB                NOT NULL UNIQUE,
    --  TODO (LOW) shorten the varchar
    access_token varchar(255)        NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE INDEX index_access_tokens ON access_tokens (access_token);

CREATE TABLE user_handle
(
    handle     BLOB PRIMARY KEY                               NOT NULL,
    user_id    BLOB REFERENCES "users" (id) ON DELETE CASCADE NOT NULL,
    created_at TIMESTAMP                                      NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX user_handle_user_id_idx ON user_handle (user_id);

CREATE TABLE oidc_mapping
(
    id       INTEGER PRIMARY KEY NOT NULL,
    user_id  BLOB                NOT NULL,
    -- TODO (LOW) shorten the varchar
    oidc_sub varchar(255)        NOT NULL UNIQUE,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE TABLE groups
(
    user_id BLOB REFERENCES "users" (id) ON DELETE CASCADE NOT NULL,
    value   varchar(255)                                   NOT NULL,
    PRIMARY KEY (user_id, value)
);

-- User Config

CREATE TABLE user_configs
(
    user_id BLOB          not null,
    key     varchar(255)  not null,
    value   varchar(1024) not null,
    PRIMARY KEY (user_id, key),
    FOREIGN KEY (user_id) REFERENCES "users" (id) ON DELETE CASCADE
);

-- User module / session bookkeeping

CREATE TABLE "module_status"
(
    user_id    BLOB         NOT NULL,
    module     VARCHAR(255) NOT NULL,
    status     SMALLINT     NOT NULL DEFAULT 0,
    completion TIMESTAMP,

    PRIMARY KEY (user_id, module),
    FOREIGN KEY (user_id) REFERENCES "users" (id) ON DELETE CASCADE
);

CREATE TABLE session_status
(
    user_id      BLOB         NOT NULL,
    module       varchar(255) NOT NULL,
    session      varchar(255) NOT NULL,
    status       smallint     NOT NULL DEFAULT 0,
    bot_id       varchar(255),
    last_conv_id BLOB,
    completion   DATETIME,

    PRIMARY KEY (user_id, module, session),
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

-- Assessments

CREATE TABLE assessment_session
(
    id         BLOB PRIMARY KEY NOT NULL,
    user_id    BLOB             NOT NULL,
    status     smallint         NOT NULL,
    assessment varchar(255)     NOT NULL,
    completed  DATETIME,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE TABLE answer
(
    assessment_session_id BLOB         NOT NULL,
    question              varchar(255) NOT NULL,
    answer_type           smallint     NOT NULL,
    data                  TEXT         NOT NULL,
    PRIMARY KEY (assessment_session_id, question),
    FOREIGN KEY (assessment_session_id) REFERENCES assessment_session (id) ON DELETE CASCADE
);

CREATE TABLE module_assessment
(
    user_id   BLOB NOT NULL,
    module    BLOB NOT NULL,
    last_pre  BLOB,
    last_post BLOB,

    PRIMARY KEY ("user_id", "module"),
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    FOREIGN KEY (last_pre) REFERENCES assessment_session (id) ON DELETE SET NULL,
    FOREIGN KEY (last_post) REFERENCES assessment_session (id) ON DELETE SET NULL
);

-- History

CREATE TABLE history
(
    id        BLOB PRIMARY KEY NOT NULL,
    user_id   BLOB             NOT NULL,
    completed timestamp        NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE INDEX history_user_id ON history (user_id);

CREATE TABLE history_modules
(
    history_id BLOB PRIMARY KEY NOT NULL,
    module     varchar(255)     NOT NULL,
    FOREIGN KEY (history_id) REFERENCES history (id) ON DELETE CASCADE
);

CREATE TABLE history_session
(
    history_id      BLOB PRIMARY KEY NOT NULL,
    module          varchar(255)     NOT NULL,
    session         varchar(255)     NOT NULL,
    conversation_id BLOB,
    FOREIGN KEY (history_id) REFERENCES history (id) ON DELETE CASCADE
);

CREATE TABLE history_assessment
(
    history_id            BLOB PRIMARY KEY NOT NULL,
    assessment_session_id BLOB             NOT NULL,
    type_id               smallint         NOT NULL,
    module                varchar(255)     NOT NULL,
    FOREIGN KEY (history_id) REFERENCES history (id) ON DELETE CASCADE,
    FOREIGN KEY (assessment_session_id) REFERENCES assessment_session (id) ON DELETE CASCADE
);

-- Journaling tables

CREATE TABLE "journal_entry"
(
    id         BLOB PRIMARY KEY NOT NULL,
    user_id    BLOB             NOT NULL,
    mood       REAL,
    title      TEXT,
    created_at TEXT             NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT             NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES "users" (id) ON DELETE CASCADE
);

CREATE TABLE "journal_content"
(
    id               BLOB PRIMARY KEY NOT NULL,
    journal_entry_id BLOB             NOT NULL,
    title            TEXT,
    content          TEXT             NOT NULL,
    created_at       TEXT             NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at       TEXT             NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (journal_entry_id) REFERENCES "journal_entry" (id) ON DELETE CASCADE
);

CREATE TABLE "tag"
(
    id      BLOB PRIMARY KEY NOT NULL,
    kind    TEXT             NOT NULL,
    name    TEXT             NOT NULL,
    icon    TEXT             NOT NULL,
    user_id BLOB,
    hidden  INTEGER          NOT NULL,
    FOREIGN KEY (user_id) REFERENCES "users" (id) ON DELETE CASCADE
);
CREATE INDEX tag_kind ON tag (kind);

CREATE TABLE "journal_entry_tag"
(
    journal_entry_id BLOB NOT NULL,
    tag_id           BLOB NOT NULL,
    PRIMARY KEY (journal_entry_id, tag_id),
    FOREIGN KEY (journal_entry_id) REFERENCES "journal_entry" (id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES "tag" (id) ON DELETE CASCADE
);

CREATE TABLE "journal_prompt"
(
    id     BLOB PRIMARY KEY NOT NULL,
    prompt TEXT             NOT NULL,
    UNIQUE (prompt)
);

CREATE TABLE "journal_entry_journal_prompt"
(
    journal_entry_id  BLOB NOT NULL,
    journal_prompt_id BLOB NOT NULL,
    PRIMARY KEY (journal_entry_id, journal_prompt_id),
    FOREIGN KEY (journal_entry_id) REFERENCES "journal_entry" (id) ON DELETE CASCADE,
    FOREIGN KEY (journal_prompt_id) REFERENCES "journal_prompt" (id) ON DELETE CASCADE
);

CREATE TABLE "journal_summary"
(
    id         BLOB PRIMARY KEY NOT NULL,
    user_id    BLOB             NOT NULL,
    key        BLOB             NOT NULL,
    created_at TEXT             NOT NULL DEFAULT CURRENT_TIMESTAMP,
    summary    TEXT             NOT NULL,
    FOREIGN KEY (user_id) REFERENCES "users" (id) ON DELETE CASCADE
);

CREATE TABLE "journal_topic"
(
    id                 BLOB PRIMARY KEY NOT NULL,
    journal_summary_id BLOB             NOT NULL,
    topic              TEXT             NOT NULL,
    summary            TEXT             NOT NULL,
    FOREIGN KEY (journal_summary_id) REFERENCES "journal_summary" (id) ON DELETE CASCADE
);

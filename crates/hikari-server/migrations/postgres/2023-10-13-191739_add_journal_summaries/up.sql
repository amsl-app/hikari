CREATE TABLE "journal_summary"
(
    id         UUID PRIMARY KEY NOT NULL,
    user_id    VARCHAR(36)      NOT NULL,
    key        BYTEA            NOT NULL,
    created_at timestamp        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    summary    TEXT             NOT NULL,
    FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE
);

CREATE TABLE "journal_topic"
(
    id                 UUID PRIMARY KEY NOT NULL,
    journal_summary_id UUID             NOT NULL,
    topic              TEXT             NOT NULL,
    summary            TEXT             NOT NULL,
    FOREIGN KEY (journal_summary_id) REFERENCES "journal_summary" (id) ON DELETE CASCADE
);

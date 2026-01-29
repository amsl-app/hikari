CREATE TABLE "journal_entry"
(
    id         UUID PRIMARY KEY         NOT NULL,
    user_id    VARCHAR(36)              NOT NULL,
    mood       REAL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE
);

CREATE TABLE "journal_content"
(
    id               UUID PRIMARY KEY         NOT NULL,
    journal_entry_id UUID                     NOT NULL,
    content          TEXT                     NOT NULL,
    created_at       TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at       TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (journal_entry_id) REFERENCES "journal_entry" (id) ON DELETE CASCADE
);

CREATE TABLE "journal_focus"
(
    id      UUID PRIMARY KEY NOT NULL,
    name    TEXT             NOT NULL,
    icon    TEXT             NOT NULL,
    user_id VARCHAR(36),
    hidden  BOOLEAN          NOT NULL,
    FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE
);

CREATE TABLE "journal_entry_journal_focus"
(
    journal_entry_id UUID NOT NULL,
    journal_focus_id UUID NOT NULL,
    PRIMARY KEY (journal_entry_id, journal_focus_id),
    FOREIGN KEY (journal_entry_id) REFERENCES "journal_entry" (id) ON DELETE CASCADE,
    FOREIGN KEY (journal_focus_id) REFERENCES "journal_focus" (id) ON DELETE CASCADE
);

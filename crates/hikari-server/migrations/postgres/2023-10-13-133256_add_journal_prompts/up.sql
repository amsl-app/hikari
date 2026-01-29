CREATE TABLE "journal_prompt"
(
    id      UUID PRIMARY KEY NOT NULL,
    prompt    TEXT             NOT NULL
);

CREATE UNIQUE INDEX journal_prompt_index ON journal_prompt (prompt);

CREATE TABLE "journal_entry_journal_prompt"
(
    journal_entry_id UUID NOT NULL,
    journal_prompt_id UUID NOT NULL,
    PRIMARY KEY (journal_entry_id, journal_prompt_id),
    FOREIGN KEY (journal_entry_id) REFERENCES "journal_entry" (id) ON DELETE CASCADE,
    FOREIGN KEY (journal_prompt_id) REFERENCES "journal_prompt" (id) ON DELETE CASCADE
);

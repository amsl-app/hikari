CREATE TABLE reminders
(
    id      varchar(255) PRIMARY KEY NOT NULL,
    user_id varchar(36)              NOT NULL,
    message varchar(255)             NOT NULL,
    date    INTEGER                  NOT NULL,
    FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE
);

CREATE INDEX reminders_date ON reminders (date);
CREATE TABLE affiliation
(
    id      varchar(36) PRIMARY KEY                              NOT NULL,
    user_id varchar(36) REFERENCES "user" (id) ON DELETE CASCADE NOT NULL,
    value   varchar(255)                                         NOT NULL
);
CREATE INDEX affiliation_user_id ON affiliation (user_id);
ALTER TABLE access_tokens
    ADD CONSTRAINT unique_user_id UNIQUE (user_id);

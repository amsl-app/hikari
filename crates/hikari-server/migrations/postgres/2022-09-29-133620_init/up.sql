CREATE TABLE "user"
(
    id    varchar(36) PRIMARY KEY NOT NULL,
    email varchar(255)            NOT NULL UNIQUE,
    "name"  varchar(255)
);

CREATE TABLE access_tokens
(
    id           SERIAL PRIMARY KEY,
    user_id      varchar(36)  NOT NULL,
    access_token varchar(255) NOT NULL,
    FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE
);

CREATE INDEX index_access_tokens ON access_tokens (access_token);

CREATE TABLE oidc_mapping
(
    id       SERIAL PRIMARY KEY,
    user_id  varchar(36)  NOT NULL,
    -- TODO (LOW) shorten the varchar
    oidc_sub varchar(255) NOT NULL UNIQUE,
    FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE
);
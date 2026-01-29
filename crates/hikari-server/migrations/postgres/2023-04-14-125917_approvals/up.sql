CREATE TABLE approvals_history
(
    id          varchar(36) PRIMARY KEY NOT NULL,
    created_at  TIMESTAMP               NOT NULL DEFAULT now(),
    user_id     varchar(36)             NOT NULL,
    approval_id varchar(255)            NOT NULL,
    version     varchar(255)            NOT NULL,
    FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE,
    unique (user_id, approval_id, version)
);

CREATE TABLE approvals
(
    id          varchar(36) PRIMARY KEY NOT NULL,
    user_id     varchar(36)             NOT NULL,
    approval_id varchar(255)            NOT NULL,
    version     varchar(255)            NOT NULL,
    FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE,
    unique (user_id, approval_id)
);

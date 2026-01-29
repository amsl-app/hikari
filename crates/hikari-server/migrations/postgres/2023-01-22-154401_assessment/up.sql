CREATE TABLE assessment
(
    id            varchar(36) PRIMARY KEY NOT NULL,
    user_id       varchar(36)             NOT NULL,
    status        smallint                NOT NULL,
    assessment_id varchar(255)            NOT NULL,
    FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE
);

CREATE TABLE answer
(
    id            varchar(36) PRIMARY KEY NOT NULL,
    assessment_id varchar(36)             NOT NULL,
    answer_type   smallint                NOT NULL,
    answer_id     varchar(255)            NOT NULL,
    data          TEXT                    NOT NULL,
    UNIQUE (assessment_id, answer_id),
    FOREIGN KEY (assessment_id) REFERENCES assessment (id) ON DELETE CASCADE
);

CREATE TABLE module_assessment
(
    id        varchar(36) PRIMARY KEY NOT NULL,
    user_id   varchar(36)             NOT NULL,
    module_id varchar(36)             NOT NULL,
    last_completed timestamp,
    last_pre  varchar(255),
    last_post varchar(255),

    FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE,
    FOREIGN KEY (last_pre) REFERENCES assessment (id) ON DELETE SET NULL,
    FOREIGN KEY (last_post) REFERENCES assessment (id) ON DELETE SET NULL
);

CREATE UNIQUE INDEX module_assessment_index ON module_assessment (user_id, module_id);
ALTER TABLE assessment ADD COLUMN completed timestamp;

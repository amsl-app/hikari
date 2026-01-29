CREATE TYPE "module_status_enum" AS ENUM ('non_started', 'started', 'finished');

CREATE TABLE "module_status"
(
    user_id    UUID   NOT NULL,
    module     VARCHAR(255)  NOT NULL,
    status     module_status_enum NOT NULL DEFAULT 'non_started',
    completion TIMESTAMP,

    PRIMARY KEY (user_id, module),
    FOREIGN KEY (user_id) REFERENCES "users" (id) ON DELETE CASCADE
);

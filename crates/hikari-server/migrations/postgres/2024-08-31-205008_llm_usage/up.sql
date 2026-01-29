CREATE TABLE llm_usage (
    number int NOT NULL PRIMARY KEY,
    user_id UUID NOT NULL,
    time TIMESTAMP NOT NULL default current_timestamp,
    tokens int NOT NULL,

    FOREIGN KEY(user_id) references USERS(id) ON DELETE cascade
);

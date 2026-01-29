CREATE TABLE groups_token (
    user_id UUID NOT NULL,
    token TEXT NOT NULL,
    added_at TIMESTAMP NOT NULL default current_timestamp,

    PRIMARY KEY(user_id, token),
    FOREIGN KEY(user_id) references USERS(id) ON DELETE cascade
);

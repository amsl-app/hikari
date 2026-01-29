create table users
(
    id              UUID                  not null primary key,
    name            varchar(255),
    birthday        DATE,
    subject         varchar(255),
    semester        smallint,
    gender          varchar(255),
    current_module  varchar(255),
    current_session varchar(255),
    onboarding      BOOLEAN default false not null
);

create table user_configs
(
    user_id UUID          not null references users (id) on delete cascade,
    key     varchar(255)  not null,
    value   varchar(1024) not null,
    PRIMARY KEY (user_id, key)
);

CREATE TABLE user_handle
(
    handle  BYTEA PRIMARY KEY                              NOT NULL,
    user_id UUID REFERENCES "users" (id) ON DELETE CASCADE NOT NULL
);

CREATE INDEX user_handle_user_id_idx ON user_handle (user_id);

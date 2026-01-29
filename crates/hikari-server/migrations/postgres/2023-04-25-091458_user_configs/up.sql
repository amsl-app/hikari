CREATE TABLE user_configs
(
    id      varchar(36) primary key not null ,
    user_id varchar(36) not null ,
    key     varchar(255) not null ,
    value   varchar(1024) not null ,
    FOREIGN KEY (user_id) REFERENCES "user" (id),
    CONSTRAINT uc_user_id_key UNIQUE (user_id, key)
);

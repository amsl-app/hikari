CREATE TYPE "message_direction_enum" AS ENUM('send', 'receive') ;
CREATE TYPE "content_type_enum" AS ENUM('text', 'payload', 'buttons') ;


create table llm_message (
    conversation_id     uuid        not null,
    message_order       int         not null,
    step                text        not null,
    created_at          timestamp   NOT NULL default current_timestamp,
    content_type        content_type_enum        not null,
    payload             text        not null,
    direction           message_direction_enum        not null,

    primary key (conversation_id, message_order),
    foreign key (conversation_id) references llm_conversation (conversation_id) on delete cascade
);

create index idx_llm_message_conversation_id_step on llm_message(conversation_id, step);
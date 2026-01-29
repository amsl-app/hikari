CREATE TYPE quiz_status_enum AS ENUM('open', 'closed');

CREATE TABLE quiz (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    module_id TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    status quiz_status_enum NOT NULL,
    FOREIGN KEY (user_id) references users (id) ON DELETE cascade
);

CREATE INDEX idx_quiz_user_id ON quiz (user_id);
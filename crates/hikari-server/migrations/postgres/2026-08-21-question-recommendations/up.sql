CREATE TABLE question_recommendation (
                                         id UUID PRIMARY KEY,
                                         quiz_id UUID NOT NULL REFERENCES quiz(id),
                                         question_id UUID NOT NULL REFERENCES question(id),
                                         recommended_at TIMESTAMP NOT NULL,
                                         used BOOLEAN NOT NULL DEFAULT FALSE,
                                         used_at TIMESTAMP NULL
);
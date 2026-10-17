CREATE TABLE goal (
                      id UUID PRIMARY KEY,
                      user_id UUID NOT NULL,
                      module_id VARCHAR NOT NULL,

                      quiz_id UUID NOT NULL UNIQUE REFERENCES quiz(id),

                      session_ids UUID[] NOT NULL DEFAULT ARRAY[]::UUID[],

                      target_mode VARCHAR NOT NULL,
                      level VARCHAR NOT NULL,
                      deadline TIMESTAMP NOT NULL,
                      outcome TEXT NOT NULL,

                      additional_outcomes JSONB NOT NULL DEFAULT '[]'::JSONB,
                      obstacles JSONB NOT NULL DEFAULT '[]'::JSONB,
                      plans JSONB NOT NULL DEFAULT '[]'::JSONB,

                      active BOOLEAN NOT NULL DEFAULT TRUE,

                      lecture_targets JSONB NOT NULL DEFAULT '{}'::JSONB
);

CREATE INDEX idx_goal_user_module
    ON goal(user_id, module_id);
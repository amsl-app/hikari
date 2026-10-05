CREATE TABLE planner_goal_milestone (
    goal_id      UUID NOT NULL,
    milestone_id UUID NOT NULL,
    PRIMARY KEY (goal_id, milestone_id),
    FOREIGN KEY (goal_id) REFERENCES planner_goal(id) ON DELETE CASCADE,
    FOREIGN KEY (milestone_id) REFERENCES planner_milestone(id) ON DELETE CASCADE
);
CREATE INDEX idx_planner_goal_milestone_milestone_id ON planner_goal_milestone(milestone_id);

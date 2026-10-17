use hikari_entity::quiz::goal::Model as GoalModel;
use hikari_model::quiz::goal::{Goal, LectureTarget, LectureTargets};


use crate::convert::FromDbModel;

impl FromDbModel<GoalModel> for Goal {
    fn from_db_model(goal: GoalModel) -> Self {
        Self {
            id: goal.id,
            module_id: goal.module_id,
            quiz_id: goal.quiz_id,
            session_ids: goal.session_ids,
            target_mode: goal.target_mode,
            level: goal.level,
            deadline: goal.deadline,
            outcome: goal.outcome,
            additional_outcomes: serde_json::from_value(goal.additional_outcomes)
                .expect("Invalid JSON in goal.additional_outcomes"),
            obstacles: serde_json::from_value(goal.obstacles)
                .expect("Invalid JSON in goal.obstacles"),
            plans: serde_json::from_value(goal.plans)
                .expect("Invalid JSON in goal.plans"),
            active: goal.active,
            lecture_targets: serde_json::from_value::<LectureTargets>(goal.lecture_targets)
                .expect("Invalid JSON in goal.lecture_targets"),
        }
    }
}
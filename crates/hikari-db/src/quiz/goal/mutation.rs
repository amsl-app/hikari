use hikari_entity::quiz::goal::{Entity as Goal, Model as GoalModel};
use sea_orm::ActiveModelTrait;
use sea_orm::EntityTrait;
use sea_orm::prelude::{Date, Json};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Set};
use std::error::Error;
use uuid::Uuid;

pub struct Mutation;
impl Mutation {
    #[allow(clippy::too_many_arguments)]
    pub async fn create_goal(
        db: &DatabaseConnection,
        user_id: Uuid,
        module_id: String,
        quiz_id: Uuid,
        session_ids: Vec<String>,
        target_mode: String,
        level: String,
        deadline: Date,
        outcome: String,
        additional_outcomes: Json,
        obstacles: Json,
        plans: Json,
        lecture_targets: Json,
    ) -> Result<GoalModel, DbErr> {
        let goal = hikari_entity::quiz::goal::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            module_id: Set(module_id),
            quiz_id: Set(quiz_id),
            session_ids: Set(session_ids),
            target_mode: Set(target_mode),
            level: Set(level),
            deadline: Set(deadline.into()),
            outcome: Set(outcome),
            additional_outcomes: Set(additional_outcomes),
            obstacles: Set(obstacles),
            plans: Set(plans),
            active: Set(true),
            lecture_targets: Set(lecture_targets),
        };

        goal.insert(db).await.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, "failed to create goal");
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_goal_in_transaction<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        module_id: String,
        quiz_id: Uuid,
        session_ids: Vec<String>,
        target_mode: String,
        level: String,
        deadline: Date,
        outcome: String,
        additional_outcomes: sea_orm::JsonValue,
        obstacles: sea_orm::JsonValue,
        plans: sea_orm::JsonValue,
        lecture_targets: sea_orm::JsonValue,
    ) -> Result<GoalModel, DbErr> {
        let goal = hikari_entity::quiz::goal::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            module_id: Set(module_id),
            quiz_id: Set(quiz_id),
            session_ids: Set(session_ids),
            target_mode: Set(target_mode),
            level: Set(level),
            deadline: Set(deadline.into()),
            outcome: Set(outcome),
            additional_outcomes: Set(additional_outcomes),
            obstacles: Set(obstacles),
            plans: Set(plans),
            active: Set(true),
            lecture_targets: Set(lecture_targets),
        };

        goal.insert(db).await
    }

    pub async fn deactivate_goal(db: &DatabaseConnection, goal_id: &Uuid) -> Result<GoalModel, DbErr> {
        let goal = Goal::find_by_id(*goal_id)
            .one(db)
            .await?
            .ok_or(DbErr::RecordNotFound(format!("Goal with id {goal_id} not found")))?;

        let mut goal: hikari_entity::quiz::goal::ActiveModel = goal.into();
        goal.active = Set(false);

        goal.update(db).await.inspect_err(|error| {
            tracing::error!(
                error = error as &dyn Error,
                goal_id = %goal_id,
                "failed to deactivate goal"
            );
        })
    }
}

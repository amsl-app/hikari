use crate::permissions::Permission;
use crate::routes::api::v0::planner::error::PlannerError;
use crate::user::ExtractUserId;
use axum::Extension;
use axum::Json;
use axum::Router;
use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::routing::get;
use chrono::NaiveDate;
use hikari_db::planner;
use hikari_db::sea_orm::DatabaseConnection;
use hikari_model::planner::{Goal, GoalFull, NewGoal, PlannerMilestone};
use hikari_model_tools::convert::FromDbModel;
use http::StatusCode;
use protect_axum::protect;
use sea_orm::ActiveValue;
use serde::Deserialize;
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub(crate) struct GoalFlags {
    pub deep: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct GoalChanges {
    pub name: Option<String>,
    pub date: Option<NaiveDate>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    #[allow(clippy::option_option)]
    pub description: Option<Option<String>>,
    pub fulfilled: Option<bool>,
}

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(get_goals).post(create_goal))
        .route("/{id}", get(get_goal).patch(update_goal).delete(delete_goal))
        .with_state(())
}

#[utoipa::path(
    get,
    path = "/api/v0/planner/goals",
    params(
        ("deep" = Option<String>, Query, description = "if set all goals are listed with their milestones embedded"),
    ),
    responses(
        (status = OK, description = "List goals for current user", body = [GoalFull]),
    ),
    tag = "v0/planner",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn get_goals(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Query(flags): Query<GoalFlags>,
) -> Result<impl IntoResponse, PlannerError> {
    let deep = flags.deep.is_some();
    let goals = planner::goal::Query::get_user_goals(&conn, user).await?;

    let mut milestones_by_goal: HashMap<Uuid, Vec<PlannerMilestone>> = HashMap::new();
    if deep {
        let goal_ids: Vec<Uuid> = goals.iter().map(|g| g.id).collect();
        let links = planner::goal_milestone::Query::get_links_for_goals(&conn, &goal_ids).await?;
        let milestone_ids: Vec<Uuid> = links.iter().map(|link| link.milestone_id).collect();
        let milestones_by_id: HashMap<Uuid, PlannerMilestone> =
            planner::planner_milestone::Query::get_user_milestones_by_ids(&conn, user, milestone_ids)
                .await?
                .into_iter()
                .map(|m| (m.id, PlannerMilestone::from_db_model(m)))
                .collect();
        for link in links {
            if let Some(milestone) = milestones_by_id.get(&link.milestone_id) {
                milestones_by_goal
                    .entry(link.goal_id)
                    .or_default()
                    .push(milestone.clone());
            }
        }
    }

    let goals = goals
        .into_iter()
        .map(|g| {
            let goal = Goal::from_db_model(g);
            let milestones = milestones_by_goal.remove(&goal.id).unwrap_or_default();
            goal.as_goal_full(milestones)
        })
        .collect::<Vec<GoalFull>>();
    Ok(Json(goals))
}

#[utoipa::path(
    get,
    path = "/api/v0/planner/goals/{id}",
    params(
        ("id" = Uuid, Path, description = "The ID of the goal to get"),
    ),
    responses(
        (status = OK, description = "Get a specific goal with its milestones", body = GoalFull),
        (status = NOT_FOUND, description = "Goal not found"),
    ),
    tag = "v0/planner",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn get_goal(
    ExtractUserId(user): ExtractUserId,
    Path(id): Path<Uuid>,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, PlannerError> {
    let goal = planner::goal::Query::get_user_goal(&conn, user, id)
        .await?
        .ok_or(PlannerError::NotFound)?;
    let goal = Goal::from_db_model(goal);

    let links = planner::goal_milestone::Query::get_links_for_goals(&conn, &[goal.id]).await?;
    let milestone_ids: Vec<Uuid> = links.iter().map(|link| link.milestone_id).collect();

    let milestones = planner::planner_milestone::Query::get_user_milestones_by_ids(&conn, user, milestone_ids)
        .await?
        .into_iter()
        .map(PlannerMilestone::from_db_model)
        .collect();

    Ok(Json(goal.as_goal_full(milestones)))
}

#[utoipa::path(
    post,
    path = "/api/v0/planner/goals",
    request_body = NewGoal,
    responses(
        (status = CREATED, description = "Create a goal", body = Goal),
    ),
    tag = "v0/planner",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn create_goal(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Json(body): Json<NewGoal>,
) -> Result<impl IntoResponse, PlannerError> {
    let name = body.name.trim().to_owned();
    if name.is_empty() {
        return Err(PlannerError::ValidationError("name must not be empty".to_owned()));
    }

    let goal = planner::goal::Mutation::create_goal(&conn, user, name, body.date, body.description).await?;
    Ok((StatusCode::CREATED, Json(Goal::from_db_model(goal))))
}

#[utoipa::path(
    patch,
    path = "/api/v0/planner/goals/{id}",
    request_body = GoalChanges,
    responses(
        (status = OK, description = "Update a goal", body = Goal),
        (status = NOT_FOUND, description = "Goal not found"),
    ),
    params(
        ("id" = Uuid, Path, description = "The ID of the goal to update"),
    ),
    tag = "v0/planner",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn update_goal(
    ExtractUserId(user): ExtractUserId,
    Path(id): Path<Uuid>,
    Extension(conn): Extension<DatabaseConnection>,
    Json(changes): Json<GoalChanges>,
) -> Result<impl IntoResponse, PlannerError> {
    let existing = planner::goal::Query::get_user_goal(&conn, user, id)
        .await?
        .ok_or(PlannerError::NotFound)?;

    let mut active_model = hikari_entity::planner::planner_goal::ActiveModel {
        id: ActiveValue::Unchanged(existing.id),
        user_id: ActiveValue::Unchanged(existing.user_id),
        ..Default::default()
    };

    if let Some(name) = changes.name {
        let name = name.trim().to_owned();
        if name.is_empty() {
            return Err(PlannerError::ValidationError("name must not be empty".to_owned()));
        }
        active_model.name = ActiveValue::Set(name);
    }
    if let Some(date) = changes.date {
        active_model.date = ActiveValue::Set(date);
    }
    if let Some(inner) = changes.description {
        active_model.description = ActiveValue::Set(inner);
    }
    if let Some(fulfilled) = changes.fulfilled {
        active_model.fulfilled = ActiveValue::Set(fulfilled);
    }

    let updated = planner::goal::Mutation::update_goal(&conn, active_model).await?;
    Ok(Json(Goal::from_db_model(updated)))
}

#[utoipa::path(
    delete,
    path = "/api/v0/planner/goals/{id}",
    responses(
        (status = NO_CONTENT, description = "Delete a goal"),
        (status = NOT_FOUND, description = "Goal not found"),
    ),
    params(
        ("id" = Uuid, Path, description = "The ID of the goal to delete"),
    ),
    tag = "v0/planner",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn delete_goal(
    ExtractUserId(user): ExtractUserId,
    Path(id): Path<Uuid>,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, PlannerError> {
    let rows_affected = planner::goal::Mutation::delete_goal(&conn, user, id).await?;
    if rows_affected == 0 {
        return Err(PlannerError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

use crate::permissions::Permission;
use crate::user::ExtractUserId;
use axum::Extension;
use axum::Json;
use axum::Router;
use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use chrono::NaiveDate;
use hikari_db::planner;
use hikari_db::sea_orm::DatabaseConnection;
use hikari_model::planner::{NewPlannerEntry, PlannerEntry};
use hikari_model_tools::convert::FromDbModel;
use http::StatusCode;
use protect_axum::protect;
use sea_orm::{ActiveValue, DbErr};
use serde::Deserialize;
use thiserror::Error;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Error, Debug)]
pub(crate) enum PlannerError {
    #[error(transparent)]
    SeaOrmError(#[from] DbErr),

    #[error("Planner entry could not be found")]
    NotFound,
}

impl IntoResponse for PlannerError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound | Self::SeaOrmError(DbErr::RecordNotFound(_)) => StatusCode::NOT_FOUND.into_response(),
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PlannerEntryChanges {
    pub date: Option<NaiveDate>,
    pub title: Option<String>,
    pub completed: Option<bool>,
    pub priority: Option<i32>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    #[allow(clippy::option_option)]
    pub module_id: Option<Option<String>>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    #[allow(clippy::option_option)]
    pub session_id: Option<Option<String>>,
}

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/entries", get(get_planner_entries).post(create_planner_entry))
        .route(
            "/entries/{id}",
            get(get_planner_entry)
                .patch(update_planner_entry)
                .delete(delete_planner_entry),
        )
        .with_state(())
}

#[derive(Debug, Deserialize)]
pub(crate) struct DateRangeFilter {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

#[utoipa::path(
    get,
    path = "/api/v0/planner/entries",
    params(
        ("from" = Option<NaiveDate>, Query, description = "Filter entries on or after this date (inclusive)"),
        ("to" = Option<NaiveDate>, Query, description = "Filter entries on or before this date (inclusive)"),
    ),
    responses(
        (status = OK, description = "List planner entries for current user", body = [PlannerEntry]),
    ),
    tag = "v0/planner",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn get_planner_entries(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Query(filter): Query<DateRangeFilter>,
) -> Result<impl IntoResponse, PlannerError> {
    let entries = planner::planner_entry::Query::get_user_planner_entries(&conn, user, filter.from, filter.to).await?;
    let entries = entries
        .into_iter()
        .map(FromDbModel::from_db_model)
        .collect::<Vec<PlannerEntry>>();
    Ok(Json(entries))
}

#[utoipa::path(
    get,
    path = "/api/v0/planner/entries/{id}",
    responses(
        (status = OK, description = "Get a specific planner entry", body = PlannerEntry),
        (status = NOT_FOUND, description = "Planner entry not found"),
    ),
    params(
        ("id" = Uuid, Path, description = "The ID of the planner entry to get"),
    ),
    tag = "v0/planner",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn get_planner_entry(
    ExtractUserId(user): ExtractUserId,
    Path(id): Path<Uuid>,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, PlannerError> {
    let entry = planner::planner_entry::Query::get_user_planner_entry(&conn, user, id)
        .await?
        .ok_or(PlannerError::NotFound)?;
    Ok(Json(PlannerEntry::from_db_model(entry)))
}

#[utoipa::path(
    post,
    path = "/api/v0/planner/entries",
    request_body = NewPlannerEntry,
    responses(
        (status = CREATED, description = "Create a planner entry", body = PlannerEntry),
    ),
    tag = "v0/planner",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn create_planner_entry(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Json(body): Json<NewPlannerEntry>,
) -> Result<impl IntoResponse, PlannerError> {
    let entry = planner::planner_entry::Mutation::create_planner_entry(
        &conn,
        user,
        body.date,
        body.title,
        body.priority,
        body.module_id,
        body.session_id,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(PlannerEntry::from_db_model(entry))))
}

#[utoipa::path(
    patch,
    path = "/api/v0/planner/entries/{id}",
    request_body = PlannerEntryChanges,
    responses(
        (status = OK, description = "Update a planner entry", body = PlannerEntry),
        (status = NOT_FOUND, description = "Planner entry not found"),
    ),
    params(
        ("id" = Uuid, Path, description = "The ID of the planner entry to update"),
    ),
    tag = "v0/planner",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn update_planner_entry(
    ExtractUserId(user): ExtractUserId,
    Path(id): Path<Uuid>,
    Extension(conn): Extension<DatabaseConnection>,
    Json(changes): Json<PlannerEntryChanges>,
) -> Result<impl IntoResponse, PlannerError> {
    let existing = planner::planner_entry::Query::get_user_planner_entry(&conn, user, id)
        .await?
        .ok_or(PlannerError::NotFound)?;

    let mut active_model = hikari_entity::planner_entry::ActiveModel {
        id: ActiveValue::Unchanged(existing.id),
        user_id: ActiveValue::Unchanged(existing.user_id),
        ..Default::default()
    };

    if let Some(date) = changes.date {
        active_model.date = ActiveValue::Set(date);
    }
    if let Some(title) = changes.title {
        active_model.title = ActiveValue::Set(title);
    }
    if let Some(completed) = changes.completed {
        active_model.completed = ActiveValue::Set(completed);
    }
    if let Some(priority) = changes.priority {
        active_model.priority = ActiveValue::Set(priority);
    }

    if let Some(inner) = changes.module_id {
        active_model.module_id = ActiveValue::Set(inner);
    }
    if let Some(inner) = changes.session_id {
        active_model.session_id = ActiveValue::Set(inner);
    }

    let updated = planner::planner_entry::Mutation::update_planner_entry(&conn, active_model).await?;
    Ok(Json(PlannerEntry::from_db_model(updated)))
}

#[utoipa::path(
    delete,
    path = "/api/v0/planner/entries/{id}",
    responses(
        (status = NO_CONTENT, description = "Delete a planner entry"),
        (status = NOT_FOUND, description = "Planner entry not found"),
    ),
    params(
        ("id" = Uuid, Path, description = "The ID of the planner entry to delete"),
    ),
    tag = "v0/planner",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn delete_planner_entry(
    ExtractUserId(user): ExtractUserId,
    Path(id): Path<Uuid>,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, PlannerError> {
    let rows_affected = planner::planner_entry::Mutation::delete_planner_entry(&conn, user, id).await?;
    if rows_affected == 0 {
        return Err(PlannerError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

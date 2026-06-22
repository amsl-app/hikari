use crate::AppConfig;
use crate::permissions::Permission;
use crate::user::{ExtractUser, ExtractUserId};
use axum::Extension;
use axum::Json;
use axum::Router;
use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use chrono::NaiveDate;
use hikari_db::planner;
use hikari_db::sea_orm::DatabaseConnection;
use hikari_model::planner::{
    NewPlannerEntry, PlannerAssistantExistingEntry, PlannerAssistantModule, PlannerAssistantRequest,
    PlannerAssistantSession, PlannerEntry, PlannerIcalToken,
};
use hikari_model_tools::convert::FromDbModel;
use http::{HeaderValue, StatusCode, header};
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

    #[error("LLM error")]
    LlmError,

    #[error("Validation error: {0}")]
    ValidationError(String),
}

impl IntoResponse for PlannerError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound | Self::SeaOrmError(DbErr::RecordNotFound(_)) => StatusCode::NOT_FOUND.into_response(),
            Self::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct PlannerEntryChanges {
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
        .route("/entries/bulk", post(create_planner_entries_bulk))
        .route(
            "/entries/{id}",
            get(get_planner_entry)
                .patch(update_planner_entry)
                .delete(delete_planner_entry),
        )
        .route("/ical-token", get(get_ical_token).delete(delete_ical_token))
        .route("/ical/{token}", get(get_planner_ical))
        .route("/assistant", post(planner_assistant))
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

#[utoipa::path(
    get,
    path = "/api/v0/planner/ical-token",
    responses(
        (status = OK, description = "Get or create the iCal feed token", body = PlannerIcalToken),
    ),
    tag = "v0/planner",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn get_ical_token(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, PlannerError> {
    let row = planner::ical_token::Mutation::get_or_create_ical_token(&conn, user).await?;
    Ok(Json(PlannerIcalToken { token: row.token }))
}

#[utoipa::path(
    delete,
    path = "/api/v0/planner/ical-token",
    responses(
        (status = NO_CONTENT, description = "Revoke the iCal feed token"),
    ),
    tag = "v0/planner",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn delete_ical_token(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, PlannerError> {
    planner::ical_token::Mutation::delete_ical_token(&conn, user).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/v0/planner/ical/{token}",
    responses(
        (status = OK, description = "iCal feed of planner entries", content_type = "text/calendar"),
        (status = NOT_FOUND, description = "Token not found"),
    ),
    params(
        ("token" = String, Path, description = "The iCal feed token"),
    ),
    tag = "v0/planner",
)]
pub(crate) async fn get_planner_ical(
    Path(token): Path<String>,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, PlannerError> {
    let user_id = planner::ical_token::Query::find_by_token(&conn, &token)
        .await?
        .ok_or(PlannerError::NotFound)?;

    let entries = planner::planner_entry::Query::get_user_planner_entries(&conn, user_id, None, None).await?;

    let body = build_ical(entries);
    Ok((
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/calendar; charset=utf-8"),
        )],
        body,
    ))
}

#[utoipa::path(
    post,
    path = "/api/v0/planner/entries/bulk",
    request_body = Vec<NewPlannerEntry>,
    responses(
        (status = CREATED, description = "Create multiple planner entries", body = [PlannerEntry]),
    ),
    tag = "v0/planner",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn create_planner_entries_bulk(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Json(body): Json<Vec<NewPlannerEntry>>,
) -> Result<impl IntoResponse, PlannerError> {
    let inputs = body
        .into_iter()
        .enumerate()
        .map(|(i, e)| validate_new_entry(i, e))
        .collect::<Result<Vec<_>, _>>()?;
    let created = planner::planner_entry::Mutation::create_planner_entries_bulk(&conn, user, inputs).await?;
    let entries = created
        .into_iter()
        .map(FromDbModel::from_db_model)
        .collect::<Vec<PlannerEntry>>();
    Ok((StatusCode::CREATED, Json(entries)))
}

fn validate_new_entry(
    index: usize,
    entry: NewPlannerEntry,
) -> Result<planner::planner_entry::PlannerEntryInput, PlannerError> {
    let title = entry.title.trim().to_owned();
    if title.is_empty() {
        return Err(PlannerError::ValidationError(format!(
            "entry {index}: title must not be empty"
        )));
    }
    if title.len() > 500 {
        return Err(PlannerError::ValidationError(format!(
            "entry {index}: title exceeds 500 characters"
        )));
    }
    if !(0..=3).contains(&entry.priority) {
        return Err(PlannerError::ValidationError(format!(
            "entry {index}: priority must be between 0 and 3"
        )));
    }
    Ok(planner::planner_entry::PlannerEntryInput {
        date: entry.date,
        title,
        priority: entry.priority,
        module_id: entry.module_id,
        session_id: entry.session_id,
    })
}

#[utoipa::path(
    post,
    path = "/api/v0/planner/assistant",
    request_body = PlannerAssistantRequest,
    responses(
        (status = OK, description = "Parsed planner entries from free text", body = [NewPlannerEntry]),
    ),
    tag = "v0/planner",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn planner_assistant(
    ExtractUser(user): ExtractUser,
    Extension(conn): Extension<DatabaseConnection>,
    Extension(app_config): Extension<AppConfig>,
    Json(body): Json<PlannerAssistantRequest>,
) -> Result<impl IntoResponse, PlannerError> {
    let today = body.today.unwrap_or_else(|| chrono::Utc::now().date_naive());

    let module_config = app_config.module_config();
    let filtered = module_config.modules_filtered(&user.groups);

    let modules: Vec<PlannerAssistantModule> = filtered
        .iter()
        .filter(|m| !m.hidden)
        .map(|m| PlannerAssistantModule {
            id: m.id.clone(),
            name: m.title.clone(),
        })
        .collect();

    let sessions: Vec<PlannerAssistantSession> = filtered
        .iter()
        .filter(|m| !m.hidden)
        .flat_map(|m| m.sessions.values().filter(|s| !s.hidden))
        .map(|s| PlannerAssistantSession {
            id: s.id.clone(),
            name: s.title.clone(),
        })
        .collect();

    let existing_db =
        planner::planner_entry::Query::get_user_planner_entries(&conn, user.id, Some(today), None).await?;
    let existing_entries: Vec<PlannerAssistantExistingEntry> = existing_db
        .into_iter()
        .map(|e| PlannerAssistantExistingEntry {
            date: e.date,
            title: e.title,
        })
        .collect();

    let entries = hikari_core::planner::planner_assistant(
        &user.id,
        body.text,
        today,
        modules,
        sessions,
        existing_entries,
        app_config.llm_config(),
        &conn,
    )
    .await
    .inspect_err(|e| tracing::error!(error = %e, "planner assistant LLM call failed"))
    .map_err(|_| PlannerError::LlmError)?;

    Ok(Json(entries))
}

fn build_ical(entries: Vec<hikari_entity::planner_entry::Model>) -> String {
    let mut out =
        String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//hikari//planner//EN\r\nCALSCALE:GREGORIAN\r\n");

    for entry in entries {
        let start = entry.date.format("%Y%m%d").to_string();
        let end = entry.date.succ_opt().unwrap_or(entry.date).format("%Y%m%d").to_string();
        let status = if entry.completed { "COMPLETED" } else { "NEEDS-ACTION" };
        let dtstamp = entry.updated_at.format("%Y%m%dT%H%M%SZ").to_string();

        out.push_str("BEGIN:VEVENT\r\n");
        out.push_str(&format!("UID:{}@hikari\r\n", entry.id));
        out.push_str(&format!("DTSTAMP:{}\r\n", dtstamp));
        out.push_str(&format!("DTSTART;VALUE=DATE:{}\r\n", start));
        out.push_str(&format!("DTEND;VALUE=DATE:{}\r\n", end));
        ical_fold_line(&mut out, "SUMMARY", &ical_escape(&entry.title));
        out.push_str(&format!("STATUS:{}\r\n", status));
        out.push_str("END:VEVENT\r\n");
    }

    out.push_str("END:VCALENDAR\r\n");
    out
}

fn ical_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace("\r\n", "\\n")
        .replace(['\r', '\n'], "\\n")
}

// RFC 5545 §3.1: fold at 75 octets
fn ical_fold_line(out: &mut String, name: &str, value: &str) {
    if name.len() + 1 + value.len() <= 75 {
        out.push_str(name);
        out.push(':');
        out.push_str(value);
        out.push_str("\r\n");
        return;
    }
    let line = format!("{}:{}", name, value);
    let bytes = line.as_bytes();
    let mut pos = 0;
    let mut first = true;
    while pos < bytes.len() {
        let limit = if first { 75 } else { 74 };
        let end = (pos + limit).min(bytes.len());
        // Walk back to a UTF-8 character boundary
        let end = (pos..=end).rev().find(|&i| line.is_char_boundary(i)).unwrap_or(end);
        if !first {
            out.push(' ');
        }
        out.push_str(&line[pos..end]);
        out.push_str("\r\n");
        first = false;
        pos = end;
    }
}

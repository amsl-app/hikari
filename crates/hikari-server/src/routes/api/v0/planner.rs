use crate::AppConfig;
use crate::permissions::Permission;
use crate::routes::api::v0::planner::error::PlannerError;
use crate::user::{ExtractUser, ExtractUserId};
use axum::Extension;
use axum::Json;
use axum::Router;
use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use chrono::NaiveDate;
use hikari_db::planner;
use hikari_db::planner::planner_milestone::MilestoneInput;
use hikari_db::sea_orm::DatabaseConnection;
use hikari_model::planner::{
    NewPlannerEntry, NewPlannerMilestone, PlannerAssistantRequest, PlannerEntry, PlannerIcalToken, PlannerMilestone,
};
use hikari_model_tools::convert::FromDbModel;
use http::{HeaderValue, StatusCode, header};
use protect_axum::protect;
use sea_orm::ActiveValue;
use serde::Deserialize;
use std::cmp::Ordering;
use std::fmt::Write;
use utoipa::ToSchema;
use uuid::Uuid;

mod error;

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct PlannerEntryChanges {
    pub date: Option<NaiveDate>,
    pub title: Option<String>,
    pub completed: Option<bool>,
    pub priority: Option<i32>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    #[allow(clippy::option_option)]
    pub milestone_id: Option<Option<Uuid>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct PlannerMilestoneChanges {
    pub title: Option<String>,
    pub date: Option<NaiveDate>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    #[allow(clippy::option_option)]
    pub description: Option<Option<String>>,
}

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/entries", get(get_planner_entries).post(create_planner_entries))
        .route(
            "/entries/{id}",
            get(get_planner_entry)
                .patch(update_planner_entry)
                .delete(delete_planner_entry),
        )
        .route("/ical-token", get(get_ical_token).delete(delete_ical_token))
        .route("/ical/{token}", get(get_planner_ical))
        .route("/assistant", post(planner_assistant))
        .route("/milestones", get(get_milestones).post(create_milestone))
        .route(
            "/milestones/{id}",
            get(get_milestone).patch(update_milestone).delete(delete_milestone),
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

    if let Some(inner) = changes.milestone_id {
        if let Some(milestone_id) = inner {
            // Check if the milestone exist and belongs to the user
            planner::planner_milestone::Query::get_user_milestones_by_ids(&conn, user, vec![milestone_id]).await?;
        }
        active_model.milestone_id = ActiveValue::Set(inner);
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
    path = "/api/v0/planner/entries",
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
pub(crate) async fn create_planner_entries(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Json(body): Json<Vec<NewPlannerEntry>>,
) -> Result<impl IntoResponse, PlannerError> {
    let inputs = body
        .into_iter()
        .enumerate()
        .map(|(i, e)| validate_new_entry(i, e))
        .collect::<Result<Vec<_>, _>>()?;

    let milestone_ids: Vec<Uuid> = inputs.iter().filter_map(|i| i.milestone_id).collect();
    if !milestone_ids.is_empty() {
        let unique: Vec<Uuid> = {
            let mut v = milestone_ids.clone();
            v.sort();
            v.dedup();
            v
        };
        // Check if all milestone_ids exist and belong to the user
        planner::planner_milestone::Query::get_user_milestones_by_ids(&conn, user, unique).await?;
    }

    let created = planner::planner_entry::Mutation::create_planner_entries(&conn, user, inputs).await?;
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
        milestone_id: entry.milestone_id,
    })
}

fn validate_milestone_title(title: &str) -> Result<String, PlannerError> {
    let title = title.trim().to_owned();
    if title.is_empty() {
        return Err(PlannerError::ValidationError("title must not be empty".to_owned()));
    }
    if title.len() > 500 {
        return Err(PlannerError::ValidationError("title exceeds 500 characters".to_owned()));
    }
    Ok(title)
}

#[utoipa::path(
    get,
    path = "/api/v0/planner/milestones",
    responses((status = OK, description = "List milestones for current user", body = [PlannerMilestone])),
    tag = "v0/planner",
    security(("token" = []))
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn get_milestones(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, PlannerError> {
    let milestones = planner::planner_milestone::Query::get_user_milestones(&conn, user).await?;
    let milestones = milestones
        .into_iter()
        .map(FromDbModel::from_db_model)
        .collect::<Vec<PlannerMilestone>>();
    Ok(Json(milestones))
}

#[utoipa::path(
    post,
    path = "/api/v0/planner/milestones",
    request_body = NewPlannerMilestone,
    responses((status = CREATED, description = "Create a milestone", body = PlannerMilestone)),
    tag = "v0/planner",
    security(("token" = []))
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn create_milestone(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Json(body): Json<NewPlannerMilestone>,
) -> Result<impl IntoResponse, PlannerError> {
    let title = validate_milestone_title(&body.title)?;
    let input = MilestoneInput {
        title,
        date: body.date,
        description: body.description,
        module_id: None,
        origin_id: None,
    };
    let created = planner::planner_milestone::Mutation::create_milestone(&conn, user, input).await?;
    Ok((StatusCode::CREATED, Json(PlannerMilestone::from_db_model(created))))
}

#[utoipa::path(
    get,
    path = "/api/v0/planner/milestones/{id}",
    responses(
        (status = OK, description = "Get a milestone", body = PlannerMilestone),
        (status = NOT_FOUND, description = "Milestone not found"),
    ),
    params(("id" = Uuid, Path, description = "The milestone id")),
    tag = "v0/planner",
    security(("token" = []))
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn get_milestone(
    ExtractUserId(user): ExtractUserId,
    Path(id): Path<Uuid>,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, PlannerError> {
    let milestone = planner::planner_milestone::Query::get_user_milestone(&conn, user, id)
        .await?
        .ok_or(PlannerError::NotFound)?;
    Ok(Json(PlannerMilestone::from_db_model(milestone)))
}

#[utoipa::path(
    patch,
    path = "/api/v0/planner/milestones/{id}",
    request_body = PlannerMilestoneChanges,
    responses(
        (status = OK, description = "Update a milestone", body = PlannerMilestone),
        (status = NOT_FOUND, description = "Milestone not found"),
    ),
    params(("id" = Uuid, Path, description = "The milestone id")),
    tag = "v0/planner",
    security(("token" = []))
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn update_milestone(
    ExtractUserId(user): ExtractUserId,
    Path(id): Path<Uuid>,
    Extension(conn): Extension<DatabaseConnection>,
    Json(changes): Json<PlannerMilestoneChanges>,
) -> Result<impl IntoResponse, PlannerError> {
    let existing = planner::planner_milestone::Query::get_user_milestone(&conn, user, id)
        .await?
        .ok_or(PlannerError::NotFound)?;

    let mut active_model = hikari_entity::planner_milestone::ActiveModel {
        id: ActiveValue::Unchanged(existing.id),
        user_id: ActiveValue::Unchanged(existing.user_id),
        ..Default::default()
    };

    if let Some(title) = changes.title {
        active_model.title = ActiveValue::Set(validate_milestone_title(&title)?);
    }
    if let Some(date) = changes.date {
        active_model.date = ActiveValue::Set(date);
    }
    if let Some(description) = changes.description {
        active_model.description = ActiveValue::Set(description);
    }

    let updated = planner::planner_milestone::Mutation::update_milestone(&conn, active_model).await?;
    Ok(Json(PlannerMilestone::from_db_model(updated)))
}

#[utoipa::path(
    delete,
    path = "/api/v0/planner/milestones/{id}",
    responses(
        (status = NO_CONTENT, description = "Delete a milestone"),
        (status = NOT_FOUND, description = "Milestone not found"),
    ),
    params(("id" = Uuid, Path, description = "The milestone id")),
    tag = "v0/planner",
    security(("token" = []))
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn delete_milestone(
    ExtractUserId(user): ExtractUserId,
    Path(id): Path<Uuid>,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, PlannerError> {
    let rows = planner::planner_milestone::Mutation::delete_milestone(&conn, user, id).await?;
    if rows == 0 {
        return Err(PlannerError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
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

    let milestones = planner::planner_milestone::Query::get_user_milestones(&conn, user.id).await?;
    let milestones: Vec<hikari_core::planner::PlannerAssistantMilestone> = milestones
        .into_iter()
        .map(|m| hikari_core::planner::PlannerAssistantMilestone {
            id: m.id,
            title: m.title,
            date: m.date,
        })
        .collect();

    let existing_db =
        planner::planner_entry::Query::get_user_planner_entries(&conn, user.id, Some(today), None).await?;
    let existing_entries: Vec<hikari_core::planner::PlannerAssistantExistingEntry> = existing_db
        .into_iter()
        .map(|e| hikari_core::planner::PlannerAssistantExistingEntry {
            date: e.date,
            title: e.title,
        })
        .collect();

    let entries = hikari_core::planner::planner_assistant(
        &user.id,
        body.text,
        today,
        milestones,
        existing_entries,
        app_config.llm_config(),
        &conn,
    )
    .await
    .inspect_err(|e| tracing::error!(error = %e, "planner assistant LLM call failed"))
    .map_err(|_| PlannerError::LlmError)?;

    Ok(Json(entries))
}

// Macro to push an ical value to the output buffer without checking the line length
macro_rules! push_ical_line {
    ($out:ident, key: $key:expr, value: $($values:expr),+) => {
        $out.push_str($key);
        $out.push(':');
        $( $out.push_str($values); )+
        $out.push_str("\r\n");
    };
    ($out:ident, key: $key:expr, write: $write:expr, value: $($values:expr),*) => {
        $out.push_str($key);
        $out.push(':');
        // We ignore the error as the Write implementation of String can't fail (only OOM Panic)
        let _ = write!(&mut $out, "{}", $write);
        $( $out.push_str($values); )+
        $out.push_str("\r\n");
    };
    ($out:ident, key: $key:expr, date: $date:expr) => {
        $out.push_str($key);
        $out.push(':');
        // We ignore the error as the Write implementation of String can't fail (only OOM Panic)
        let _ = $date.write_to(&mut $out);
        $out.push_str("\r\n");
    }
}

fn ascii_ical_len<'a, I: Iterator<Item = &'a str>>(entries: I) -> usize {
    let mut total = 81;
    for entry in entries {
        total += 175;
        total += ical_fold_required_ascii_space("SUMMARY".len(), entry);
    }
    total += 15;
    total
}

fn build_ical(entries: Vec<hikari_entity::planner_entry::Model>) -> String {
    // We reserve enough space to avoid reallocations in the simple case, where all entries are ASCII or contain very few special characters.
    // We extra some space to avoid reallocations in case a special character falls into a folding point. (3 bytes overhead per extra line)
    let mut out = String::with_capacity(ascii_ical_len(entries.iter().map(|entry| entry.title.as_str())) + 3 * 2);
    out.push_str("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//hikari//planner//EN\r\nCALSCALE:GREGORIAN\r\n");

    for entry in entries {
        let start = entry.date.format("%Y%m%d");
        let end = entry.date.succ_opt().unwrap_or(entry.date).format("%Y%m%d");
        let status = if entry.completed { "CANCELLED" } else { "CONFIRMED" };
        let dtstamp = entry.updated_at.format("%Y%m%dT%H%M%SZ");

        out.push_str("BEGIN:VEVENT\r\n");
        push_ical_line!(out, key: "UID", write: &entry.id, value: "@hikari");
        push_ical_line!(out, key: "DTSTAMP", date: dtstamp);
        push_ical_line!(out, key: "DTSTART;VALUE=DATE", date: start);
        push_ical_line!(out, key: "DTEND;VALUE=DATE", date: end);
        ical_fold_line(&mut out, "SUMMARY", &ical_escape(&entry.title));
        push_ical_line!(out, key: "STATUS", value: status);
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

/// Calculates the required space for folding a value only containing ASCII characters.
fn ical_fold_required_ascii_space(key_len: usize, value: &str) -> usize {
    key_len + value.len() + (((key_len + value.len()) / 74) + 1) * 3
}

// RFC 5545 §3.1: fold at 75 octets
fn ical_fold_line(out: &mut String, name: &str, value: &str) {
    out.push_str(name);
    out.push(':');

    let bytes = value.as_bytes();

    let mut pos = 0;
    // First line: name + ":" + value_part must be <= 75
    // Continuation lines: " " + value_part must be <= 75
    let mut limit = 75 - name.len() - 1;

    loop {
        let mut new_pos = pos + limit;
        match new_pos.cmp(&bytes.len()) {
            Ordering::Greater => {
                new_pos = bytes.len();
            }
            Ordering::Equal => {
                // Already at the end of the line, do nothing
            }
            Ordering::Less => {
                // pos might point into the middle of a UTF-8 char, Walk back to find UTF-8 char boundary
                new_pos = value.floor_char_boundary(new_pos);
                // This should never fail: value is valid utf-8, and an utf-8 char cannot be longer than 6 bytes, so we should always find a valid boundary
                debug_assert!(new_pos > pos);
            }
        }
        out.push_str(&value[pos..new_pos]);
        pos = new_pos;
        if new_pos == bytes.len() {
            out.push_str("\r\n");
            break;
        }
        out.push_str("\r\n ");
        limit = 74;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fold(name: &str, value: &str) -> String {
        let mut out = String::new();
        ical_fold_line(&mut out, name, value);
        out
    }

    fn expected_ical_vevent(value: &str) -> String {
        format!(
            "\
BEGIN:VEVENT\r\n\
UID:00000000-0000-0000-0000-000000000000@hikari\r\n\
DTSTAMP:19700101T000000Z\r\n\
DTSTART;VALUE=DATE:19700101\r\n\
DTEND;VALUE=DATE:19700102\r\n\
SUMMARY:{value}\r\n\
STATUS:CONFIRMED\r\n\
END:VEVENT\r\n\
"
        )
    }

    fn expected_ical_output(vevent: &str) -> String {
        format!(
            "\
BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
PRODID:-//hikari//planner//EN\r\n\
CALSCALE:GREGORIAN\r\n\
{}\
END:VCALENDAR\r\n",
            vevent
        )
    }

    fn create_planner_entry(value: &str) -> hikari_entity::planner_entry::Model {
        hikari_entity::planner_entry::Model {
            id: Default::default(),
            user_id: Default::default(),
            date: Default::default(),
            title: value.to_string(),
            completed: false,
            priority: 0,
            milestone_id: None,
            created_at: Default::default(),
            updated_at: Default::default(),
        }
    }

    const ICAL_TEST_VALUES: [(&str, &str); 3] = [
        ("", ""),
        ("test", "test"),
        (
            "long test that requires adding linebreaks according to RFC 5545 section 3.1 guidelines for iCalendar format",
            "long test that requires adding linebreaks according to RFC 5545 sec\r\n tion 3.1 guidelines for iCalendar format",
        ),
    ];

    #[test]
    fn test_build_ical() {
        for (value, split_value) in ICAL_TEST_VALUES {
            let entries = vec![create_planner_entry(value)];
            let expected = expected_ical_output(&expected_ical_vevent(split_value));
            let res = build_ical(entries);
            assert_eq!(res, expected);
            // 96: Header + Footer
            // 175: Per VEVENT Constant
            // 10: Len of "Summary:\r\n"
            assert_eq!(
                res.len(),
                96 + 175 + 10 + split_value.len(),
                "Calculated length does not match expected length for value: {}",
                value
            );
            assert_eq!(
                res.capacity(),
                96 + 175 + 10 + split_value.len() + 6,
                "Calculated capacity does not match expected capacity for value: {}",
                value
            );
        }
    }

    #[test]
    fn test_build_ical_multiple_entries() {
        let models: Vec<_> = ICAL_TEST_VALUES
            .iter()
            .map(|(val, _)| create_planner_entry(val))
            .collect();
        let expected_vevents = ICAL_TEST_VALUES
            .iter()
            .map(|(_, expected)| expected_ical_vevent(expected))
            .collect::<Vec<_>>();
        let res = build_ical(models);

        let expected = expected_ical_output(&expected_vevents.join(""));
        let total_vevent_len = expected_vevents.iter().map(|vevent| vevent.len()).sum::<usize>();
        assert_eq!(res, expected);
        assert_eq!(
            res.len(),
            96 + total_vevent_len,
            "Calculated length does not match expected length"
        );
        assert_eq!(
            res.capacity(),
            96 + total_vevent_len + 6,
            "Calculated capacity does not match expected capacity"
        );
    }

    #[test]
    fn test_build_ical_special_chars() {
        let value = "a".repeat(61) + "👍🏽" + "a".repeat(71).as_str();
        let expected_value = "a".repeat(61) + "👍\r\n 🏽" + "a".repeat(70).as_str() + "\r\n a";
        let entry = create_planner_entry(&value);
        let expected = expected_ical_output(&expected_ical_vevent(&expected_value));
        let res = build_ical(vec![entry]);
        assert_eq!(res, expected);
        // The emoji without the color modifier should be on the first line
        let expected_summary_line = String::from("SUMMARY:") + "a".repeat(61).as_str() + "👍";
        assert_eq!(expected_summary_line.len(), 73); // Sanity check: Expected line is one shorter than possible
        assert_eq!(res.lines().skip(9).next().unwrap(), expected_summary_line);
        let expected_next_line = String::from(" 🏽") + "a".repeat(70).as_str();
        // Next line should only be the color modifier
        assert_eq!(res.lines().skip(10).next().unwrap(), expected_next_line);
        assert_eq!(
            res.len(),
            96 + 175 + 10 + value.len() + 3 * 2,
            "Calculated length does not match expected length for value: {}",
            value
        );
        assert_eq!(
            res.capacity(),
            96 + 175 + 10 + value.len() + 3 * 2 + 6 - 3,
            "Calculated capacity does not match expected capacity for value: {}",
            value
        );
    }

    #[test]
    fn test_short_line_no_folding() {
        let result = fold("SUMMARY", "Short event");
        assert_eq!(result, "SUMMARY:Short event\r\n");
    }

    #[test]
    fn test_exact_75_chars_no_folding() {
        let value = "a".repeat(67); // SUMMARY: is 8 chars, so 67 + 8 = 75
        let result = fold("SUMMARY", &value);
        assert_eq!(result, format!("SUMMARY:{}\r\n", value));
    }

    #[test]
    fn test_just_over_75_chars_folds() {
        let value = "a".repeat(68); // SUMMARY: is 8 chars, 68 > 67 available on first line
        let result = fold("SUMMARY", &value);
        let lines: Vec<&str> = result.split("\r\n").filter(|s| !s.is_empty()).collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].len() <= 75, "First line exceeds 75 bytes");
        assert!(lines[1].starts_with(' '), "Continuation line missing leading space");
        // Verify reassembly
        let reassembled: String = format!("{}{}", &lines[0][8..], &lines[1][1..]);
        assert_eq!(reassembled, value);
    }

    #[test]
    fn test_multiple_folds() {
        let value = "x".repeat(200);
        let result = fold("SUMMARY", &value);
        let lines: Vec<&str> = result.split("\r\n").filter(|s| !s.is_empty()).collect();
        assert_eq!(lines.len(), 3);
        for (i, line) in lines.iter().enumerate() {
            assert!(line.len() <= 75, "Line {} exceeds 75 bytes: len={}", i, line.len());
            if i > 0 {
                assert!(line.starts_with(' '), "Continuation line {} missing leading space", i);
            }
        }
        // Verify reassembly
        let reassembled: String = lines
            .iter()
            .enumerate()
            .map(|(i, l)| if i == 0 { &l[8..] } else { &l[1..] })
            .collect();
        assert_eq!(reassembled, value);
    }

    #[test]
    fn test_utf8_char_boundary() {
        let value = "ä".repeat(40); // 80 bytes (2 bytes per ä)
        let result = fold("SUMMARY", &value);
        let lines: Vec<&str> = result.split("\r\n").filter(|s| !s.is_empty()).collect();
        // Verify no line exceeds 75 bytes
        for line in &lines {
            assert!(line.len() <= 75, "Line exceeds 75 bytes: {}", line.len());
        }
        // Reassemble by removing fold prefixes
        let mut reassembled = String::new();
        for (i, line) in lines.iter().enumerate() {
            if i == 0 {
                reassembled.push_str(&line[8..]); // skip "SUMMARY:"
            } else {
                reassembled.push_str(&line[1..]); // skip leading space
            }
        }
        assert_eq!(reassembled, value);
    }

    #[test]
    fn test_empty_value() {
        let result = fold("SUMMARY", "");
        assert_eq!(result, "SUMMARY:\r\n");
    }

    #[test]
    fn test_long_name() {
        let name = "DESCRIPTION";
        let value = "This is a very long description that should be folded properly according to RFC 5545 section 3.1 guidelines for iCalendar format";
        let result = fold(name, value);
        let lines: Vec<&str> = result.trim_end_matches("\r\n").split("\r\n").collect();
        for (i, line) in lines.iter().enumerate() {
            if i == 0 {
                assert!(line.len() <= 75);
            } else {
                assert!(line.len() <= 75);
                assert!(line.starts_with(' '));
            }
        }
    }

    #[test]
    fn test_fold_required_ascii_space() {
        let test_cases = [
            (1, "This is a short description, that should fit on one line"),
            (
                2,
                "This is a very long description that should be folded properly according to RFC 5545 section 3.1 guidelines for iCalendar format",
            ),
            (
                3,
                "This is another very long description that should be folded twice to properly fit the iCalendar format according to RFC 5545 section 3.1 guidelines",
            ),
        ];
        let name = "SUMMARY";
        for (expected_lines, test_case) in test_cases {
            let required_space = ical_fold_required_ascii_space(name.len(), test_case);
            let mut result = String::new();
            result.reserve(required_space);
            ical_fold_line(&mut result, name, test_case);
            assert_eq!(result.len(), required_space);
            assert_eq!(result.capacity(), required_space);
            assert_eq!(result.lines().count(), expected_lines);
        }
    }

    #[test]
    fn test_ical_escape() {
        let test_cases = [
            ("", ""),
            ("a", "a"),
            ("a,b", "a\\,b"),
            ("a;", "a\\;"),
            ("a\\", "a\\\\"),
            ("a\\;", "a\\\\\\;"),
            ("a\r\nb", "a\\nb"),
            ("a\r\n\nb", "a\\n\\nb"),
        ];
        for (input, expected) in test_cases {
            let result = ical_escape(input);
            assert_eq!(result, expected);
        }
    }
}

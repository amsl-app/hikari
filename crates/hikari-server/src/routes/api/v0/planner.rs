use std::cmp::Ordering;
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
use hikari_db::sea_orm::DatabaseConnection;
use hikari_model::planner::{
    NewPlannerEntry, PlannerAssistantExistingEntry, PlannerAssistantModule, PlannerAssistantRequest,
    PlannerAssistantSession, PlannerEntry, PlannerIcalToken,
};
use hikari_model_tools::convert::FromDbModel;
use http::{HeaderValue, StatusCode, header};
use protect_axum::protect;
use sea_orm::ActiveValue;
use serde::Deserialize;
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
        let status = if entry.completed { "CANCELLED" } else { "CONFIRMED" };
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

/// Calculates the required space for folding a value only containing ASCII characters.
fn ical_fold_required_ascii_space(name: &str, value: &str) -> usize {
    name.len() + value.len() + (((name.len() + value.len()) / 74) + 1) * 3
}

// RFC 5545 §3.1: fold at 75 octets
fn ical_fold_line(out: &mut String, name: &str, value: &str) {
    out.reserve(ical_fold_required_ascii_space(name, value));
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
            },
            Ordering::Equal => {
                // Already at the end of the line, do nothing
            },
            Ordering::Less => {
                // pos might point into the middle of a UTF-8 char, Walk back to find UTF-8 char boundary
                new_pos = value.floor_char_boundary(new_pos);
                // This should never fail: value is valid utf-8, and an utf-8 char cannot be longer than 6 bytes, so we should always find a valid boundary
                debug_assert!(new_pos > pos);
            },
        }
        out.push_str(&value[pos..new_pos]);
        pos = new_pos;
        if new_pos == bytes.len() {
            out.push_str("\r\n");
            break;
        } else {
            out.push_str("\r\n ");
        }
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
            (2, "This is a very long description that should be folded properly according to RFC 5545 section 3.1 guidelines for iCalendar format"),
            (3, "This is another very long description that should be folded twice to properly fit the iCalendar format according to RFC 5545 section 3.1 guidelines"),
        ];
        let name = "SUMMARY";
        for (expected_lines, test_case) in test_cases {
            let required_space = ical_fold_required_ascii_space(name, test_case);
            let result = fold(name, test_case);
            assert_eq!(result.len(), required_space);
            assert_eq!(result.capacity(), required_space);
            assert_eq!(result.lines().count(), expected_lines);
        }
    }
}

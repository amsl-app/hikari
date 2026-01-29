use crate::routes::api::v0::journal::error::JournalError;
use crate::user::ExtractUserId;
use axum::Extension;
use axum::extract::Path;
use axum::response::{IntoResponse, Json};
use axum::routing::{Router, get};
use hikari_db::journal::journal_entry;
use hikari_db::sea_orm::DatabaseConnection;
use http::StatusCode;
use uuid::Uuid;

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/mood", get(get_mood).put(set_mood).delete(unset_mood))
        .with_state(())
}

#[utoipa::path(
    get,
    path = "/api/v0/journal/entries/{journal_entry}/mood",
    responses(
        (status = OK, description = "Gets the mood for the journal entry.", content_type = "application/json", body = f32),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
pub(crate) async fn get_mood(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(journal_entry): Path<Uuid>,
) -> Result<impl IntoResponse, JournalError> {
    let journal_entry = journal_entry::Query::get_user_journal_entry(&conn, user, journal_entry)
        .await?
        .ok_or(JournalError::NotFound)?;

    Ok(Json(journal_entry.mood))
}

#[utoipa::path(
    put,
    path = "/api/v0/journal/entries/{journal_entry}/mood",
    request_body = f32,
    responses(
        (status = OK, description = "Sets the mood for a journal entry. The mood should be a float be in the range [0, 1]."),
        (status = BAD_REQUEST, description = "Mood is outside the range of [0, 1]."),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
pub(crate) async fn set_mood(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(journal_entry): Path<Uuid>,
    Json(new_mood): Json<f32>,
) -> Result<impl IntoResponse, JournalError> {
    if !(0.0..=1.0).contains(&new_mood) {
        return Ok((StatusCode::BAD_REQUEST, "Mood must be in the range [0, 1].").into_response());
    }

    journal_entry::Mutation::set_journal_entry_mood(&conn, user, journal_entry, Some(new_mood)).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    delete,
    path = "/api/v0/journal/entries/{journal_entry}/mood",
    responses(
        (status = OK, description = "Sets the mood for a journal entry."),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]

pub(crate) async fn unset_mood(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(journal_entry): Path<Uuid>,
) -> Result<impl IntoResponse, JournalError> {
    journal_entry::Mutation::set_journal_entry_mood(&conn, user, journal_entry, None).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

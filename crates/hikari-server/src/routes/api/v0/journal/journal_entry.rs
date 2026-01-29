pub(crate) mod focus;
pub(crate) mod mood;

use crate::permissions::Permission;
use crate::routes::api::v0::journal::error::JournalError;
use crate::user::ExtractUserId;
use axum::Extension;
use axum::extract::Path;
use axum::response::{IntoResponse, Json};
use axum::routing::{Router, get};
use hikari_db::journal;
use hikari_db::sea_orm::DatabaseConnection;
use hikari_model::journal::JournalEntry;
use hikari_model::journal::content::{JournalContent, JournalContentId};
use hikari_model::journal::partial::NewJournalContent;
use hikari_model_tools::convert::FromDbModel;
use protect_axum::protect;
use uuid::Uuid;

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(get_journal_entry))
        .nest(
            "/content",
            Router::new()
                .route("/", get(list_journal_entry_contents).post(add_journal_entry_content))
                .route("/{content_id}", get(get_journal_entry_content)),
        )
        .merge(focus::create_router())
        .merge(mood::create_router())
        .with_state(())
}

#[utoipa::path(
    get,
    path = "/api/v0/journal/entries/{journal_entry}",
    responses(
        (status = OK, description = "List journal entries", body = [JournalEntry]),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Journal", ty = "Permission")]
pub(crate) async fn get_journal_entry(
    ExtractUserId(user): ExtractUserId,
    Path(journal_entry): Path<Uuid>,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, JournalError> {
    let journal_entry = journal::journal_entry::Query::get_user_journal_entry(&conn, user, journal_entry)
        .await?
        .ok_or(JournalError::NotFound)?;

    Ok(Json(JournalEntry::from_db_model(journal_entry)))
}

#[utoipa::path(
    get,
    path = "/api/v0/journal/entries/{journal_entry}/content",
    responses(
        (status = OK, description = "List journal entries", body = [JournalContent]),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Journal", ty = "Permission")]
pub(crate) async fn list_journal_entry_contents(
    ExtractUserId(user): ExtractUserId,
    Path(journal_entry): Path<Uuid>,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, JournalError> {
    let journal_contents =
        journal::journal_content::Query::get_user_journal_entry_contents(&conn, user, journal_entry).await?;

    let journal_entries = journal_contents
        .into_iter()
        .map(JournalContent::from_db_model)
        .collect::<Vec<_>>();
    Ok(Json(journal_entries))
}

#[utoipa::path(
    post,
    path = "/api/v0/journal/entries/{journal_entry}/content",
    request_body(content = NewJournalContent, description = "The content to create", content_type = "text/plain"),
    responses(
        (status = CREATED, description = "Add journal entry content", body = JournalContentId),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Journal", ty = "Permission")]
pub(crate) async fn add_journal_entry_content(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(journal_entry): Path<Uuid>,
    Json(body): Json<NewJournalContent>,
) -> Result<impl IntoResponse, JournalError> {
    // Make sure the user actually owns the journal entry by querying it
    let journal_entry = journal::journal_entry::Query::get_user_journal_entry(&conn, user, journal_entry)
        .await?
        .ok_or(JournalError::NotFound)?;

    let journal_entry =
        journal::journal_content::Mutation::add_journal_content(&conn, journal_entry.id, body.content).await?;
    Ok(Json(JournalContentId {
        id: journal_entry.last_insert_id,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v0/journal/entries/{journal_entry}/content/{content_id}",
    responses(
        (status = OK, description = "Get journal entry content body", body = JournalContent),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Journal", ty = "Permission")]
pub(crate) async fn get_journal_entry_content(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path((_journal_entry, journal_content)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, JournalError> {
    let journal_content = journal::journal_content::Query::get_user_journal_content(&conn, user, journal_content)
        .await?
        .ok_or(JournalError::NotFound)?;
    Ok(Json(JournalContent::from_db_model(journal_content)))
}

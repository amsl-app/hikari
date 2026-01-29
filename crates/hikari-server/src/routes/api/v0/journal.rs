pub(crate) mod assistant;
pub(crate) mod error;
pub(crate) mod journal_entry;
pub(crate) mod journal_focus;

use crate::permissions::Permission;
use crate::routes::api::v0::journal::error::JournalError;
use crate::user::ExtractUserId;
use axum::Extension;
use axum::Json;
use axum::response::IntoResponse;
use axum::routing::{Router, get, post};
use hikari_db::journal;
use hikari_db::sea_orm::DatabaseConnection;
use hikari_model::journal::partial::NewJournalEntryWithData;
use hikari_model::journal::{JournalEntry, MetaContent, MetaJournalEntryWithMetaContent};
use hikari_model_tools::convert::{FromDbModel, IntoModel};
use protect_axum::protect;

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .nest(
            "/entries",
            Router::new()
                .route("/", get(get_journal_entries).post(create_journal_entry))
                .route("/empty", post(create_empty_journal_entry))
                .nest("/{journal_entry}", journal_entry::create_router()),
        )
        .nest("/focus", journal_focus::create_router())
        .nest("/assistant", assistant::create_router())
        .with_state(())
}

#[utoipa::path(
    get,
    path = "/api/v0/journal/entries",
    responses(
        (status = OK, description = "List journal entries", body = [JournalEntry]),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Journal", ty = "Permission")]
pub(crate) async fn get_journal_entries(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, JournalError> {
    let journal_entries = journal::journal_entry::Query::get_user_journal_entries(&conn, user).await?;

    let journal_entries = journal_entries
        .into_iter()
        .map(FromDbModel::from_db_model)
        .collect::<Vec<JournalEntry>>();
    Ok(Json(journal_entries))
}

#[utoipa::path(
    post,
    path = "/api/v0/journal/entries/empty",
    responses(
        (status = OK, description = "Create journal entry", body = JournalEntry),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Journal", ty = "Permission")]
pub(crate) async fn create_empty_journal_entry(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, JournalError> {
    let journal_entry = journal::journal_entry::Mutation::create_journal_entry(&conn, user).await?;
    Ok(Json(JournalEntry::from_db_model(journal_entry)))
}

#[utoipa::path(
    post,
    path = "/api/v0/journal/entries",
    request_body = NewJournalEntryWithData,
    responses(
        (status = CREATED, description = "Create journal entry", body = MetaJournalEntryWithMetaContent),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Journal", ty = "Permission")]
pub(crate) async fn create_journal_entry(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Json(json): Json<NewJournalEntryWithData>,
) -> Result<impl IntoResponse, JournalError> {
    let entry = journal::journal_entry::Mutation::create_journal_entry_with_content(
        &conn,
        user,
        json.title,
        json.content.into_iter().map(|content| content.content).collect(),
        json.focus,
        json.mood,
        json.prompts,
    )
    .await?;
    let entry = MetaJournalEntryWithMetaContent {
        id: entry.id,
        user_id: entry.user_id,
        title: None,
        mood: entry.mood,
        created_at: entry.created_at,
        updated_at: entry.updated_at,
        content: entry
            .content
            .into_iter()
            .map(|content| MetaContent {
                id: content.id,
                journal_entry_id: entry.id,
                created_at: content.created_at,
                updated_at: content.updated_at,
            })
            .collect(),
        focus: entry.focus.into_iter().map(IntoModel::into_model).collect::<Vec<_>>(),
        prompts: vec![],
    };
    Ok(Json(entry))
}

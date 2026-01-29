use crate::routes::api::v0::journal::error::JournalError;
use crate::user::ExtractUserId;
use axum::Extension;
use axum::extract::Path;
use axum::response::{IntoResponse, Json};
use axum::routing::{Router, get};
use hikari_db::journal::journal_entry_journal_focus;
use hikari_db::sea_orm::DatabaseConnection;
use hikari_db::tag;
use hikari_model::tag::Tag;
use hikari_model_tools::convert::IntoModel;
use http::StatusCode;
use uuid::Uuid;

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/focus", get(get_focus).put(set_focus))
        .with_state(())
}

#[utoipa::path(
    get,
    path = "/api/v0/journal/entries/{journal_entry}/focus",
    responses(
        (status = OK, description = "Retrieve all foci for a journal entry.", content_type = "application/json", body = Vec<Tag>),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
pub(crate) async fn get_focus(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(journal_entry): Path<Uuid>,
) -> Result<impl IntoResponse, JournalError> {
    let focus = tag::Query::get_user_journal_entry_focus(&conn, user, journal_entry).await?;

    let focus = focus.into_iter().map(IntoModel::into_model).collect::<Vec<Tag>>();

    Ok(Json(focus))
}

#[utoipa::path(
    put,
    path = "/api/v0/journal/entries/{journal_entry}/focus",
    request_body(content = Vec<Uuid>, description = "Focus to set for the journal entry", content_type = "application/json"),
    responses(
        (status = NO_CONTENT, description = "Sets the focus for a journal entry (replaces current focus)."),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
pub(crate) async fn set_focus(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(journal_entry): Path<Uuid>,
    Json(focus): Json<Vec<Uuid>>,
) -> Result<impl IntoResponse, JournalError> {
    journal_entry_journal_focus::Mutation::set_user_journal_entry_focus(&conn, user, journal_entry, focus).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

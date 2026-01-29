use crate::permissions::Permission;
use crate::routes::api::v0::journal::error::JournalError;
use crate::user::ExtractUserId;
use axum::Extension;
use axum::extract::{Json, Path};
use axum::response::IntoResponse;
use axum::routing::{Router, get, patch};
use hikari_db::sea_orm::DatabaseConnection;
use hikari_db::tag;
use hikari_model::tag::Tag;
use hikari_model_tools::convert::{FromDbModel, IntoModel};
use http::StatusCode;
use protect_axum::protect;
use serde_derive::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(get_focus_incl_global))
        .nest(
            "/user",
            Router::new()
                .route("/", get(get_user_focus).post(create_user_focus))
                .route("/{focus_id}", patch(update_user_focus)),
        )
        .with_state(())
}

#[derive(Debug, Clone, ToSchema, Deserialize)]
pub(crate) struct NewFocus {
    name: String,
    icon: String,
}

#[utoipa::path(
    get,
    path = "/api/v0/journal/focus",
    responses(
        (status = OK, description = "Retrieve all foci including global foci.", content_type = "application/json", body = Vec<Tag>),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
pub(crate) async fn get_focus_incl_global(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, JournalError> {
    get_focus(user, conn, true).await
}

#[utoipa::path(
    get,
    path = "/api/v0/journal/focus/user",
    responses(
        (status = OK, description = "Retrieve user created foci.", content_type = "application/json", body = Vec<Tag>),
    ),
    tag = "v0/journal"
)]
#[protect("Permission::Journal", ty = "Permission")]
pub(crate) async fn get_user_focus(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, JournalError> {
    get_focus(user, conn, false).await
}

async fn get_focus(
    user: Uuid,
    conn: DatabaseConnection,
    include_global: bool,
) -> Result<impl IntoResponse, JournalError> {
    let focus = tag::Query::get_user_focuses(&conn, user, include_global).await?;

    let focus = focus.into_iter().map(IntoModel::into_model).collect::<Vec<Tag>>();

    Ok(Json(focus))
}

#[utoipa::path(
    post,
    path = "/api/v0/journal/focus/user",
    request_body(content = NewFocus, description = "The focus to create", content_type = "application/json"),
    responses(
        (status = CREATED, description = "Create a focus for a user.", content_type = "application/json", body = Tag),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Journal", ty = "Permission")]
pub(crate) async fn create_user_focus(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Json(new_focus): Json<NewFocus>,
) -> Result<impl IntoResponse, JournalError> {
    if new_focus.name.len() > 32 {
        return Err(JournalError::TooLarge("name".to_string()));
    }

    let focus = tag::Mutation::create_focus(&conn, Some(user), new_focus.name, new_focus.icon, false).await?;
    Ok(Json(Tag::from_db_model(focus)))
}

#[derive(Debug, Clone, ToSchema, Deserialize)]
pub(crate) struct FocusUpdate {
    name: Option<String>,
    icon: Option<String>,
    hidden: Option<bool>,
}

#[utoipa::path(
    patch,
    path = "/api/v0/journal/focus/user/{focus_id}",
    request_body(content = FocusUpdate, description = "Fields to update. All fields are optional. Missing values are not updated.", content_type = "application/json"),
    responses(
        (status = NO_CONTENT, description = "Focus updated"),
        (status = NOT_FOUND, description = "Focus not found or not owned by user"),
    ),
    tag = "v0/journal",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Journal", ty = "Permission")]
pub(crate) async fn update_user_focus(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(path): Path<Uuid>,
    Json(focus): Json<FocusUpdate>,
) -> Result<impl IntoResponse, JournalError> {
    if let Some(name) = &focus.name
        && name.len() > 32
    {
        return Err(JournalError::TooLarge("name".to_string()));
    }

    tag::Mutation::update_tag(&conn, path, user, focus.name, focus.icon, focus.hidden).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

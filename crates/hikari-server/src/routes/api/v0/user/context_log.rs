use crate::permissions::Permission;
use crate::routes::api::v0::modules::error::UserError;
use crate::user::ExtractUserId;
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Extension, Json, Router};
use hikari_db::sea_orm::DatabaseConnection;
use hikari_db::user_context_logs;
use hikari_model::user_context_log::UserContextLog;
use hikari_model_tools::convert::{FromDbModel, IntoModel};
use http::StatusCode;
use protect_axum::protect;
use serde_derive::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub(crate) struct TypePath {
    r#type: String,
}

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(get_user_context_logs))
        .route("/latest", get(get_latest_user_context_log))
        .nest(
            "/{type}",
            Router::new()
                .route("/", get(get_user_context_logs_by_type).post(add_user_context_log))
                .route("/latest", get(get_latest_user_context_log_by_type)),
        )
        .with_state(())
}

#[utoipa::path(
    get,
    path = "/api/v0/user/context_log",
    responses(
        (status = OK, body = Vec<UserContextLog>, description = "Returns all context logs for the current user, ordered by creation time descending"),
    ),
    tag = "v0/user",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn get_user_context_logs(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, UserError> {
    let logs: Vec<UserContextLog> = user_context_logs::Query::get_all(&conn, user_id)
        .await?
        .into_iter()
        .map(IntoModel::into_model)
        .collect();
    Ok(Json(logs))
}

#[utoipa::path(
    get,
    path = "/api/v0/user/context_log/latest",
    responses(
        (status = OK, body = UserContextLog, description = "Returns the most recent context log for the current user"),
        (status = NOT_FOUND, description = "No context log found"),
    ),
    tag = "v0/user",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn get_latest_user_context_log(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, UserError> {
    let log = user_context_logs::Query::get_latest(&conn, user_id).await?;
    log.map(|m| Json(UserContextLog::from_db_model(m)))
        .ok_or(UserError::NotFound)
}

#[utoipa::path(
    get,
    path = "/api/v0/user/context_log/{type}",
    responses(
        (status = OK, body = Vec<UserContextLog>, description = "Returns all context logs for the given type, ordered by creation time descending"),
    ),
    params(
        ("type" = String, Path, description = "the context log type to filter by"),
    ),
    tag = "v0/user",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn get_user_context_logs_by_type(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(path): Path<TypePath>,
) -> Result<impl IntoResponse, UserError> {
    let logs: Vec<UserContextLog> = user_context_logs::Query::get_all_by_type(&conn, user_id, &path.r#type)
        .await?
        .into_iter()
        .map(IntoModel::into_model)
        .collect();
    Ok(Json(logs))
}

#[utoipa::path(
    post,
    path = "/api/v0/user/context_log/{type}",
    request_body = Value,
    responses(
        (status = CREATED, description = "Context log entry stored successfully"),
    ),
    params(
        ("type" = String, Path, description = "the context log type"),
    ),
    tag = "v0/user",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn add_user_context_log(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(path): Path<TypePath>,
    Json(body): Json<Value>,
) -> Result<impl IntoResponse, UserError> {
    user_context_logs::Mutation::insert(&conn, user_id, path.r#type, body).await?;
    Ok(StatusCode::CREATED)
}

#[utoipa::path(
    get,
    path = "/api/v0/user/context_log/{type}/latest",
    responses(
        (status = OK, body = UserContextLog, description = "Returns the most recent context log for the given type"),
        (status = NOT_FOUND, description = "No context log found for the given type"),
    ),
    params(
        ("type" = String, Path, description = "the context log type to filter by"),
    ),
    tag = "v0/user",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn get_latest_user_context_log_by_type(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(path): Path<TypePath>,
) -> Result<impl IntoResponse, UserError> {
    let log = user_context_logs::Query::get_latest_by_type(&conn, user_id, &path.r#type).await?;
    log.map(|m| Json(UserContextLog::from_db_model(m)))
        .ok_or(UserError::NotFound)
}

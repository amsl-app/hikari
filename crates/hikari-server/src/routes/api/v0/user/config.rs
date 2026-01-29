use crate::AppConfig;
use crate::permissions::Permission;
use crate::routes::api::v0::modules::error::UserError;

use crate::user::ExtractUserId;
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Extension, Json, Router};
use hikari_config::global::GlobalConfig;
use hikari_db::config;
use http::{HeaderValue, StatusCode, header};
use protect_axum::protect;
use sea_orm::ConnectionTrait;
use serde_derive::Deserialize;
use serde_json::{Map, Value};
use uuid::Uuid;

use hikari_db::sea_orm::DatabaseConnection;

#[derive(Deserialize)]
pub(crate) struct KeyPath {
    key: String,
}

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(get_user_configs))
        .route(
            "/{key}",
            get(get_user_config_value)
                .put(set_user_config)
                .delete(delete_user_config),
        )
        .with_state(())
}

#[utoipa::path(
    get,
    path = "/api/v0/user/config",
    responses(
        (status = OK, description = "Returns a key, value json object, with the stored configs"),
    ),
    tag = "v0/user",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]

pub(crate) async fn get_user_configs(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, UserError> {
    let configs = config::Query::get_user_config(&conn, user_id).await?;

    let res = configs
        .into_iter()
        .map(|entry| Ok((entry.key, serde_json::from_str(&entry.value)?)))
        .collect::<Result<Map<String, Value>, serde_json::Error>>()?;

    Ok(Json(res))
}

#[utoipa::path(
    get,
    path = "/api/v0/user/config/{key}",
    responses(
        (status = OK, description = "Returns the value of the given key"),
    ),
    params(
        ("key" = String, Path, description = "the key of the value, which should be loaded"),
    ),
    tag = "v0/user",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]

pub(crate) async fn get_user_config_value(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(path): Path<KeyPath>,
) -> Result<impl IntoResponse, UserError> {
    let res = read_user_config(&conn, user_id, &path.key).await?;
    // We set the mime type manually here instead of using Json(...) because
    // the database returns a json string and wrapping it in Json(...) would
    // result in a double json encoded response.
    // E.g "\"foo\"" instead of "foo"
    Ok((
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
        )],
        res,
    ))
}

pub(crate) async fn read_user_config(db: &DatabaseConnection, user_id: Uuid, key: &str) -> Result<String, UserError> {
    let config = config::Query::get_config_value(db, user_id, key).await?;

    config.ok_or(UserError::NotFound)
}

#[utoipa::path(
    put,
    path = "/api/v0/user/config/{key}",
    responses(
        (status = CREATED, description = "Stores the body as value for the given key, note overrides existing values"),
    ),
    params(
        ("key" = String, Path, description = "the key which should be used to upsert"),
    ),
    tag = "v0/user",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]

pub(crate) async fn set_user_config(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Extension(app_config): Extension<AppConfig>,
    Path(path): Path<KeyPath>,
    Json(body): Json<Value>,
) -> Result<impl IntoResponse, UserError> {
    let cfg = app_config.config();
    update_user_config(&conn, user_id, cfg, path.key, &body).await?;
    Ok(StatusCode::CREATED)
}

#[utoipa::path(
    delete,
    path = "/api/v0/user/configs/{key}",
    responses(
        (status = NO_CONTENT, description = "Deletes given key from users config"),
    ),
    params(
        ("key" = String, Path, description = "the key which should be deleted"),
    ),
    tag = "v0/user",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]

pub(crate) async fn delete_user_config(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(path): Path<KeyPath>,
) -> Result<impl IntoResponse, UserError> {
    config::Mutation::delete_config_value(&conn, user_id, path.key).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn update_user_config<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    cfg: &GlobalConfig,
    key: String,
    value: &Value,
) -> Result<(), UserError> {
    let user_cfg = cfg.config();
    let allowed = user_cfg.allowed_keys.contains(&key);
    if !allowed {
        return Err(UserError::InvalidKey);
    }

    config::Mutation::set_config_value(conn, user_id, key, serde_json::to_string(value)?).await?;
    Ok(())
}

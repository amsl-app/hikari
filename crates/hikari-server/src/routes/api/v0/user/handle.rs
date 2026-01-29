use crate::permissions::Permission;
use crate::routes::api::v0::modules::error::UserError;
use crate::user::ExtractUserId;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Extension, Json, Router};
use hikari_db::user_handle::{Mutation, Query, RandomHandleGenerator};
use hikari_model::user_handle::UserHandle;
use hikari_model_tools::convert::FromDbModel;
use protect_axum::protect;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr};
use std::sync::Arc;
use tokio::sync::RwLock;

async fn get_handle_length<C: ConnectionTrait>(
    handle_length_lock: &RwLock<Option<usize>>,
    db: &C,
) -> Result<usize, DbErr> {
    let handle_length = handle_length_lock.read().await;
    match *handle_length {
        None => {
            drop(handle_length);
            // Query first so we don't have to lock the RwLock for the whole function
            let len = Query::get_max_handle_length(db).await?.unwrap_or(2);
            let mut handle_length = handle_length_lock.write().await;
            // Recheck because another thread might have already set it
            if let Some(len) = *handle_length {
                return Ok(len);
            }
            *handle_length = Some(len);
            Ok(len)
        }
        Some(len) => Ok(len),
    }
}

async fn write_handle_length(handle_length_lock: &RwLock<Option<usize>>, len: usize) {
    let mut handle_length = handle_length_lock.write().await;
    // Recheck because another thread might have already set it
    if let Some(old_len) = *handle_length
        && len <= old_len
    {
        // We don't want to shrink the handle length and we can skip writing
        return;
    }
    *handle_length = Some(len);
    drop(handle_length);
    tracing::info!(%len, "updated handle length");
}

async fn update_handle_length(handle_length_lock: &RwLock<Option<usize>>, len: usize) {
    let handle_length = handle_length_lock.read().await;
    match *handle_length {
        None => {
            drop(handle_length);
            write_handle_length(handle_length_lock, len).await;
        }
        Some(old_len) => {
            if len > old_len {
                drop(handle_length);
                write_handle_length(handle_length_lock, len).await;
            }
        }
    }
}

type RouterState = Arc<RwLock<Option<usize>>>;

pub fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", post(get_user_handle))
        .with_state(Arc::new(RwLock::new(None)))
}

/// Gets or crates a handle for the user
#[utoipa::path(
    post,
    path = "/api/v0/user/handle",
    responses(
        (status = OK, body = UserHandle, description = "The user handle"),
    ),
    tag = "v0/user",
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]
pub(crate) async fn get_user_handle(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    State(handle_length_lock): State<RouterState>,
) -> Result<impl IntoResponse, UserError> {
    let len = get_handle_length(&handle_length_lock, &conn).await?;
    let model = Mutation::get_or_create_handle::<RandomHandleGenerator, _>(&conn, user_id, len).await?;
    let new_len = model.handle.len();
    // Avoid going through the RwLock if we don't need to
    if new_len > len {
        update_handle_length(&handle_length_lock, new_len).await;
    }
    Ok(Json(UserHandle::from_db_model(model)))
}

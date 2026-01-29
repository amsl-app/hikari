use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use hikari_db::sea_orm::DatabaseConnection;
use protect_axum::protect;

use super::error::MessageError;
use crate::AppConfig;
use crate::permissions::Permission;
use crate::user::ExtractUser;

use hikari_model::chat::Request;

#[utoipa::path(
    post,
    request_body = Request,
    path = "/api/v0/bots/{id}/message",
    tag = "v0/bots",
    responses(
        (status = OK, description = "Send a message to a bot to continue a conversation"),
        (status = NOT_FOUND, description = "Bot could not be found"),
    ),
    params(
        ("id" = String, Path, description = "ID of the bot to message"),
    ),
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]
#[allow(clippy::type_complexity)]
pub(crate) async fn message(
    ExtractUser(user): ExtractUser,
    Extension(conn): Extension<DatabaseConnection>,
    Extension(app_config): Extension<AppConfig>,
    Path(id): Path<String>,
    Json(request): Json<Request>,
) -> Result<impl IntoResponse, MessageError> {
    let payload = request.payload;
    let metadata = request.metadata.unwrap_or_default();

    Ok(Json(
        super::start_conversation(user, &id, request.client, payload, metadata, app_config.bots(), &conn).await?,
    ))
}

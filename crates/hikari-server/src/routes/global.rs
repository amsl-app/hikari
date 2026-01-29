use crate::AppConfig;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Extension, Json, Router};
use hikari_config::global::frontend::FrontendConfig;

pub fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/version", get(frontend_version)).with_state(())
}

#[utoipa::path(
    get,
    path = "/version",
    responses(
        (status = OK, body = Option<FrontendConfig>, description = "returns information about the required version")
    ),
    tag = "util"
)]
pub(crate) async fn frontend_version(app_config: Extension<AppConfig>) -> impl IntoResponse {
    let response = app_config.config().frontend();
    Json(response).into_response()
}

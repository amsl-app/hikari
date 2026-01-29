use crate::AppConfig;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Extension, Json, Router};
use futures::future::join;
use hikari_core::status::get_sea_orm_db_status;
use hikari_model::status::ComponentStatus;
use http::StatusCode;
use reqwest_tracing::{DefaultSpanBackend, TracingMiddleware};
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};
use std::error::Error;
use std::time::Duration;
use tracing::instrument;
use url::Url;
use utoipa::ToSchema;

#[instrument(skip_all)]
async fn get_worker_status(base_url: &Url) -> ComponentStatus {
    let url = match base_url.join("api/v0/health") {
        Ok(url) => url,
        Err(error) => {
            tracing::error!(
                error = &error as &dyn Error,
                "configuration error: worker url is not valid"
            );
            return ComponentStatus::from_error_text("Error, connecting to worker");
        }
    };
    // We unwrap here because this can only fail if the TLS backend can't be initialized
    // Timeout is 10 seconds because the db timeout of the worker is 5 seconds
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build reqwest client");
    let middleware_client = reqwest_middleware::ClientBuilder::new(client)
        .with(TracingMiddleware::<DefaultSpanBackend>::new())
        .build();
    let resp = match middleware_client.get(url).send().await {
        Ok(resp) => resp,
        Err(error) => {
            tracing::warn!(error = &error as &dyn Error, "error getting worker status");
            return ComponentStatus::from_error_text("error getting worker status");
        }
    };
    let status = resp.status();
    let data: Value = match resp.json().await {
        Ok(data) => data,
        Err(error) => {
            tracing::error!(error = &error as &dyn Error, "error parsing worker status");
            return ComponentStatus::from_error_text("error parsing worker status");
        }
    };
    ComponentStatus::new(status, Some(data))
}

pub fn create_router<S>() -> Router<S> {
    Router::new().route("/", get(get_status)).with_state(())
}

#[derive(Debug, Clone, ToSchema)]
struct Status {
    database: ComponentStatus,
    worker: ComponentStatus,
}

impl Status {
    pub(crate) fn status_code(&self) -> StatusCode {
        if self.database.is_ok() && self.worker.is_ok() {
            StatusCode::OK
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

impl From<Status> for hikari_model::status::Status {
    fn from(val: Status) -> Self {
        hikari_model::status::Status {
            database: val.database.into_message(),
            worker: val.worker.into_message(),
        }
    }
}

impl IntoResponse for Status {
    fn into_response(self) -> Response {
        let status_code = self.status_code();
        let status: hikari_model::status::Status = self.into();
        (status_code, Json(status)).into_response()
    }
}

#[utoipa::path(
    get,
    path = "/api/v0/status",
    responses(
        (status = OK, description = "Server is ok", body = Status, example = json!( hikari_model::status::Status { database: json!("ok"), worker: json!({ "database": "ok" }) } )),
    ),
    tag = "util"
)]
#[instrument(skip_all)]
pub(crate) async fn get_status(
    Extension(conn): Extension<DatabaseConnection>,
    Extension(app_config): Extension<AppConfig>,
) -> impl IntoResponse {
    let worker_url = app_config.worker_url();
    let (sea_orm_status, worker_status) = join(get_sea_orm_db_status(&conn, None), get_worker_status(worker_url)).await;

    Status {
        database: sea_orm_status,
        worker: worker_status,
    }
}

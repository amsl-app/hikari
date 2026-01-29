pub(crate) mod csml;
pub(crate) mod journal;

use axum::response::IntoResponse;
use axum::{Extension, Json};
use hikari_core::status::get_sea_orm_db_status;
use hikari_model::status::ComponentStatus;
use http::StatusCode;
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Expr;
use sea_orm::sea_query::Query;
use serde::Serialize;
use std::fmt::Debug;
use tracing::instrument;
use utoipa::ToSchema;

#[derive(Debug, Clone, ToSchema, Serialize)]
pub(crate) struct Status {
    database: ComponentStatus,
}

impl Status {
    fn status_code(&self) -> StatusCode {
        if self.database.is_ok() {
            StatusCode::OK
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

impl IntoResponse for Status {
    fn into_response(self) -> axum::response::Response {
        let status_code = self.status_code();
        (status_code, Json(self)).into_response()
    }
}

#[utoipa::path(
    get,
    path = "/api/v0/health",
    responses(
        (status = OK, description = "Server is ok", body = Status, example = json!( Status { database: ComponentStatus::ok() } )),
    ),
    tag = "util"
)]
#[instrument(skip_all)]
pub async fn get_health(Extension(conn): Extension<DatabaseConnection>) -> impl IntoResponse {
    let mut query = Query::select();
    query.expr(Expr::current_timestamp());

    Status {
        database: get_sea_orm_db_status(&conn, None).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_serialization() {
        let status = Status {
            database: ComponentStatus::ok(),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#"{"database":"ok"}"#);
    }

    #[test]
    fn test_message_serialization() {
        let status = Status {
            database: ComponentStatus::from_ok_text("hi"),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#"{"database":"hi"}"#);
    }
}

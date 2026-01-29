use crate::app::AuthConfig;
use crate::auth::validate_jwt;
use crate::db;
use crate::routes::error::{ErrorData, LoginError, LoginErrorType};
use crate::user::ExtractUserId;
use anyhow::Result;
use axum::body::{Body, Bytes};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Router};
use hikari_db::access_tokens;
use hikari_model::login::Token;
use http::StatusCode;
use http::header;
use sea_orm::DatabaseConnection;
use std::borrow::Cow;
use std::error::Error;
use std::str;
use std::str::from_utf8;
use tracing;

#[allow(clippy::too_many_arguments)]
pub fn create_router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/whoami", get(whoami))
        .route("/logout", post(logout))
        .nest("/login", Router::new().route("/token", post(login_token)))
        .with_state(())
}

#[utoipa::path(
    post,
    path = "/login/token",
    request_body(content = String, description = "The plain jwt token received from auth-server", content_type = "text/plain"),
    responses(
        (status = OK, description = "Successful login, returns Bearer token", body = Token, example = json!( Token { access_token: "abcToken12345678".into() })),
        (status = UNAUTHORIZED, description = "Authentication failed. Possible reason may be that the token is expired.", body = ErrorData<LoginErrorType>),
        (status = FORBIDDEN, description = "Authentication succeeded but access was denied. See body for reason", body = ErrorData<LoginErrorType>),
    ),
    tag = "util"
)]
pub(crate) async fn login_token(
    Extension(state): Extension<AuthConfig>,
    Extension(conn): Extension<DatabaseConnection>,
    token: Bytes,
) -> Result<Response, LoginError> {
    let state = state.as_ref();
    let token =
        from_utf8(&token).inspect_err(|error| tracing::warn!(error = error as &dyn Error, "could not read token"))?;

    let (sub, groups) = validate_jwt(
        token,
        state.audience(),
        state.required_claims(),
        state.groups_claim(),
        state.groups(),
        state.jwk(),
    )
    .await?
    .ok_or_else(|| {
        tracing::warn!("authentication failed: invalid token");
        LoginError::Invalid
    })?;

    let access_token = db::sea_orm::user::create_user_and_get_token(&conn, &sub, groups).await?;

    let mut response = Response::builder();
    response = response.status(StatusCode::OK);
    response = response.header(header::CONTENT_TYPE, "application/json");

    let res = response
        .body(Body::from(
            serde_json::to_string(&Token {
                access_token: access_token.access_token,
            })
            .map_err(|error| {
                tracing::error!(error = &error as &dyn Error, "could not serialize token");
                LoginError::ResponseError
            })?,
        ))
        .map_err(|error| {
            tracing::error!(error = &error as &dyn Error, "could not create response");
            LoginError::ResponseError
        })?;

    Ok(res)
}

async fn whoami(user: Option<ExtractUserId>) -> impl IntoResponse {
    match user {
        None => {
            tracing::debug!("no user found");
            (StatusCode::NOT_FOUND, Cow::Borrowed("no user"))
        }
        Some(user) => (StatusCode::OK, Cow::Owned(format!("Hello {}", user.0))),
    }
}

#[utoipa::path(
    post,
    path = "/logout",
    responses(
        (status = NO_CONTENT, description = "User Logged out successfully"),
        (status = INTERNAL_SERVER_ERROR, description = "Failed to delete access token")
    ),
    tag = "oidc",
    security(
        ("token" = [])
    )
)]
pub(crate) async fn logout(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
) -> impl IntoResponse {
    if let Err(error) = access_tokens::Mutation::delete_access_token(&conn, user_id).await {
        tracing::error!(
            user = %user_id,
            error = &error as &dyn Error,
            "failed to delete access token"
        );
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    tracing::debug!(user = %user_id, "user logged out");
    StatusCode::NO_CONTENT
}

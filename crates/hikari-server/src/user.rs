use crate::app::AuthConfig;
use crate::auth::{AuthError, validate_jwt};
use crate::db::sea_orm::user::create_user;
use axum::extract::{FromRequestParts, OptionalFromRequestParts};
use axum::{Extension, RequestPartsExt};
use axum_auth::AuthBearer;
use axum_extra::extract::Cached;
use hikari_db::user;
use hikari_model::user::User;
use hikari_model_tools::convert::{TryFromDbModel, TryIntoModel};
use http::StatusCode;
use http::request::Parts;

use sea_orm::DatabaseConnection;
use std::error::Error;
use url::form_urlencoded;
use uuid::Uuid;

pub fn extract_auth_token_from_params(parts: &mut Parts) -> Option<String> {
    if let Some(query) = parts.uri.query() {
        for (key, value) in form_urlencoded::parse(query.as_bytes()) {
            if key == "access_token" {
                return Some(value.to_string());
            }
        }
    }
    None
}

type Rejection = (StatusCode, &'static str);

#[derive(Clone)]
struct Session {
    user: User,
}

#[derive(Clone)]
pub(crate) struct ExtractUser(pub User);

#[derive(Clone)]
pub(crate) struct ExtractUserId(pub Uuid);

impl<S> FromRequestParts<S> for Session
where
    S: Send + Sync,
{
    type Rejection = Rejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to extract token from Authorization header first
        let token = if let Ok(AuthBearer(token)) = parts.extract::<AuthBearer>().await {
            token
        } else if let Some(token) = extract_auth_token_from_params(parts) {
            token
        } else {
            return Err((StatusCode::UNAUTHORIZED, "No authentication token provided"));
        };

        let Ok(app_state) = parts.extract::<Option<Extension<AuthConfig>>>().await;

        let Extension::<DatabaseConnection>(conn) =
            parts
                .extract::<Extension<DatabaseConnection>>()
                .await
                .map_err(|error| {
                    tracing::error!(
                        error = &error as &dyn Error,
                        "database connection not found in app data"
                    );
                    (StatusCode::INTERNAL_SERVER_ERROR, "Database Connection not found")
                })?;
        if let Some(Extension(app_state)) = app_state {
            let state = app_state.as_ref();

            match validate_jwt(
                &token,
                state.audience(),
                state.required_claims(),
                state.groups_claim(),
                state.groups(),
                state.jwk(),
            )
            .await
            {
                Ok(Some((sub, groups))) => {
                    let (user, custom_groups) = create_user(&conn, &sub, groups.clone()).await.map_err(|error| {
                        tracing::error!(error = &error as &dyn Error, "failed to create user");
                        (StatusCode::INTERNAL_SERVER_ERROR, "Error creating user")
                    })?;
                    let user_id = user.id;
                    let user = User::try_from_db_model((user, groups, custom_groups)).map_err(|error| {
                        tracing::error!(error = &error as &dyn Error, %user_id, "failed to create user");
                        (StatusCode::INTERNAL_SERVER_ERROR, "Error to decode db user")
                    })?;
                    return Ok(Self { user });
                }
                Err(err) => {
                    tracing::error!(error = &err as &dyn Error, "error validating token");
                    if matches!(err, AuthError::Unauthorized) {
                        return Err((StatusCode::UNAUTHORIZED, "Invalid token claims"));
                    }
                    return Err((StatusCode::INTERNAL_SERVER_ERROR, "Error validating token"));
                }
                Ok(None) => {
                    tracing::debug!("JWT authentication failed, try from DB");
                }
            }
        } else {
            tracing::warn!("No app state found for JWT authentication");
        }

        Self::from_db(&conn, &token).await.map(|user| Self { user })
    }
}

impl Session {
    async fn from_db(conn: &DatabaseConnection, token: &str) -> Result<User, Rejection> {
        let Ok(Some(user)) = user::Query::find_by_token(conn, token).await else {
            return Err((StatusCode::UNAUTHORIZED, "Authentication failed."));
        };

        sentry::configure_scope(|scope| {
            scope.set_user(Some(sentry::User {
                id: Some(user.0.id.as_hyphenated().to_string()),
                ..Default::default()
            }));
        });

        user.try_into_model().map_err(|error| {
            tracing::error!(error = &error as &dyn Error, "error converting user");
            (StatusCode::INTERNAL_SERVER_ERROR, "Error loading user")
        })
    }
}

impl<S> OptionalFromRequestParts<S> for ExtractUser
where
    S: Send + Sync,
{
    type Rejection = Rejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Option<Self>, Self::Rejection> {
        // TODO fail if the session information is present but invalid
        let Ok(session) = Cached::<Session>::from_request_parts(parts, state).await else {
            return Ok(None);
        };
        Ok(Some(Self(session.0.user)))
    }
}

impl<S> FromRequestParts<S> for ExtractUser
where
    S: Send + Sync,
{
    type Rejection = Rejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session: Session = Cached::<Session>::from_request_parts(parts, state).await?.0;
        Ok(Self(session.user))
    }
}

impl<S> OptionalFromRequestParts<S> for ExtractUserId
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Option<Self>, Self::Rejection> {
        // TODO fail if the session information is present but invalid
        let session: Session = match Cached::<Session>::from_request_parts(parts, state).await {
            Ok(session) => session.0,
            Err(_) => return Ok(None),
        };
        Ok(Some(Self(session.user.id)))
    }
}

impl<S> FromRequestParts<S> for ExtractUserId
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session: Session = Cached::<Session>::from_request_parts(parts, state).await?.0;
        Ok(Self(session.user.id))
    }
}

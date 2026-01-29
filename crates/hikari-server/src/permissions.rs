use crate::user::ExtractUser;
use axum::RequestExt;
use axum::extract::{FromRequestParts, Request};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::Cached;
use http::StatusCode;
use http::request::Parts;
use serde_derive::Serialize;
use std::collections::HashSet;
use std::ops::Not;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Serialize)]
pub(crate) enum Permission {
    Basic,   // like a user
    Journal, // for journal features
    Beta,    // for beta features
}

#[derive(PartialEq, Eq, Clone, Debug, Default)]
struct Session {
    permissions: HashSet<Permission>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize)]
pub(crate) struct Permissions(HashSet<Permission>);

impl<S> FromRequestParts<S> for Session
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = ExtractUser::from_request_parts(parts, state).await;
        let Ok(ExtractUser(user)) = user else {
            return Ok(Session::default());
        };
        let permissions: Permissions = user.groups.into();
        Ok(Session {
            permissions: permissions.0,
        })
    }
}

const POSITIVE_PERMISSION_MAP: &[(&str, Permission)] = &[("beta", Permission::Beta)];

const NEGATIVE_PERMISSION_MAP: &[(&str, Permission)] = &[("no-journal", Permission::Journal)];

impl From<Vec<String>> for Permissions {
    fn from(groups: Vec<String>) -> Self {
        let groups: HashSet<_> = groups.into_iter().collect();
        let permissions: HashSet<Permission> = std::iter::once(Permission::Basic)
            .chain(
                POSITIVE_PERMISSION_MAP
                    .iter()
                    .filter_map(|(group, permission)| groups.contains(*group).then_some(*permission)),
            )
            .chain(
                NEGATIVE_PERMISSION_MAP
                    .iter()
                    .filter_map(|(group, permission)| groups.contains(*group).not().then_some(*permission)),
            )
            .collect();
        Self(permissions)
    }
}

impl<S> FromRequestParts<S> for Permissions
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Cached::<Session>::from_request_parts(parts, state).await?.0;
        Ok(Self(session.permissions))
    }
}

pub(crate) async fn extract(request: &mut Request) -> Result<HashSet<Permission>, Response> {
    request
        .extract_parts::<Permissions>()
        .await
        .map(|permissions| permissions.0)
        .map_err(IntoResponse::into_response)
}

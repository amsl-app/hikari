use crate::AppConfig;
use crate::permissions::Permission;
use crate::routes::api::v0::modules::error::UserError;
use crate::user::ExtractUser;
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use hikari_config::global::access::GroupAccess;
use hikari_db::groups::{self, groups_token};
use hikari_db::sea_orm::DatabaseConnection;
use hikari_db::util::FlattenTransactionResultExt;
use protect_axum::protect;
use rand::rng;
use rand::seq::IndexedRandom;
use sea_orm::{DbErr, TransactionTrait};
use serde::Serialize;
use serde_derive::Deserialize;
use std::error::Error;
use utoipa::ToSchema;

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", post(add_access))
        .route("/{token}/approvals", get(access_approvals))
        .with_state(())
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct GroupToken {
    token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AccessApproval {
    pub declaration_of_consent: String,
    pub privacy_policy: String,
    pub participant_information: Option<String>,
}

impl From<hikari_config::global::access::AccessApproval> for AccessApproval {
    fn from(value: hikari_config::global::access::AccessApproval) -> Self {
        Self {
            declaration_of_consent: value.declaration_of_consent.to_string(),
            privacy_policy: value.privacy_policy.to_string(),
            participant_information: value.participant_information.map(|s| s.to_string()),
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v0/user/access/{token}/approvals",
    responses((status = OK, body = AccessApproval, description = "the approvals which are needed for the access token"),
    ),
    tag = "v0/user",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn access_approvals(
    Extension(config): Extension<AppConfig>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, UserError> {
    let access_config = config.config().access();
    let approvals: Option<AccessApproval> = access_config
        .iter()
        .find(|group| group.token == token)
        .and_then(|a| a.approvals.clone().map(AccessApproval::from));
    Ok(Json(approvals))
}

#[utoipa::path(
    post,
    request_body = GroupToken,
    path = "/api/v0/user/access",
    responses((status = OK, description = "adds the access token to the user's account"),
    ),
    tag = "v0/user",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn add_access(
    ExtractUser(user): ExtractUser,
    Extension(conn): Extension<DatabaseConnection>,
    Extension(config): Extension<AppConfig>,
    Json(GroupToken { token }): Json<GroupToken>,
) -> Result<impl IntoResponse, UserError> {
    let access_config = config.config().access();
    let Some(access_index) = access_config.iter().position(|group| group.token == token) else {
        tracing::warn!(token = %token, "invalid token");
        return Err(UserError::InvalidToken);
    };

    let res = conn
        .transaction::<_, http::status::StatusCode, UserError>(|txn| {
            Box::pin(async move {
                // Doing the lookup here avoids having to clone the groups
                // The index access is safe because we just found it above
                let groups = &config
                    .config()
                    .access()
                    .get(access_index)
                    .expect("index lookup - this is a bug")
                    .groups;
                // Add token and check if it already exists
                let res = groups_token::Mutation::add(txn, user.id, token.clone()).await;
                if let Err(error) = res {
                    tracing::warn!(error = &error as &dyn Error, "error adding group");
                    if DbErr::RecordNotInserted == error {
                        // Token already exists, but it's not a problem
                        return Ok(http::status::StatusCode::OK);
                    }
                    return Err(UserError::from(error));
                }

                for group in groups {
                    let name = match group {
                        GroupAccess::Single(value) => value,
                        GroupAccess::Random { random } => select_group(random.as_slice())?,
                    };
                    groups::custom_groups::Mutation::add(txn, user.id, name.to_owned()).await?;
                }
                Ok(http::status::StatusCode::OK)
            })
        })
        .await;

    res.flatten_res()
}

fn select_group(groups: &[String]) -> Result<&str, UserError> {
    let mut rng = rng();
    let group = groups.choose(&mut rng).ok_or(UserError::NoGroupsToSelect)?;
    Ok(group)
}

// test

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_group() {
        let groups = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
        let group = select_group(groups.as_slice()).unwrap();
        assert!(group == "a" || group == "b" || group == "c");
    }

    #[test]
    fn test_single_element() {
        let groups = vec!["a".to_owned()];
        let group = select_group(groups.as_slice()).unwrap();
        assert_eq!(group, "a");
    }
}

use crate::permissions::Permission;
use crate::routes::api::v0::modules::error::UserError;
use crate::user::{ExtractUser, ExtractUserId};
use axum::response::IntoResponse;
use axum::routing::{delete, get};
use axum::{Extension, Json, Router};
use chrono::NaiveDate;
use hikari_db::sea_orm::DatabaseConnection;
use hikari_db::user;
use hikari_model::user::{Gender, User};
use hikari_model_tools::convert::IntoDbModel;
use protect_axum::protect;
use sea_orm::ActiveValue;
use serde_derive::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

pub(crate) mod access;
pub(crate) mod config;
pub(crate) mod handle;

pub(crate) fn create_router<S>(deletable: bool) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let mut router = Router::new()
        .route("/", get(get_user_info).patch(update_user_info))
        .nest("/handle", handle::create_router())
        .nest("/config", config::create_router())
        .nest("/access", access::create_router());

    if deletable {
        router = router.route("/delete", delete(delete_user));
    }

    router.with_state(())
}

pub(crate) async fn delete_user(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, UserError> {
    user::Mutation::delete(&conn, user_id).await?;

    tracing::debug!(%user_id, "user deleted!");

    Ok(http::StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UserChanges {
    #[serde(default, with = "::serde_with::rust::double_option")]
    #[allow(clippy::option_option)] // Out Option = Set/Unset, Inner Option = Some/None
    name: Option<Option<String>>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    #[allow(clippy::option_option)] // Out Option = Set/Unset, Inner Option = Some/None
    birthday: Option<Option<NaiveDate>>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    #[allow(clippy::option_option)] // Out Option = Set/Unset, Inner Option = Some/None
    subject: Option<Option<String>>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    #[allow(clippy::option_option)] // Out Option = Set/Unset, Inner Option = Some/None
    semester: Option<Option<u8>>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    #[allow(clippy::option_option)] // Out Option = Set/Unset, Inner Option = Some/None
    gender: Option<Option<Gender>>,
    onboarding: Option<bool>,
}

pub(crate) fn apply_changes(id: Uuid, changes: UserChanges) -> hikari_entity::user::ActiveModel {
    let mut active_user = hikari_entity::user::ActiveModel {
        id: ActiveValue::Unchanged(id),
        ..hikari_entity::user::ActiveModel::default()
    };

    macro_rules! apply_change {
        ($i:ident, $conv:expr_2021) => {
            if let Some(inner) = changes.$i {
                match inner {
                    Some($i) => {
                        tracing::debug!(new_value = ?$i, concat!("changed user ", stringify!($i)));
                        active_user.$i = ActiveValue::Set(Some($conv));
                    }
                    None => {
                        tracing::debug!(concat!("unset user field ", stringify!($i)));
                        active_user.$i = ActiveValue::Set(None);
                    }
                }
            };
        };
        ($i:ident) => {
            apply_change!($i, $i.into())
        };
    }

    apply_change!(name);
    apply_change!(birthday);
    apply_change!(subject);
    apply_change!(semester);
    apply_change!(gender, IntoDbModel::into_db_model(gender));

    if let Some(on) = changes.onboarding {
        tracing::debug!(new_value = %on, "changed user onboarding");
        active_user.onboarding = ActiveValue::Set(on);
    }

    active_user
}

#[utoipa::path(
    get,
    path = "/api/v0/user",
    responses(
        (status = OK, body = User, description = "returns information that is stored about the current user"),
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
pub(crate) async fn get_user_info(ExtractUser(user): ExtractUser) -> Result<impl IntoResponse, UserError> {
    let user: User = user;
    Ok(Json(user))
}

#[utoipa::path(
    patch,
    request_body = UserChanges,
    path = "/api/v0/user",
    responses(
        (status = CREATED, description = "stores the changes listed in the request body to current user"),
    ),
    tag = "v0/user",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn update_user_info(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Json(changes): Json<UserChanges>,
) -> Result<impl IntoResponse, UserError> {
    tracing::debug!(user_changes = ?changes, "preparing to save user changes to db");
    user::Mutation::update_user(&conn, apply_changes(user_id, changes)).await?;

    Ok(http::status::StatusCode::CREATED)
}

use crate::AppConfig;
use crate::data::assessment::{AnswerRequest, get_or_start_session, require_running_session, submit_session};
use crate::data::modules::assessment::get_module_assessment;
use crate::permissions::Permission;
use crate::routes::api::v0::assessment::SessionResponse;
use crate::routes::api::v0::assessment::error::Error;
use crate::routes::api::v0::modules::error::ModuleError;
use crate::user::ExtractUser;
use axum::extract::Path;
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use axum::{Extension, Router};
use chrono::NaiveDateTime;
use hikari_config::module::assessment::ModuleAssessment;
use hikari_entity::assessment::session::Model as AssessmentSession;
use http::StatusCode;
use protect_axum::protect;
use sea_orm::DatabaseConnection;
use serde_derive::{Deserialize, Serialize};
use strum::IntoStaticStr;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, Debug, Copy, Clone, IntoStaticStr, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PrePost {
    Pre,
    Post,
}

impl PrePost {
    pub(crate) fn get_assessment_id<'a>(self, assessment: &'a ModuleAssessment) -> &'a str {
        match self {
            Self::Pre => &assessment.pre,
            Self::Post => &assessment.post,
        }
    }
}

#[derive(Serialize, Debug, ToSchema)]
pub(crate) struct StartModuleAssessmentResponse {
    #[serde(flatten)]
    session: SessionResponse,
    assessment_id: String,
}

#[derive(Serialize, Debug, Default, ToSchema)]
pub(crate) struct ModuleAssessmentResponse {
    session_id: Option<Uuid>,
    assessment_id: Option<String>,
    status: AssessmentStatus,
    completed: Option<NaiveDateTime>,
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, ToSchema)]
pub(crate) enum AssessmentStatus {
    #[serde(rename = "not_started")]
    #[default]
    NotStarted = 1,
    #[serde(rename = "running")]
    Running = 2,
    #[serde(rename = "finished")]
    Finished = 3,
}

impl From<AssessmentSession> for ModuleAssessmentResponse {
    fn from(value: AssessmentSession) -> Self {
        Self {
            session_id: Some(value.id),
            assessment_id: Some(value.assessment),
            status: match value.status {
                hikari_entity::assessment::session::AssessmentStatus::NotStarted => AssessmentStatus::NotStarted,
                hikari_entity::assessment::session::AssessmentStatus::Running => AssessmentStatus::Running,
                hikari_entity::assessment::session::AssessmentStatus::Finished => AssessmentStatus::Finished,
            },
            completed: value.completed,
        }
    }
}

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(pre_post_assessment))
        .route("/start", post(start_module_assessment))
        .route("/submit", post(submit_module_assessment))
        .with_state(())
}

#[utoipa::path(
    post,
    path = "/api/v0/modules/{module}/assessments/{pre_post}/start",
    responses(
        (status = OK, body = StartModuleAssessmentResponse, description = "Starts a new assessment, or returns the already running session for this (pre/post) assessment if one exists"),
    ),
    params(
        ("module" = String, Path, description = "module id from which the assessment should be started"),
        ("pre_post" = String, Path, description = "either pre or post to select which assessment should be started"),
    ),
    tag = "v0/modules",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]
pub(crate) async fn start_module_assessment(
    ExtractUser(user): ExtractUser,
    Extension(conn): Extension<DatabaseConnection>,
    Extension(app_config): Extension<AppConfig>,
    Path((module_id, pre_post)): Path<(String, PrePost)>,
) -> Result<impl IntoResponse, ModuleError> {
    let (_, assessment) = get_module_assessment(app_config.module_config(), &module_id, &user.groups)?;
    let assessment_id = pre_post.get_assessment_id(assessment);
    app_config
        .assessments()
        .get(assessment_id)
        .ok_or(Error::AssessmentConfigNotFound)?;

    let session = get_or_start_session(&conn, user.id, assessment_id).await?;

    let res = StartModuleAssessmentResponse {
        session: SessionResponse { session_id: session.id },
        assessment_id: assessment_id.to_owned(),
    };
    Ok(Json(res))
}

#[utoipa::path(
    post,
    request_body = [AnswerRequest],
    path = "/api/v0/modules/{module}/assessments/{pre_post}/submit",
    responses(
        (status = OK, description = "Persists the answers and marks this session as finished"),
    ),
    params(
        ("module" = String, Path, description = "module id from which the assessment should be started"),
        ("pre_post" = String, Path, description = "either pre or post to select which assessment should be started"),
    ),
    tag = "v0/modules",
    security(
            ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]

pub(crate) async fn submit_module_assessment(
    ExtractUser(user): ExtractUser,
    Extension(conn): Extension<DatabaseConnection>,
    Extension(app_config): Extension<AppConfig>,
    Path((module_id, pre_post)): Path<(String, PrePost)>,
    Json(answers): Json<Vec<AnswerRequest>>,
) -> Result<impl IntoResponse, ModuleError> {
    tracing::debug!(
        user_id = %user.id,
        module = module_id,
        assessment = Into::<&str>::into(pre_post),
        "got module assessment submission"
    );

    let (_, assessment) = get_module_assessment(app_config.module_config(), &module_id, &user.groups)?;
    let assessment_id = pre_post.get_assessment_id(assessment);

    let session = require_running_session(&conn, &user.id, assessment_id).await?;
    submit_session(&conn, user.id, session, app_config.assessments(), answers).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    get,
    path = "/api/v0/modules/{module}/assessments/{pre_post}",
    responses(
        (status = OK, body = ModuleAssessmentResponse, description = "Returns basic information about the selected"),
    ),
    params(
        ("module" = String, Path, description = "module id from which the assessment should be shown"),
        ("pre_post" = String, Path, description = "either pre or post to select which assessment should be shown"),
    ),
    tag = "v0/modules",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]

pub(crate) async fn pre_post_assessment(
    ExtractUser(user): ExtractUser,
    Extension(conn): Extension<DatabaseConnection>,
    Extension(app_config): Extension<AppConfig>,
    Path((module_id, pre_post)): Path<(String, PrePost)>,
) -> Result<impl IntoResponse, ModuleError> {
    let (_, assessment) = get_module_assessment(app_config.module_config(), &module_id, &user.groups)?;
    let assessment_id = pre_post.get_assessment_id(assessment);

    // Mirrors the `last_pre`/`last_post` shown on the module itself: first completed session for pre,
    // latest completed session (after module completion) for post.
    let session = match pre_post {
        PrePost::Pre => {
            hikari_db::assessment::session::Query::load_first_or_running_session(&conn, assessment_id, &user.id).await?
        }
        PrePost::Post => {
            let completion = hikari_db::module::status::Query::get_for_user(&conn, user.id, &module_id)
                .await?
                .and_then(|status| status.completion);

            if completion.is_none() {
                return Ok(Json(ModuleAssessmentResponse::default()));
            }

            hikari_db::assessment::session::Query::load_last_or_running_session(
                &conn,
                assessment_id,
                completion,
                &user.id,
            )
            .await?
        }
    };

    let res = session.map_or_else(ModuleAssessmentResponse::default, ModuleAssessmentResponse::from);

    Ok(Json(res))
}

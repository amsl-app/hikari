use crate::permissions::Permission;
use crate::routes::api::v0::modules::error::ModuleError;
use crate::user::ExtractUserId;
use crate::{AppConfig, user::ExtractUser};
use axum::Json;
use axum::extract::Query;
use axum::response::Response;
use axum::{
    Extension, Router,
    extract::Path,
    response::IntoResponse,
    routing::{get, post},
};
use hikari_db::quiz::quiz::Mutation;
use hikari_model::quiz::question::Question;
use hikari_model::quiz::quiz::Quiz;
use hikari_model::quiz::score::Score;
use hikari_model_tools::convert::IntoModel;
use protect_axum::protect;
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde::Deserialize;
use tokio::try_join;
use utoipa::ToSchema;

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(get_quizzes))
        .route("/start", post(start_quiz))
        .route("/scores", get(get_scores))
}

#[derive(Deserialize, ToSchema)]
struct StartQuizRequest {
    session_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct QuizFlags {
    pub deep: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v0/modules/{module_id}/quizzes",
    params(
        ("deep" = Option<String>, Query, description = "if set all quizzes are listed with their questions"),
    ),
    responses(
        (status = OK, body = Vec<Quiz>, description = "List of quizzes available to the user"),
    ),
    tag = "v0/modules",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
async fn get_quizzes(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(module_id): Path<String>,
    Query(deep): Query<QuizFlags>,
) -> Result<Response, ModuleError> {
    let deep = deep.deep.is_some();
    let quizzes = hikari_db::quiz::quiz::Query::get_quizzes_by_module(&conn, &user_id, &module_id).await?;
    let mut quiz_list = Vec::new();
    for quiz in quizzes {
        let (quiz_sessions, quiz_questions) = try_join!(
            hikari_db::quiz::quiz_sessions::Query::get_quiz_sessions(&conn, &quiz.id),
            hikari_db::quiz::question::Query::get_questions_by_quiz(&conn, &quiz.id),
        )?;

        let quiz_model: Quiz = quiz.into_model();

        let quiz_question_models: Vec<Question> = quiz_questions
            .into_iter()
            .map(|m| {
                let mut q: Question = m.into_model();
                q.sanitize_for_client();
                q
            })
            .collect();

        let quiz_question_models: Vec<&Question> = quiz_question_models.iter().collect();

        let quiz_session_models: Vec<&str> = quiz_sessions.iter().map(std::string::String::as_str).collect();

        let full_quiz = quiz_model.as_quiz_full(deep, quiz_question_models, quiz_session_models);

        let serialized_quiz = serde_json::to_value(&full_quiz)?; // We already serialize to avoid lifetime issues
        quiz_list.push(serialized_quiz);
    }

    Ok(Json(quiz_list).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/modules/{module_id}/quizzes/start",
    request_body = StartQuizRequest,
    responses(
        (status = OK, body = Quiz, description = "Started a new quiz for the user"),
    ),
    tag = "v0/modules",
    request_body(content = StartQuizRequest, content_type = "application/json"),
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
async fn start_quiz(
    ExtractUser(user): ExtractUser,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
    Path(module_id): Path<String>,
    Json(payload): Json<StartQuizRequest>,
) -> Result<Response, ModuleError> {
    let user_groups = &user.groups;

    let module = app_config
        .module_config()
        .get_for_group(&module_id, user_groups)
        .ok_or(crate::data::modules::error::ModuleError::ModuleNotFound)
        .map_err(ModuleError::from)?;

    if payload.session_ids.is_empty() {
        return Err(ModuleError::from(
            crate::data::modules::error::ModuleError::SessionNotFound,
        ));
    }

    for session in &payload.session_ids {
        if module.sessions.get(session).is_none() {
            return Err(ModuleError::from(
                crate::data::modules::error::ModuleError::SessionNotFound,
            ));
        }
    }

    let txn = conn.begin().await?;

    Mutation::close_quizzes_for_module(&txn, &user.id, &module_id).await?;
    let quiz = Mutation::create_quiz(&txn, &user.id, &module_id, payload.session_ids.clone()).await?;

    txn.commit().await?;

    let quiz_model: Quiz = quiz.into_model();

    let full_quiz_model = quiz_model.as_quiz_full(
        false,
        vec![],
        payload.session_ids.iter().map(std::string::String::as_str).collect(),
    );

    Ok(Json(full_quiz_model).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v0/modules/{module_id}/quizzes/scores",
    responses(
        (status = OK, body = Vec<Score>, description = "List of scores of a session"),
    ),
    tag = "v0/modules",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
async fn get_scores(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(module_id): Path<String>,
) -> Result<Response, ModuleError> {
    let scores = hikari_db::quiz::score::Query::get_scores_by_module(&conn, &user_id, &module_id).await?;

    let score_model: Vec<Score> = scores
        .into_iter()
        .map(hikari_model_tools::convert::IntoModel::into_model)
        .collect();

    Ok(Json(score_model).into_response())
}

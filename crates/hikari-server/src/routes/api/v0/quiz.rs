use crate::AppConfig;
use crate::permissions::Permission;
use crate::routes::api::v0::quiz::error::QuizError;
use crate::user::ExtractUserId;
use axum::Json;
use axum::extract::Query;
use axum::{
    Extension, Router,
    extract::Path,
    response::IntoResponse,
    response::Response,
    routing::{get, post},
};
use futures::TryFutureExt;
use hikari_config::module::content::{Content, ContentExam};
use hikari_config::module::session::Session;
use hikari_core::llm_config::LlmConfig;
use hikari_core::quiz::evaluation::evaluate_answer;
use hikari_core::quiz::question::create_question;
use hikari_model::quiz::question::{Question, QuestionFeedback};
use hikari_model::quiz::quiz::{Quiz, QuizFull};
use hikari_model::quiz::score::Score;
use hikari_model_tools::convert::{IntoDbModel, IntoModel};
use protect_axum::protect;
use rand::rng;
use rand::seq::IndexedRandom;
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use tokio::try_join;
use utoipa::ToSchema;
use uuid::Uuid;

mod error;

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(get_quizzes))
        .route("/scores", get(get_scores))
        .nest(
            "/{quiz_id}",
            Router::new().route("/", get(get_quiz)).nest(
                "/questions",
                Router::new()
                    .route("/", get(get_questions))
                    .route("/next", get(get_next_question))
                    .nest(
                        "/{question_id}",
                        Router::new()
                            .route("/", get(get_question))
                            .route("/feedback", post(add_feedback))
                            .route("/skip", post(skip_question))
                            .route("/answer", post(submit_answer)),
                    ),
            ),
        )
        .with_state(())
}

#[utoipa::path(
    get,
    path = "/api/v0/quizzes/scores",
    responses(
        (status = OK, body = Vec<Score>, description = "List of scores of a session"),
    ),
    tag = "v0/quizzes",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
async fn get_scores(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<Response, QuizError> {
    let scores = hikari_db::quiz::score::Query::get_scores(&conn, &user_id).await?;

    let score_model: Vec<Score> = scores
        .into_iter()
        .map(hikari_model_tools::convert::IntoModel::into_model)
        .collect();

    Ok(Json(score_model).into_response())
}

#[derive(Debug, Deserialize)]
pub(crate) struct QuizFlags {
    pub deep: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v0/quizzes",
        params(
        ("deep" = Option<String>, Query, description = "if set all quizzes are listed with their questions"),
    ),
    responses(
        (status = OK, body = Vec<QuizFull>, description = "List of quizzes available to the user"),
    ),
    tag = "v0/quizzes",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
async fn get_quizzes(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Query(deep): Query<QuizFlags>,
) -> Result<Response, QuizError> {
    let deep = deep.deep.is_some();

    tracing::info!("Fetching quizzes for user {}", user_id);
    let quizzes = hikari_db::quiz::quiz::Query::get_quizzes(&conn, &user_id).await?;
    tracing::info!("Found {} quizzes for user {}", quizzes.len(), user_id);
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
    get,
    path = "/api/v0/quizzes/{quiz_id}",
     responses(
        (status = OK, body = QuizFull, description = "Detailed quiz information including questions and sessions"),
    ),
    tag = "v0/quizzes",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
async fn get_quiz(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(quiz_id): Path<Uuid>,
) -> Result<Response, QuizError> {
    let (quiz, quiz_sessions, quiz_questions) = try_join!(
        get_quiz_by_id(&conn, &user_id, &quiz_id),
        hikari_db::quiz::quiz_sessions::Query::get_quiz_sessions(&conn, &quiz_id).map_err(QuizError::from),
        hikari_db::quiz::question::Query::get_questions_by_quiz(&conn, &quiz_id).map_err(QuizError::from),
    )?;

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

    let full_quiz = quiz.as_quiz_full(true, quiz_question_models, quiz_session_models);

    Ok(Json(full_quiz).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v0/quizzes/{quiz_id}/questions",
     responses(
        (status = OK, body = Vec<Question>, description = "List of questions for a quiz"),
    ),
    tag = "v0/quizzes",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
async fn get_questions(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(quiz_id): Path<Uuid>,
) -> Result<Response, QuizError> {
    let (_, quiz_questions) = try_join!(
        // To ensure the user has access to the quiz, we first fetch the quiz
        get_quiz_by_id(&conn, &user_id, &quiz_id),
        hikari_db::quiz::question::Query::get_questions_by_quiz(&conn, &quiz_id).map_err(QuizError::from),
    )?;

    let quiz_question_models: Vec<Question> = quiz_questions
        .into_iter()
        .map(|m| {
            let mut q: Question = m.into_model();
            q.sanitize_for_client();
            q
        })
        .collect();

    Ok(Json(quiz_question_models).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v0/quizzes/{quiz_id}/questions/{question_id}",
        responses(
            (status = OK, body = Question, description = "Detailed information about a specific question"),
        ),
    tag = "v0/quizzes",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
async fn get_question(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path((quiz_id, question_id)): Path<(Uuid, Uuid)>,
) -> Result<Response, QuizError> {
    let (_, quiz_question) = try_join!(
        // To ensure the user has access to the quiz, we first fetch the quiz
        get_quiz_by_id(&conn, &user_id, &quiz_id),
        get_question_by_id(&conn, &question_id)
    )?;

    if quiz_question.quiz_id != quiz_id {
        return Err(QuizError::QuestionNotFound);
    }

    Ok(Json(quiz_question).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v0/quizzes/{quiz_id}/questions/next",
    responses(
        (status = OK, body = Question, description = "Next question for the quiz"),
    ),
    tag = "v0/quizzes",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
async fn get_next_question(
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
    ExtractUserId(user_id): ExtractUserId,
    Path(quiz_id): Path<Uuid>,
) -> Result<Response, QuizError> {
    let (quiz, open_question, session_ids) = try_join!(
        get_quiz_by_id(&conn, &user_id, &quiz_id),
        hikari_db::quiz::question::Query::get_open_question(&conn, &quiz_id).map_err(QuizError::from),
        hikari_db::quiz::quiz_sessions::Query::get_quiz_sessions(&conn, &quiz_id).map_err(QuizError::from),
    )?;

    if let Some(question) = open_question {
        let mut question_model: Question = question.into_model();
        question_model.sanitize_for_client();
        return Ok(Json(question_model).into_response());
    }

    let module_id = quiz.module_id;

    let selected_session_id = pick_random_session(&session_ids).ok_or(QuizError::NoSessionIds)?;

    let session = app_config
        .module_config()
        .get(&module_id)
        .ok_or(QuizError::ModuleNotFound(module_id.clone()))?
        .sessions
        .get(&selected_session_id)
        .ok_or(QuizError::SessionNotFound(selected_session_id.clone()))?;

    let content: Content = pick_random_content(session).ok_or(QuizError::NoContentProvided)?;

    let topic = &content.title;

    let specific_content: &str = pick_specific_content(&content);

    let llm_config: &LlmConfig = AppConfig::llm_config(&app_config);

    let contents = &session.contents;

    let exams = contents
        .iter()
        .flat_map(|c| c.exams.iter().map(|e| (c.title.clone(), e.clone())))
        .collect::<Vec<(String, ContentExam)>>();

    let sources: Vec<String> = contents
        .iter()
        .flat_map(|c| c.sources.primary().iter().map(|s| s.file_id.clone()))
        .collect();

    let mut question = create_question(
        &user_id,
        &selected_session_id,
        specific_content,
        topic,
        &exams,
        llm_config,
        &conn,
        &quiz_id,
        &sources,
    )
    .await?;

    question.sanitize_for_client();

    Ok(Json(question).into_response())
}

#[derive(Deserialize, ToSchema)]
struct EvaluationRequest {
    answer: String,
}

#[utoipa::path(
    post,
    path = "/api/v0/quizzes/{quiz_id}/questions/{question_id}/answer",
    responses(
        (status = OK, body = Question, description = "Evaluated question with answer and evaluation"),
    ),
    request_body = EvaluationRequest,
    tag = "v0/quizzes",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
async fn submit_answer(
    ExtractUserId(user_id): ExtractUserId,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
    Path((quiz_id, question_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<EvaluationRequest>,
) -> Result<impl IntoResponse, QuizError> {
    let (quiz, quiz_question) = try_join!(
        // To ensure the user has access to the quiz, we first fetch the quiz
        get_quiz_by_id(&conn, &user_id, &quiz_id),
        get_question_by_id(&conn, &question_id)
    )?;

    let module_id = quiz.module_id;
    let session_id = quiz_question.session_id.as_str();

    let session = app_config
        .module_config()
        .get(&module_id)
        .ok_or(QuizError::ModuleNotFound(module_id.clone()))?
        .sessions
        .get(session_id)
        .ok_or(QuizError::SessionNotFound(session_id.to_string()))?;

    let llm_config: &LlmConfig = AppConfig::llm_config(&app_config);

    let session_sources: Vec<String> = session
        .contents
        .iter()
        .flat_map(|c| c.sources.primary().iter().map(|s| s.file_id.clone()))
        .collect();

    let contents = &session.contents;

    let exams = contents
        .iter()
        .flat_map(|c| c.exams.iter().map(|e| (c.title.clone(), e.clone())))
        .collect::<Vec<(String, ContentExam)>>();

    let evaluated_question = evaluate_answer(
        &user_id,
        &module_id,
        &quiz_question,
        &exams,
        &payload.answer,
        llm_config,
        &conn,
        session_sources,
    )
    .await?;

    tracing::info!("Evaluated question: {:?}", evaluated_question.evaluation);

    Ok(Json(evaluated_question).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/quizzes/{quiz_id}/questions/{question_id}/skip",
    responses(
        (status = OK, description = "Question skipped successfully"),
    ),
    tag = "v0/quizzes",
    security(
        ("token" = [])
    )
)]
async fn skip_question(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path((quiz_id, question_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, QuizError> {
    let (_, quiz_question) = try_join!(
        // To ensure the user has access to the quiz, we first fetch the quiz
        get_quiz_by_id(&conn, &user_id, &quiz_id),
        get_question_by_id(&conn, &question_id)
    )?;

    if quiz_question.quiz_id != quiz_id {
        return Err(QuizError::QuestionNotFound);
    }

    hikari_db::quiz::question::Mutation::skip_question(&conn, &quiz_question.id).await?;

    Ok(())
}

#[derive(serde_derive::Deserialize, utoipa::ToSchema)]
struct FeedbackPayload {
    feedback: QuestionFeedback,
    feedback_explanation: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v0/quizzes/{quiz_id}/questions/{question_id}/feedback",
    request_body = FeedbackPayload,
    responses(
        (status = OK, description = "Feedback added successfully"),
    ),
    tag = "v0/quizzes",
    security(
        ("token" = [])
    )
)]
async fn add_feedback(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path((quiz_id, question_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<FeedbackPayload>,
) -> Result<impl IntoResponse, QuizError> {
    let (_, quiz_question) = try_join!(
        // To ensure the user has access to the quiz, we first fetch the quiz
        get_quiz_by_id(&conn, &user_id, &quiz_id),
        get_question_by_id(&conn, &question_id)
    )?;

    if quiz_question.quiz_id != quiz_id {
        return Err(QuizError::QuestionNotFound);
    }

    let feedback = payload.feedback.into_db_model();
    let feedback_explanation = payload.feedback_explanation.clone();

    hikari_db::quiz::question::Mutation::add_feedback(
        &conn,
        &quiz_question.id,
        &feedback,
        feedback_explanation.as_deref(),
    )
    .await?;

    Ok(())
}

async fn get_quiz_by_id(conn: &DatabaseConnection, user_id: &Uuid, quiz_id: &Uuid) -> Result<Quiz, QuizError> {
    let result = hikari_db::quiz::quiz::Query::get_quiz_by_id(conn, user_id, quiz_id)
        .await?
        .ok_or(QuizError::QuizNotFound)?
        .into_model();
    Ok(result)
}

async fn get_question_by_id(conn: &DatabaseConnection, question_id: &Uuid) -> Result<Question, QuizError> {
    let mut result: Question = hikari_db::quiz::question::Query::get_question_by_id(conn, question_id)
        .await?
        .ok_or(QuizError::QuestionNotFound)?
        .into_model();
    result.sanitize_for_client();
    Ok(result)
}

fn pick_random_session(sessions: &[String]) -> Option<String> {
    let mut rng = rng();
    let session = sessions.choose(&mut rng).cloned();
    drop(rng);
    session
}

fn pick_random_content(session: &Session) -> Option<Content> {
    let mut rng = rng();
    let content = session.contents.choose(&mut rng).cloned();
    drop(rng);
    content
}

fn pick_specific_content(content: &Content) -> &str {
    let mut rng = rng();
    let content = content
        .contents
        .choose(&mut rng)
        .expect("Der Vektor 'contents' darf nicht leer sein!")
        .as_str();
    drop(rng);
    content
}

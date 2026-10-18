use crate::AppConfig;
use crate::permissions::Permission;
use crate::routes::api::v0::quiz::error::QuizError;
use crate::user::ExtractUserId;
use axum::Json;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::{
    Extension, Router,
    extract::Path,
    response::IntoResponse,
    response::Response,
    routing::{get, post},
};
use chrono::NaiveDate;
use futures::TryFutureExt;
use hikari_config::module::content::{Content, ContentExam, QuestionBloomLevel};
use hikari_config::module::session::Session;
use hikari_core::llm_config::LlmConfig;
use hikari_core::quiz::evaluation::evaluate_answer;
use hikari_core::quiz::question::create_question;
use hikari_model::quiz::goal::Goal;
use hikari_model::quiz::goal_feedback::FeedbackInput;
use hikari_model::quiz::goal_feedback::GoalFeedback;
use hikari_model::quiz::question::{Question, QuestionOption, QuestionType};
use hikari_model::quiz::quiz::{Quiz, QuizFull};
use hikari_model::quiz::quiz_question_attempt::{QuestionFeedback, QuizQuestionAttempt};
use hikari_model::quiz::score::Score;
use hikari_model_tools::convert::{IntoDbModel, IntoModel};
use protect_axum::protect;
use rand::RngExt;
use rand::rng;
use rand::seq::IndexedRandom;
use reqwest::Client;
use reqwest_middleware::reqwest;
use sea_orm::DatabaseConnection;
use sea_orm::TransactionTrait;
use serde::Deserialize;
use std::collections::HashMap;
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
        .route("/{module_id}/all", get(get_all_questions_by_module))
        .route("/{module_id}/goal/new", post(create_goal))
        .route("/{module_id}/goal/all", get(get_all_goals))
        .route("/goal/{goal_id}/deactivate", post(deactivate_goal))
        .route("/goal/feedback", post(generate_goal_feedback))
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
                            .route("/{attempt}/feedback", post(add_feedback))
                            .route("/{attempt}/skip", post(skip_question))
                            .route("/{attempt}/answer", post(submit_answer))
                            .route("/{attempt}/answer/again", post(submit_answer_again)),
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
                let q: Question = m.into_model();
                //q.sanitize_for_client();
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
            let q: Question = m.into_model();
            //q.sanitize_for_client();
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
            let q: Question = m.into_model();
            //q.sanitize_for_client();
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

    //if quiz_question.quiz_id != quiz_id {
    //return Err(QuizError::QuestionNotFound);
    //}

    Ok(Json(quiz_question).into_response())
}

use serde::Serialize;

#[derive(Serialize, ToSchema)]
struct QuestionResponse {
    pub id: Uuid,
    pub quiz_id: Uuid,
    pub topic: String,
    pub content: String,
    pub question: String,
    pub session_id: String,
    pub level: QuestionBloomLevel,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluation: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub grade: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_solution: Option<String>,

    pub r#type: QuestionType,
    pub options: Vec<QuestionOption>,

    pub attempt: i32,

    pub asked_at: Option<chrono::NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answered_at: Option<chrono::NaiveDateTime>,
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
        tracing::debug!(
            ?quiz_id,
            question_id = ?question.id,
            "found open question for quiz",
        );

        let attempt_model =
            hikari_db::quiz::quiz_question_attempt::query::Query::get_open_attempt(&conn, &quiz_id, &question.id)
                .await?
                .ok_or(QuizError::QuestionNotFound)?;

        let attempt_model: QuizQuestionAttempt = attempt_model.into_model();
        let question_model: Question = question.into_model();

        let response = QuestionResponse {
            id: question_model.id,
            quiz_id,
            topic: question_model.topic,
            content: question_model.content,
            question: question_model.question,
            session_id: attempt_model.session_id,
            level: question_model.level,

            answer: attempt_model.answer,
            evaluation: attempt_model.evaluation,
            grade: attempt_model.grade,

            ai_solution: question_model.ai_solution,

            r#type: question_model.r#type,
            options: question_model.options,

            attempt: attempt_model.attempt,
            answered_at: attempt_model.answered_at,
            asked_at: attempt_model.asked_at,
        };

        return Ok(Json(response).into_response());
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

    let number_questions_answered =
        hikari_db::quiz::quiz_question_attempt::query::Query::count_attempts_by_user_module(
            &conn, &user_id, &module_id,
        )
        .await?;

    if number_questions_answered <= 20 {
        let content: Content = pick_random_content(session).ok_or(QuizError::NoContentProvided)?;

        let topic = &content.title;

        let specific_content: &str = pick_specific_content(&content);

        let llm_config: &LlmConfig = app_config.llm_config();

        let contents = &session.contents;

        let exams = contents
            .iter()
            .flat_map(|c| c.exams.iter().map(|e| (c.title.clone(), e.clone())))
            .collect::<Vec<(String, ContentExam)>>();

        let sources: Vec<String> = contents
            .iter()
            .flat_map(|c| c.sources.primary().iter().map(|s| s.file_id.clone()))
            .collect();

        tracing::debug!(selected_session_id, topic, "requesting new question");

        let question = create_question(
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

        //question.sanitize_for_client();
        let attempt = 1_i32;

        hikari_db::quiz::quiz_question_attempt::mutation::Mutation::create_attempt(
            &conn,
            &quiz_id,
            &question.id,
            attempt,
            &selected_session_id,
        )
        .await?;

        let attempt_model: QuizQuestionAttempt =
            hikari_db::quiz::quiz_question_attempt::query::Query::get_attempt(&conn, &quiz_id, &question.id, &attempt)
                .await?
                .ok_or(QuizError::QuestionNotFound)?
                .into_model();
        let response = QuestionResponse {
            id: question.id,
            quiz_id,
            topic: question.topic,
            content: question.content,
            question: question.question,
            session_id: attempt_model.session_id,
            level: question.level,

            answer: attempt_model.answer,
            evaluation: attempt_model.evaluation,
            grade: attempt_model.grade,

            ai_solution: question.ai_solution,

            r#type: question.r#type,
            options: question.options,

            attempt: attempt_model.attempt,
            answered_at: attempt_model.answered_at,
            asked_at: attempt_model.asked_at,
        };

        Ok(Json(response).into_response())
    } else {
        let saved_recommendations =
            hikari_db::quiz::question_recommendation::query::Query::get_unused_recommendations_by_quiz(&conn, &quiz_id)
                .await?;

        if saved_recommendations.len() <= 1 {
            let conn = conn.clone();
            let quiz_id = quiz_id;
            let user_id = user_id;
            let module_id = module_id.clone();
            let all_sessions: Vec<Session> = app_config
                .module_config()
                .get(&module_id)
                .ok_or(QuizError::ModuleNotFound(module_id.clone()))?
                .sessions
                .values()
                .cloned()
                .collect();

            let sessions: Vec<Session> = all_sessions
                .into_iter()
                .filter(|session| session_ids.contains(&session.id))
                .collect();

            tracing::info!(
                "LEARNER_MODEL_URL: {:?}",
                std::env::var("LEARNER_MODEL_URL").expect("LEARNER_MODEL_URL must be set")
            );

            tokio::spawn(async move {
                let learner_model_client = RecommendationClient::new(
                    std::env::var("LEARNER_MODEL_URL").expect("LEARNER_MODEL_URL must be set"),
                );

                match learner_model_client
                    .get_recommendations(user_id, module_id, sessions, &conn)
                    .await
                {
                    Ok(recommendations) => {
                        tracing::info!("Learner Model recommendations: {:?}", recommendations);
                        let question_ids: Vec<Uuid> = recommendations
                            .recommendations
                            .iter()
                            .map(|recommendation| recommendation.question_id)
                            .collect();

                        if question_ids.is_empty() {
                            tracing::warn!(?quiz_id, "recommendation service returned no question IDs");
                            return;
                        }

                        if let Err(error) =
                            hikari_db::quiz::question_recommendation::mutation::Mutation
                            ::create_multiple_recommendations(
                                &conn,
                                &quiz_id,
                                &question_ids,
                            )
                                .await
                        {
                            tracing::error!(
                        ?error,
                        ?quiz_id,
                        "failed to save recommendations"
                    );
                        }
                    }

                    Err(error) => {
                        tracing::error!(?error, ?quiz_id, "recommendation request failed");
                    }
                }
            });
        }

        let unused_recommendations =
            hikari_db::quiz::question_recommendation::query::Query::get_unused_recommendations_by_quiz(&conn, &quiz_id)
                .await?;

        if unused_recommendations.is_empty() {
            let content: Content = pick_random_content(session).ok_or(QuizError::NoContentProvided)?;

            let topic = &content.title;

            let specific_content: &str = pick_specific_content(&content);

            let llm_config: &LlmConfig = app_config.llm_config();

            let contents = &session.contents;

            let exams = contents
                .iter()
                .flat_map(|c| c.exams.iter().map(|e| (c.title.clone(), e.clone())))
                .collect::<Vec<(String, ContentExam)>>();

            let sources: Vec<String> = contents
                .iter()
                .flat_map(|c| c.sources.primary().iter().map(|s| s.file_id.clone()))
                .collect();

            tracing::debug!(selected_session_id, topic, "requesting new question");

            let question = create_question(
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

            //question.sanitize_for_client();
            let attempt = 1_i32;

            hikari_db::quiz::quiz_question_attempt::mutation::Mutation::create_attempt(
                &conn,
                &quiz_id,
                &question.id,
                attempt,
                &selected_session_id,
            )
            .await?;

            let attempt_model: QuizQuestionAttempt = hikari_db::quiz::quiz_question_attempt::query::Query::get_attempt(
                &conn,
                &quiz_id,
                &question.id,
                &attempt,
            )
            .await?
            .ok_or(QuizError::QuestionNotFound)?
            .into_model();
            let response = QuestionResponse {
                id: question.id,
                quiz_id,
                topic: question.topic,
                content: question.content,
                question: question.question,
                session_id: attempt_model.session_id,
                level: question.level,

                answer: attempt_model.answer,
                evaluation: attempt_model.evaluation,
                grade: attempt_model.grade,

                ai_solution: question.ai_solution,

                r#type: question.r#type,
                options: question.options,

                attempt: attempt_model.attempt,
                answered_at: attempt_model.answered_at,
                asked_at: attempt_model.asked_at,
            };

            return Ok(Json(response).into_response());
        }

        let recommendation = unused_recommendations
            .into_iter()
            .next()
            .ok_or(QuizError::RecommendationNotReceived)?;

        hikari_db::quiz::question_recommendation::mutation::Mutation::mark_recommendation_as_used(
            &conn,
            &recommendation.id,
        )
        .await?;

        let recommended_question =
            hikari_db::quiz::question::Query::get_question_by_id(&conn, &recommendation.question_id)
                .await?
                .ok_or(QuizError::QuestionNotFound)?;

        let question_model: Question = recommended_question.into_model();

        if rng().random_bool(0.7) {
            let last_attempt =
                hikari_db::quiz::quiz_question_attempt::query::Query::count_question_attempts_by_user_module(
                    &conn,
                    &user_id,
                    &module_id,
                    &question_model.id,
                )
                .await?;
            let attempt = last_attempt as i32 + 1;

            hikari_db::quiz::quiz_question_attempt::mutation::Mutation::create_attempt(
                &conn,
                &quiz_id,
                &question_model.id,
                attempt,
                &selected_session_id,
            )
            .await?;

            let attempt_model: QuizQuestionAttempt = hikari_db::quiz::quiz_question_attempt::query::Query::get_attempt(
                &conn,
                &quiz_id,
                &question_model.id,
                &attempt,
            )
            .await?
            .ok_or(QuizError::QuestionNotFound)?
            .into_model();
            let response = QuestionResponse {
                id: question_model.id,
                quiz_id,
                topic: question_model.topic,
                content: question_model.content,
                question: question_model.question,
                session_id: attempt_model.session_id,
                level: question_model.level,

                answer: attempt_model.answer,
                evaluation: attempt_model.evaluation,
                grade: attempt_model.grade,

                ai_solution: question_model.ai_solution,

                r#type: question_model.r#type,
                options: question_model.options,

                attempt: attempt_model.attempt,
                answered_at: attempt_model.answered_at,
                asked_at: attempt_model.asked_at,
            };

            Ok(Json(response).into_response())
        } else {
            let specific_content: &str = &question_model.content;

            let topic = question_model.topic;

            let llm_config: &LlmConfig = app_config.llm_config();

            let contents = &session.contents;

            let exams = contents
                .iter()
                .flat_map(|c| c.exams.iter().map(|e| (c.title.clone(), e.clone())))
                .collect::<Vec<(String, ContentExam)>>();

            let sources: Vec<String> = contents
                .iter()
                .flat_map(|c| c.sources.primary().iter().map(|s| s.file_id.clone()))
                .collect();

            tracing::debug!(selected_session_id, topic, "requesting new question");

            let question = create_question(
                &user_id,
                &selected_session_id,
                specific_content,
                &topic,
                &exams,
                llm_config,
                &conn,
                &quiz_id,
                &sources,
            )
            .await?;

            //question.sanitize_for_client();
            let attempt = 1_i32;

            hikari_db::quiz::quiz_question_attempt::mutation::Mutation::create_attempt(
                &conn,
                &quiz_id,
                &question.id,
                attempt,
                &selected_session_id,
            )
            .await?;

            let attempt_model: QuizQuestionAttempt = hikari_db::quiz::quiz_question_attempt::query::Query::get_attempt(
                &conn,
                &quiz_id,
                &question.id,
                &attempt,
            )
            .await?
            .ok_or(QuizError::QuestionNotFound)?
            .into_model();

            let response = QuestionResponse {
                id: question.id,
                quiz_id,
                topic: question.topic,
                content: question.content,
                question: question.question,
                session_id: attempt_model.session_id,
                level: question.level,

                answer: attempt_model.answer,
                evaluation: attempt_model.evaluation,
                grade: attempt_model.grade,

                ai_solution: question.ai_solution,

                r#type: question.r#type,
                options: question.options,

                attempt: attempt_model.attempt,
                answered_at: attempt_model.answered_at,
                asked_at: attempt_model.asked_at,
            };

            Ok(Json(response).into_response())
        }
    }
}

#[derive(Deserialize, ToSchema)]
struct EvaluationRequest {
    answer: String,
}

#[utoipa::path(
    post,
    path = "/api/v0/quizzes/{quiz_id}/questions/{question_id}/{attempt}/answer",
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
    Path((quiz_id, question_id, attempt)): Path<(Uuid, Uuid, i32)>,
    Json(payload): Json<EvaluationRequest>,
) -> Result<impl IntoResponse, QuizError> {
    let (quiz, quiz_question) = try_join!(
        // To ensure the user has access to the quiz, we first fetch the quiz
        get_quiz_by_id(&conn, &user_id, &quiz_id),
        get_question_by_id(&conn, &question_id)
    )?;
    let session_id = hikari_db::quiz::quiz_question_attempt::query::Query::get_session_id_by_attempt(
        &conn,
        &quiz_id,
        &question_id,
        &attempt,
    )
    .await?
    .ok_or(QuizError::SessionNotFound("ID not found".to_owned()))?;

    let module_id = quiz.module_id;
    //let session_id = quiz_question.session_id.as_str();

    let session = app_config
        .module_config()
        .get(&module_id)
        .ok_or(QuizError::ModuleNotFound(module_id.clone()))?
        .sessions
        .get(&session_id)
        .ok_or(QuizError::SessionNotFound("ID not found".to_owned()))?;

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
        &quiz_id,
        &attempt,
        &session_id,
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
    let question_model: Question = hikari_db::quiz::question::Query::get_question_by_id(&conn, &question_id)
        .await?
        .ok_or(QuizError::QuestionNotFound)?
        .into_model();

    let response = QuestionResponse {
        id: question_model.id,
        quiz_id,
        topic: question_model.topic,
        content: question_model.content,
        question: question_model.question,
        session_id: evaluated_question.session_id,
        level: question_model.level,

        answer: evaluated_question.answer,
        evaluation: evaluated_question.evaluation,
        grade: evaluated_question.grade,

        ai_solution: question_model.ai_solution,

        r#type: question_model.r#type,
        options: question_model.options,

        attempt: evaluated_question.attempt,
        answered_at: evaluated_question.answered_at,
        asked_at: evaluated_question.asked_at,
    };
    Ok(Json(response).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/quizzes/{quiz_id}/questions/{question_id}/{attempt}/answer/again",
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
async fn submit_answer_again(
    ExtractUserId(user_id): ExtractUserId,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
    Path((quiz_id, question_id, attempt)): Path<(Uuid, Uuid, i32)>,
    Json(payload): Json<EvaluationRequest>,
) -> Result<impl IntoResponse, QuizError> {
    let (quiz, quiz_question) = try_join!(
        // To ensure the user has access to the quiz, we first fetch the quiz
        get_quiz_by_id(&conn, &user_id, &quiz_id),
        get_question_by_id(&conn, &question_id)
    )?;
    let session_id = hikari_db::quiz::quiz_question_attempt::query::Query::get_session_id_by_attempt(
        &conn,
        &quiz_id,
        &question_id,
        &attempt,
    )
    .await?
    .ok_or(QuizError::SessionNotFound("ID not found".to_owned()))?;

    let module_id = quiz.module_id;
    //let session_id = quiz_question.session_id.as_str();

    let session = app_config
        .module_config()
        .get(&module_id)
        .ok_or(QuizError::ModuleNotFound(module_id.clone()))?
        .sessions
        .get(&session_id)
        .ok_or(QuizError::SessionNotFound("ID not found".to_owned()))?;

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

    let last_attempt = hikari_db::quiz::quiz_question_attempt::query::Query::count_question_attempts_by_user_module(
        &conn,
        &user_id,
        &module_id,
        &question_id,
    )
    .await?;
    let new_attempt = last_attempt as i32 + 1;

    hikari_db::quiz::quiz_question_attempt::mutation::Mutation::create_attempt(
        &conn,
        &quiz_id,
        &question_id,
        new_attempt,
        &session_id,
    )
    .await?;

    let evaluated_question = evaluate_answer(
        &user_id,
        &quiz_id,
        &new_attempt,
        &session_id,
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
    let question_model: Question = hikari_db::quiz::question::Query::get_question_by_id(&conn, &question_id)
        .await?
        .ok_or(QuizError::QuestionNotFound)?
        .into_model();

    let response = QuestionResponse {
        id: question_model.id,
        quiz_id,
        topic: question_model.topic,
        content: question_model.content,
        question: question_model.question,
        session_id: evaluated_question.session_id,
        level: question_model.level,

        answer: evaluated_question.answer,
        evaluation: evaluated_question.evaluation,
        grade: evaluated_question.grade,

        ai_solution: question_model.ai_solution,

        r#type: question_model.r#type,
        options: question_model.options,

        attempt: evaluated_question.attempt,
        answered_at: evaluated_question.answered_at,
        asked_at: evaluated_question.asked_at,
    };
    Ok(Json(response).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/quizzes/{quiz_id}/questions/{question_id}/{attempt}/skip",
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
    Path((quiz_id, question_id, attempt)): Path<(Uuid, Uuid, i32)>,
) -> Result<impl IntoResponse, QuizError> {
    let (_, quiz_question) = try_join!(
        // To ensure the user has access to the quiz, we first fetch the quiz
        get_quiz_by_id(&conn, &user_id, &quiz_id),
        get_question_by_id(&conn, &question_id)
    )?;

    //if quiz_question.quiz_id != quiz_id {
    //return Err(QuizError::QuestionNotFound);
    //}

    hikari_db::quiz::quiz_question_attempt::mutation::Mutation::skip_question(
        &conn,
        &quiz_id,
        &quiz_question.id,
        &attempt,
    )
    .await?;

    Ok(())
}

#[derive(serde_derive::Deserialize, utoipa::ToSchema)]
struct FeedbackPayload {
    feedback: QuestionFeedback,
    feedback_explanation: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v0/quizzes/{quiz_id}/questions/{question_id}/{attempt}/feedback",
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
    Path((quiz_id, question_id, attempt)): Path<(Uuid, Uuid, i32)>,
    Json(payload): Json<FeedbackPayload>,
) -> Result<impl IntoResponse, QuizError> {
    let (_, quiz_question) = try_join!(
        // To ensure the user has access to the quiz, we first fetch the quiz
        get_quiz_by_id(&conn, &user_id, &quiz_id),
        get_question_by_id(&conn, &question_id)
    )?;

    //if quiz_question.quiz_id != quiz_id {
    //return Err(QuizError::QuestionNotFound);
    //}

    let feedback = payload.feedback.into_db_model();
    let feedback_explanation = payload.feedback_explanation.clone();

    hikari_db::quiz::quiz_question_attempt::mutation::Mutation::add_feedback(
        &conn,
        &quiz_id,
        &quiz_question.id,
        &attempt,
        &feedback,
        feedback_explanation.as_deref(),
    )
    .await?;

    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/v0/quizzes/{module_id}/all",
    params(
        ("module_id" = String, Path, description = "ID of the module")
    ),
    responses(
    (
        status = OK,
        body = [QuestionResponse],
        description = "All questions with their attempts for the user's module"
    ),
    (
        status = UNAUTHORIZED,
        description = "Authentication required"
    ),
    (
        status = INTERNAL_SERVER_ERROR,
        description = "Failed to load questions or attempts"
    )
    ),
    tag = "v0/quizzes",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
async fn get_all_questions_by_module(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(module_id): Path<String>,
) -> Result<Json<Vec<QuestionResponse>>, QuizError> {
    let quizzes = hikari_db::quiz::quiz::query::Query::get_quizzes_by_module(&conn, &user_id, &module_id).await?;

    let mut questions = Vec::new();

    for quiz in quizzes {
        let attempts =
            hikari_db::quiz::quiz_question_attempt::query::Query::get_attempts_by_quiz(&conn, &quiz.id).await?;

        for attempt in attempts {
            let question = hikari_db::quiz::question::query::Query::get_question_by_id(&conn, &attempt.question_id)
                .await?
                .ok_or(QuizError::QuestionNotFound)?;
            let attempt_model: QuizQuestionAttempt = attempt.into_model();
            let question_model: Question = question.into_model();

            questions.push(QuestionResponse {
                id: question_model.id,
                quiz_id: attempt_model.quiz_id,
                topic: question_model.topic,
                content: question_model.content,
                question: question_model.question,
                session_id: attempt_model.session_id,
                level: question_model.level,

                answer: attempt_model.answer,
                evaluation: attempt_model.evaluation,
                grade: attempt_model.grade,
                ai_solution: question_model.ai_solution,

                r#type: question_model.r#type,
                options: question_model.options,
                attempt: attempt_model.attempt,

                asked_at: attempt_model.asked_at,
                answered_at: attempt_model.answered_at,
            });
        }
    }

    Ok(Json(questions))
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateGoalWithQuizRequest {
    pub session_ids: Vec<String>,
    pub target_mode: String,
    pub level: String,
    pub deadline: String,
    pub outcome: String,
    pub additional_outcomes: Vec<String>,
    pub obstacles: Vec<String>,
    pub plans: Vec<String>,
    pub lecture_targets: hikari_model::quiz::goal::LectureTargets,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GoalResponse {
    pub id: Uuid,
    pub module_id: String,
    pub quiz_id: Uuid,
    pub session_ids: Vec<String>,
    pub target_mode: String,
    pub level: String,
    pub deadline: String,
    pub outcome: String,
    pub additional_outcomes: Vec<String>,
    pub obstacles: Vec<String>,
    pub plans: Vec<String>,
    pub active: bool,
    pub lecture_targets: hikari_model::quiz::goal::LectureTargets,
}

#[utoipa::path(
    post,
    path = "/api/v0/quizzes/{module_id}/goal/new",
    params(
        ("module_id" = String, Path, description = "ID of the module")
    ),
    request_body = CreateGoalWithQuizRequest,
    responses(
        (
            status = OK,
            body = Goal,
            description = "Created a new goal and its associated quiz"
        ),
        (
            status = UNAUTHORIZED,
            description = "Authentication required"
        ),
        (
            status = INTERNAL_SERVER_ERROR,
            description = "Failed to create goal or quiz"
        )
    ),
    tag = "v0/quizzes",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
async fn create_goal(
    ExtractUserId(user_id): ExtractUserId,
    Extension(_app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
    Path(module_id): Path<String>,
    Json(payload): Json<CreateGoalWithQuizRequest>,
) -> Result<Response, QuizError> {
    let goal_session_ids = payload.session_ids.clone();

    let deadline = NaiveDate::parse_from_str(&payload.deadline, "%Y-%m-%d")
        .map_err(|err| QuizError::InvalidDeadline(err.to_string()))?;

    let txn = conn.begin().await?;

    let quiz = hikari_db::quiz::quiz::mutation::Mutation::create_quiz_in_transaction(
        &txn,
        &user_id,
        &module_id,
        payload.session_ids,
    )
    .await?;

    let goal = hikari_db::quiz::goal::mutation::Mutation::create_goal_in_transaction(
        &txn,
        user_id,
        module_id,
        quiz.id,
        goal_session_ids,
        payload.target_mode,
        payload.level,
        deadline,
        payload.outcome,
        serde_json::to_value(payload.additional_outcomes)?,
        serde_json::to_value(payload.obstacles)?,
        serde_json::to_value(payload.plans)?,
        serde_json::to_value(payload.lecture_targets)?,
    )
    .await?;

    txn.commit().await?;

    let goal_model: Goal = goal.into_model();

    let goal_response = GoalResponse {
        id: goal_model.id,
        module_id: goal_model.module_id,
        quiz_id: goal_model.quiz_id,
        session_ids: goal_model.session_ids.clone(),
        target_mode: goal_model.target_mode,
        level: goal_model.level,
        deadline: goal_model.deadline.format("%Y-%m-%d").to_string(),
        outcome: goal_model.outcome,
        additional_outcomes: goal_model.additional_outcomes,
        obstacles: goal_model.obstacles,
        plans: goal_model.plans,
        active: goal_model.active,
        lecture_targets: goal_model.lecture_targets,
    };

    Ok(Json(goal_response).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v0/quizzes/{module_id}/goal/all",
    params(
        ("module_id" = String, Path, description = "ID of the module")
    ),
    responses(
        (
            status = OK,
            body = Vec<GoalResponse>,
            description = "Returns all goals for the authenticated user and module"
        ),
        (
            status = UNAUTHORIZED,
            description = "Authentication required"
        ),
        (
            status = INTERNAL_SERVER_ERROR,
            description = "Failed to load goals"
        )
    ),
    tag = "v0/quizzes",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
async fn get_all_goals(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(module_id): Path<String>,
) -> Result<Response, QuizError> {
    let goals = hikari_db::quiz::goal::query::Query::get_goals_by_user_and_module(&conn, &user_id, &module_id).await?;

    let responses: Vec<GoalResponse> = goals
        .into_iter()
        .map(|goal| GoalResponse {
            id: goal.id,
            module_id: goal.module_id,
            quiz_id: goal.quiz_id,
            session_ids: goal.session_ids,
            target_mode: goal.target_mode,
            level: goal.level,
            deadline: goal.deadline.format("%Y-%m-%d").to_string(),
            outcome: goal.outcome,
            additional_outcomes: serde_json::from_value(goal.additional_outcomes).unwrap_or_default(),
            obstacles: serde_json::from_value(goal.obstacles).unwrap_or_default(),
            plans: serde_json::from_value(goal.plans).unwrap_or_default(),
            active: goal.active,
            lecture_targets: serde_json::from_value(goal.lecture_targets).unwrap_or_default(),
        })
        .collect();

    Ok(Json(responses).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/quizzes/goal/{goal_id}/deactivate",
    params(
        ("goal_id" = Uuid, Path, description = "ID of the goal to deactivate")
    ),
    responses(
        (
            status = OK,
            body = GoalResponse,
            description = "Goal successfully deactivated"
        ),
        (
            status = UNAUTHORIZED,
            description = "Authentication required"
        ),
        (
            status = NOT_FOUND,
            description = "Goal not found"
        ),
        (
            status = INTERNAL_SERVER_ERROR,
            description = "Failed to deactivate goal"
        )
    ),
    tag = "v0/quizzes",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub async fn deactivate_goal(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(goal_id): Path<Uuid>,
) -> Result<StatusCode, QuizError> {
    let goal = hikari_db::quiz::goal::query::Query::get_goal_by_id_and_user(&conn, &goal_id, &user_id)
        .await?
        .ok_or(QuizError::GoalNotFound)?;

    hikari_db::quiz::goal::mutation::Mutation::deactivate_goal(&conn, &goal.id).await?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/api/v0/quizzes/goal/feedback",
    request_body = FeedbackInput,
    responses(
        (
            status = OK,
            body = GoalFeedback,
            description = "Goal feedback successfully generated"
        ),
        (
            status = UNAUTHORIZED,
            description = "Authentication required"
        ),
        (
            status = INTERNAL_SERVER_ERROR,
            description = "Failed to generate goal feedback"
        )
    ),
    tag = "v0/quizzes",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
async fn generate_goal_feedback(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Extension(app_config): Extension<AppConfig>,
    Json(payload): Json<FeedbackInput>,
) -> Result<Response, QuizError> {
    let llm_config: &LlmConfig = AppConfig::llm_config(&app_config);
    let feedback = hikari_core::quiz::goal::generate_feedback(&user_id, &payload, llm_config, &conn).await?;

    Ok(Json(feedback).into_response())
}

async fn get_quiz_by_id(conn: &DatabaseConnection, user_id: &Uuid, quiz_id: &Uuid) -> Result<Quiz, QuizError> {
    let result = hikari_db::quiz::quiz::Query::get_quiz_by_id(conn, user_id, quiz_id)
        .await?
        .ok_or(QuizError::QuizNotFound)?
        .into_model();
    Ok(result)
}

async fn get_question_by_id(conn: &DatabaseConnection, question_id: &Uuid) -> Result<Question, QuizError> {
    let result: Question = hikari_db::quiz::question::Query::get_question_by_id(conn, question_id)
        .await?
        .ok_or(QuizError::QuestionNotFound)?
        .into_model();
    //result.sanitize_for_client();
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

#[derive(Debug, Serialize)]
struct RecommendationRequest {
    student_id: Uuid,
    module_id: String,
    chapter_bloom_limits: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct Recommendation {
    question_id: Uuid,
    #[allow(dead_code)]
    question_text: String, // TODO remove this field (also from LLM schema) if not needed
}

#[derive(Debug, Deserialize)]
pub struct RecommendationResponse {
    recommendations: Vec<Recommendation>,
}

pub struct RecommendationClient {
    client: Client,
    base_url: String,
}

impl RecommendationClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    async fn get_recommendations(
        &self,
        user_id: Uuid,
        module_id: String,
        sessions: Vec<Session>,
        conn: &DatabaseConnection,
    ) -> Result<RecommendationResponse, QuizError> {
        let mut chapter_bloom_limits: HashMap<String, String> = HashMap::new();

        for session in &sessions {
            let topics: Vec<String> = session.contents.iter().map(|content| content.title.clone()).collect();

            let limits = get_chapter_bloom_limits(conn, &user_id, &session.id, &topics).await?;

            chapter_bloom_limits.extend(limits);
        }

        let request = RecommendationRequest {
            student_id: user_id,
            module_id,
            chapter_bloom_limits: Some(chapter_bloom_limits),
        };

        let response = self
            .client
            .post(format!("{}/recommendations", self.base_url))
            .json(&request)
            .send()
            .await?
            .error_for_status()?
            .json::<RecommendationResponse>()
            .await?;

        Ok(response)
    }
}
async fn get_chapter_bloom_limits(
    conn: &DatabaseConnection,
    user_id: &Uuid,
    session_id: &str,
    topics: &[String],
) -> Result<HashMap<String, String>, QuizError> {
    let mut limits = HashMap::new();

    for topic in topics {
        let score = hikari_db::quiz::score::Query::get_score_by_topic(conn, user_id, session_id, topic).await?;

        let level = get_bloom_level(score.unwrap_or(0.0));

        limits.insert(topic.clone(), level.to_string());
    }

    Ok(limits)
}
fn get_bloom_level(score: f64) -> QuestionBloomLevel {
    let levels = [
        (25.0, QuestionBloomLevel::Create),
        (20.0, QuestionBloomLevel::Evaluate),
        (15.0, QuestionBloomLevel::Analyze),
        (10.0, QuestionBloomLevel::Apply),
        (5.0, QuestionBloomLevel::Understand),
        (0.0, QuestionBloomLevel::Remember),
    ];

    levels
        .into_iter()
        .find(|(threshold, _)| score >= *threshold)
        .map(|(_, level)| level)
        .unwrap_or(QuestionBloomLevel::Remember)
}

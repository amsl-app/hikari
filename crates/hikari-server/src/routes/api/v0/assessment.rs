use crate::AppConfig;
use crate::data::assessment::{
    AnswerRequest, HasSeaOrmAnswerType, answer_value_to_string, get_or_start_session, require_running_session,
    submit_session,
};
use crate::permissions::Permission;
use crate::user::ExtractUserId;
use axum::Extension;
use axum::Json;
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::routing::{Router, get, post, put};
use chrono::NaiveDateTime;
use error::Error;
use hikari_config::assessment::Assessment;
use hikari_config::assessment::AssessmentConfig;
use hikari_config::assessment::question::Answer;
use hikari_config::assessment::question::AnswerValue;
use hikari_config::assessment::question::QuestionBody;
use hikari_config::assessment::question::QuestionExt;
use hikari_config::assessment::scale::Mode;
use hikari_model::assessment::scales::ItemValue;
use hikari_model::assessment::session::AssessmentSession;
use hikari_model_tools::convert::IntoModel;
use http::StatusCode;
use indexmap::IndexMap;
use num_traits::ToPrimitive;
use protect_axum::protect;
use sea_orm::DatabaseConnection;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

pub(crate) mod error;

trait Operation {
    fn evaluate(&self, data: Vec<u8>) -> Result<f64, Error>;
}

impl Operation for Mode {
    fn evaluate(&self, data: Vec<u8>) -> Result<f64, Error> {
        let length = data.len();
        if length == 0 {
            return Err(Error::Other("No data to evaluate".to_owned()));
        }
        let sum = data.into_iter().map(f64::from).sum();
        let res = match self {
            Self::Sum => sum,
            Self::Average => {
                sum / length
                    .to_f64()
                    .ok_or_else(|| Error::Other("Failed to evaluate average".to_owned()))?
            }
        };
        Ok(res)
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SessionResponse {
    pub(crate) session_id: Uuid,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct SessionScales {
    pub(crate) completed: NaiveDateTime,
    pub(crate) scales: Vec<ItemValue>,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct AssessmentScales {
    pub(crate) assessment_id: String,
    pub(crate) sessions: Vec<SessionScales>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(crate) struct ListFlags {
    deep: Option<String>,
}

#[allow(deprecated)]
pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(list_assessments))
        .route("/sessions", get(list_user_assessments))
        .route("/scales", get(get_user_scales))
        .nest(
            "/{assessment}",
            Router::new()
                .route("/", get(get_assessment))
                .route("/start", post(start))
                .route("/scales", get(get_assessment_scales))
                .nest(
                    "/sessions",
                    Router::new().route("/", get(list_assessment_sessions)).nest(
                        "/{session}",
                        Router::new()
                            .route("/", get(get_session))
                            .route("/load", get(load))
                            .route("/scales", get(get_scales))
                            .route("/submit", post(submit))
                            .route("/update/{question}", put(update)),
                    ),
                ),
        )
        .with_state(())
}

#[utoipa::path(
    get,
    path = "/api/v0/assessments",
    responses(
        (status = OK, body = [Assessment], description = "Returns all available assessments"),
    ),
    tag = "v0/assessment",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]

pub(crate) async fn list_assessments(Extension(app_config): Extension<AppConfig>) -> Result<impl IntoResponse, Error> {
    let assessments = app_config.assessments().assessments().values().collect::<Vec<_>>();
    Ok(Json(assessments).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v0/assessments/sessions",
    responses(
        (status = OK, body = [AssessmentSession], description = "Returns all assessments started by the current user"),
    ),
    params(),
    tag = "v0/assessment",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]
pub(crate) async fn list_user_assessments(
    ExtractUserId(user): ExtractUserId,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, Error> {
    tracing::trace!(user = user.as_hyphenated().to_string(), "list assessment sessions");
    let config = app_config.assessments();
    let sessions_with_answers = hikari_db::assessment::answer::Query::load_answers_for_sessions(&conn, user).await?;
    tracing::debug!(
        user = user.as_hyphenated().to_string(),
        sessions = ?sessions_with_answers,
        "loaded assessment sessions"
    );
    let response = sessions_with_answers
        .into_iter()
        .map(|(session, answers)| {
            let assessment = config.get(&session.assessment).ok_or(Error::AssessmentConfigNotFound)?;
            generate_answered_assessment(&answers, assessment, &session)
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/v0/assessments/{assessment}",
    responses(
        (status = OK, body = Assessment, description = "Returns a single assessment"),
    ),
    params(
        ("assessment" = String, Path, description = "the assessment id of the assessment, which should be processed"),
    ),
    tag = "v0/assessment",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]
pub(crate) async fn get_assessment(
    Extension(app_config): Extension<AppConfig>,
    Path(assessment): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let assessment = app_config
        .assessments()
        .get(&assessment)
        .ok_or(Error::AssessmentConfigNotFound)?;

    Ok(Json(assessment).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/assessments/{assessment}/start",
    responses(
        (status = OK, body = SessionResponse, description = "Starts a new assessment"),
    ),
    params(
        ("assessment" = String, Path, description = "the assessment id of the assessment, which should be processed"),
    ),
    tag = "v0/assessment",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]

pub(crate) async fn start(
    ExtractUserId(user_id): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Extension(app_config): Extension<AppConfig>,
    Path(assessment): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let assessment = app_config
        .assessments()
        .get(&assessment)
        .ok_or(Error::AssessmentConfigNotFound)?;

    let session = get_or_start_session(&conn, user_id, &assessment.assessment_id).await?;

    Ok(Json(SessionResponse { session_id: session.id }))
}

#[utoipa::path(
    get,
    path = "/api/v0/assessments/scales",
    responses(
        (status = OK, body = [AssessmentScales], description = "Returns all scales for every assessment"),
    ),
    tag = "v0/assessment",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn get_user_scales(
    ExtractUserId(user_id): ExtractUserId,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, Error> {
    tracing::trace!(
        user_id = %user_id.as_hyphenated(),
        "getting scale values for all assessments"
    );

    let sessions_with_answers = hikari_db::assessment::answer::Query::load_answers_for_sessions(&conn, user_id).await?;

    let mut scales: HashMap<String, Vec<SessionScales>> = HashMap::new();
    for (session, answers) in sessions_with_answers {
        let Some(completed) = session.completed else {
            continue;
        };
        let assessment = app_config
            .assessments()
            .get(&session.assessment)
            .ok_or(Error::AssessmentConfigNotFound)?;
        let scale = build_scale_answers(assessment, &answers)?;
        scales
            .entry(session.assessment.clone())
            .or_default()
            .push(SessionScales {
                completed,
                scales: scale,
            });
    }

    let response: Vec<AssessmentScales> = scales
        .into_iter()
        .map(|(assessment_id, sessions)| AssessmentScales {
            assessment_id,
            sessions,
        })
        .collect();

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/v0/assessments/{assessment}/scales",
    responses(
        (status = OK, body = [SessionScales], description = "Returns all scales"),
    ),
    params(
        ("assessment" = String, Path, description = "the assessment id of the assessment, which should be processed"),
    ),
    tag = "v0/assessment",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn get_assessment_scales(
    ExtractUserId(user_id): ExtractUserId,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
    Path(assessment): Path<String>,
) -> Result<impl IntoResponse, Error> {
    tracing::trace!(
        user_id = %user_id.as_hyphenated(),
        assessment = %assessment,
        "getting scale values for whole assessment"
    );
    let assessment = app_config
        .assessments()
        .get(&assessment)
        .ok_or(Error::AssessmentConfigNotFound)?;

    let session =
        hikari_db::assessment::answer::Query::load_answers_for_assessment(&conn, &assessment.assessment_id, user_id)
            .await?;

    let scales: Vec<SessionScales> = session
        .into_iter()
        .map(|(session, answeres)| {
            if let Some(completed) = session.completed {
                let scale = build_scale_answers(assessment, &answeres)?;
                Ok(Some(SessionScales {
                    completed,
                    scales: scale,
                }))
            } else {
                Ok(None)
            }
        })
        .collect::<Result<Vec<Option<SessionScales>>, Error>>()?
        .into_iter()
        .flatten()
        .collect();

    return Ok(Json(scales));
}

#[utoipa::path(
    get,
    path = "/api/v0/assessments/{assessment}/sessions",
    responses(
        (status = OK, body = [AssessmentSession], description = "Returns all sessions for a specific assessment"),
    ),
    params(
        ("assessment" = String, Path, description = "the assessment id of the assessment, which should be processed"),
    ),
    tag = "v0/assessment",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]

pub(crate) async fn list_assessment_sessions(
    ExtractUserId(user_id): ExtractUserId,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
    Path(assessment): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let assessment = app_config
        .assessments()
        .get(&assessment)
        .ok_or(Error::AssessmentConfigNotFound)?;

    tracing::trace!(
        user = user_id.as_hyphenated().to_string(),
        assessment = assessment.assessment_id,
        "list assessment sessions"
    );

    let sessions_with_answers =
        hikari_db::assessment::answer::Query::load_answers_for_assessment(&conn, &assessment.assessment_id, user_id)
            .await?;

    tracing::debug!(
        user = user_id.as_hyphenated().to_string(),
        assessment = assessment.assessment_id,
        sessions = ?sessions_with_answers,
        "loaded assessment sessions"
    );

    let response = sessions_with_answers
        .into_iter()
        .map(|(session, answers)| generate_answered_assessment(&answers, assessment, &session))
        .collect::<Result<Vec<_>, Error>>()?;

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/v0/assessments/{assessment}/sessions/{session}",
    responses(
        (status = OK, body = AssessmentSession, description = "Returns all questions with if answered the saved reply"),
    ),
    params(
        ("session" = String, Path, description = "the session id of the assessment which should be loaded"),
        ("assessment" = String, Path, description = "the assessment id of the assessment, which should be processed"),
    ),
    tag = "v0/assessment",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]
pub(crate) async fn get_session(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Extension(app_config): Extension<AppConfig>,
    Path((assessment, session)): Path<(String, Uuid)>,
) -> Result<impl IntoResponse, Error> {
    let res = inner_load_session(&conn, app_config.assessments(), user, &assessment, session).await?;
    Ok(Json(res))
}

#[utoipa::path(
    get,
    path = "/api/v0/assessments/{assessment}/sessions/{session}/load",
    responses(
        (status = OK, body = AssessmentSession, description = "Returns all questions with if answered the saved reply"),
    ),
    params(
        ("session" = String, Path, description = "the session id of the assessment which should be loaded"),
        ("assessment" = String, Path, description = "the assessment id of the assessment, which should be processed"),
    ),
    tag = "v0/assessment",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]
#[deprecated(note = "use get_session instead")]
pub(crate) async fn load(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Extension(app_config): Extension<AppConfig>,
    Path((assessment, session)): Path<(String, Uuid)>,
) -> Result<impl IntoResponse, Error> {
    let res = inner_load_session(&conn, app_config.assessments(), user, &assessment, session).await?;
    Ok(Json(res))
}

async fn inner_load_session(
    conn: &DatabaseConnection,
    config: &AssessmentConfig,
    user: Uuid,
    assessment: &str,
    session: Uuid,
) -> Result<AssessmentSession, Error> {
    let config = config.get(assessment).ok_or(Error::AssessmentConfigNotFound)?;

    let (entry, answers) = hikari_db::assessment::answer::Query::load_answers_for_session(conn, session, user).await?;

    if entry.assessment.ne(&assessment) {
        return Err(Error::UnrelatedSessionId);
    }

    let res = generate_answered_assessment(&answers, config, &entry)?;

    Ok(res)
}

#[utoipa::path(
    post,
    request_body = [AnswerRequest],
    path = "/api/v0/assessments/{assessment}/sessions/{session}/submit",
    responses(
        (status = OK, description = "Persists the answers and marks this session as finished"),
    ),
    params(
        ("session" = String, Path, description = "the session id of the assessment which should be submitted"),
        ("assessment" = String, Path, description = "the assessment id of the assessment, which should be processed"),
    ),
    tag = "v0/assessment",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]

pub(crate) async fn submit(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Extension(app_config): Extension<AppConfig>,
    Path((assessment, session)): Path<(String, Uuid)>,
    Json(body): Json<Vec<AnswerRequest>>,
) -> Result<impl IntoResponse, Error> {
    tracing::trace!(
        assessment_id = assessment,
        session_id = session.as_hyphenated().to_string(),
        "submit assessment session"
    );

    let entry = require_running_session(&conn, user, &assessment).await?;
    if entry.id != session {
        return Err(Error::UnrelatedSessionId);
    }

    // Assessments started outside of a module have no module to attribute the history entry to
    submit_session(&conn, user, entry, app_config.assessments(), body).await?;

    Ok(StatusCode::OK.into_response())
}

#[utoipa::path(
    get,
    path = "/api/v0/assessments/{assessment}/sessions/{session}/scales",
    responses(
        (status = OK, body = [ItemValue], description = "Returns all scales"),
    ),
    params(
        ("assessment" = String, Path, description = "the assessment id of the assessment, which should be processed"),
        ("session" = String, Path, description = "the session id of the assessment which should be processed"),
    ),
    tag = "v0/assessment",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]

pub(crate) async fn get_scales(
    ExtractUserId(user): ExtractUserId,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
    Path((assessment_id, session)): Path<(String, Uuid)>,
) -> Result<impl IntoResponse, Error> {
    let result = get_scale_values(user, app_config.assessments(), &conn, assessment_id, session).await?;
    Ok(Json(result))
}

#[utoipa::path(
    put,
    request_body = AnswerValue,
    path = "/api/v0/assessments/{assessment}/sessions/{session}/update/{question}",
    responses(
        (status = CREATED, description = "Saves the changes to the selected assessment"),
    ),
    params(
        ("assessment" = String, Path, description = "the assessment id of the assessment, which should be processed"),
        ("session" = String, Path, description = "the session id of the assessment which should be updated"),
        ("question" = String, Path, description = "the question id of the question of which the answer should be set"),
    ),
    tag = "v0/assessment",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]

pub(crate) async fn update(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Extension(app_config): Extension<AppConfig>,
    Path((assessment, session, question)): Path<(String, Uuid, String)>,
    Json(body): Json<AnswerValue>,
) -> Result<impl IntoResponse, Error> {
    let entry = hikari_db::assessment::session::Query::load_session(&conn, user, session).await?;

    if entry.assessment.ne(&assessment) {
        return Err(Error::UnrelatedSessionId);
    }

    if entry.status != hikari_entity::assessment::session::AssessmentStatus::Running {
        return Err(Error::NotRunning);
    }
    let (_, question) = app_config
        .assessments()
        .get(&entry.assessment)
        .ok_or(Error::AssessmentConfigNotFound)?
        .questions
        .iter()
        .find(|(_, q)| q.id.as_str() == question)
        .ok_or(Error::AnswerNotFound)?;

    question.validate(&body).map_err(|error| {
        tracing::error!(error = &error as &dyn std::error::Error, "Failed to validate answer");
        Error::InvalidAnswer
    })?;

    hikari_db::assessment::answer::Mutation::insert_or_update(
        &conn,
        session,
        question.id.clone(),
        question.sea_orm_answer_type(),
        answer_value_to_string(body),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn get_scale_values(
    user_id: Uuid,
    config: &AssessmentConfig,
    conn: &DatabaseConnection,
    assessment_id: String,
    session: Uuid,
) -> Result<Vec<ItemValue>, Error> {
    tracing::trace!(
        user_id = %user_id.as_hyphenated(),
        session_id = %session.as_hyphenated(),
        "getting scale values"
    );
    let assessment_session = hikari_db::assessment::session::Query::load_session(conn, user_id, session).await?;

    if assessment_session.assessment.ne(&assessment_id) || session != assessment_session.id {
        return Err(Error::UnrelatedSessionId);
    }
    if assessment_session.status == hikari_entity::assessment::session::AssessmentStatus::Running {
        return Err(Error::NotCompleted);
    }
    tracing::trace!(
        user_id = %user_id.as_hyphenated(),
        session_id = %session.as_hyphenated(),
        "found completed session"
    );
    let answers = hikari_db::assessment::answer::Query::load_answers(conn, session).await?;

    let Some(assessment) = config.get(&assessment_id) else {
        tracing::error!(assessment_id, "assessment config does not exist");
        return Err(Error::AssessmentConfigNotFound);
    };

    build_scale_answers(assessment, &answers)
}

fn build_scale_answers(
    assessment: &Assessment,
    answers: &[hikari_entity::assessment::answer::Model],
) -> Result<Vec<ItemValue>, Error> {
    let answers: HashMap<_, _> = answers
        .iter()
        .map(|answer| (answer.question.as_str(), answer))
        .collect();

    let result: Result<Vec<_>, Error> = assessment
        .scales
        .values()
        .map(|scale| {
            let values: Result<Vec<u8>, Error> = scale
                .items
                .iter()
                .map(|item| {
                    let question = assessment
                        .questions
                        .get(item.id.as_str())
                        .ok_or(Error::QuestionIdDoesNotExist(item.id.clone()))?;
                    let (min, max) = match &question.body {
                        QuestionBody::Scale(scale) => (scale.min, scale.max),
                        scale_type => {
                            return Err(Error::InvalidScaleType(Into::<&str>::into(scale_type).to_owned()));
                        }
                    };

                    match answers.get(item.id.as_str()) {
                        Some(&answer) => answer
                            .data
                            .parse::<u8>()
                            .map_err(|_| {
                                tracing::error!(data = answer.data, "Failed to parse data as u8");
                                Error::InvalidValue(answer.data.clone())
                            })
                            .map(|val| if item.reverse { max + min - val } else { val }),
                        None => Err(Error::AnswerNotFound),
                    }
                })
                .collect();
            let values = values?;
            Ok(ItemValue {
                id: scale.id.clone(),
                title: scale.title.clone(),
                value: scale.mode.evaluate(values)?,
            })
        })
        .collect();
    if let Err(error) = &result {
        tracing::error!(
            assessment_id = assessment.assessment_id,
            error = error as &dyn std::error::Error,
            "failed to build scale values"
        );
    }
    result
}

fn generate_answered_assessment(
    answers: &[hikari_entity::assessment::answer::Model],
    assessment: &Assessment,
    entry: &hikari_entity::assessment::session::Model,
) -> Result<AssessmentSession, Error> {
    let answers = answers
        .iter()
        .map(|f| (f.question.as_str(), f))
        .collect::<HashMap<&str, _>>();

    let questions: Result<IndexMap<_, _>, Error> = assessment
        .questions
        .values()
        .map(|question| {
            let mut answered_question = question.clone();
            answered_question.answer = match answers.get(question.id.as_str()) {
                None => None,
                Some(&answer) => Some(match answered_question.body {
                    QuestionBody::Scale(_) => Answer::Scale(
                        answer
                            .data
                            .parse()
                            .map_err(|_| Error::InvalidValue(answer.data.clone()))?,
                    ),
                    QuestionBody::Textfield(_) | QuestionBody::Textarea(_) | QuestionBody::MultiChoice(_) => {
                        Answer::Text(answer.data.clone())
                    }
                    QuestionBody::Select(_) | QuestionBody::SingleChoice(_) => Answer::Bool(
                        answer
                            .data
                            .parse()
                            .map_err(|_| Error::InvalidValue(answer.data.clone()))?,
                    ),
                }),
            };
            Ok((question.id.clone(), answered_question))
        })
        .collect();

    let questions = questions
        .inspect_err(|error| tracing::error!(error = error as &dyn std::error::Error, "failed to parse data"))?;

    let assessment = Assessment {
        assessment_id: assessment.assessment_id.clone(),
        title: assessment.title.clone(),
        questions,
        scales: assessment.scales.clone(),
        weight: assessment.weight,
        hidden: assessment.hidden,
    };

    Ok(AssessmentSession {
        session_id: entry.id,
        status: entry.status.into_model(),
        completed: entry.completed.as_ref().map(NaiveDateTime::and_utc),
        assessment,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::LazyLock;

    use hikari_config::assessment::{
        question::{AssessmentQuestion, LikertScaleBody, SelectBody},
        scale::{Item, Scale, ScaleBody},
    };
    use hikari_entity::assessment::answer::{AnswerType, Model as Answer};

    use test_log::test;

    const QUESTION_ID_1: &str = "test-1";
    const QUESTION_ID_2: &str = "test-2";
    const QUESTION_ID_3: &str = "test-3";
    const SCALE_ID: &str = "test-scale";
    const ASSESSMENT_ID: &str = "test-assessment";
    static ASSESSMENT_CONFIG: LazyLock<AssessmentConfig> = LazyLock::new(|| AssessmentConfig {
        assessments: IndexMap::from([(
            ASSESSMENT_ID.to_owned(),
            Assessment {
                assessment_id: ASSESSMENT_ID.to_owned(),
                title: "Test Assessment".to_owned(),
                questions: IndexMap::from([
                    (
                        QUESTION_ID_1.to_owned(),
                        AssessmentQuestion {
                            id: QUESTION_ID_1.to_owned(),
                            title: "Test One".to_owned(),
                            body: QuestionBody::Scale(LikertScaleBody {
                                min: 1,
                                max: 5,
                                hint_min: None,
                                hint_max: None,
                            }),
                            answer: None,
                        },
                    ),
                    (
                        QUESTION_ID_2.to_owned(),
                        AssessmentQuestion {
                            id: QUESTION_ID_2.to_owned(),
                            title: "Test Two".to_owned(),
                            body: QuestionBody::Scale(LikertScaleBody {
                                min: 1,
                                max: 5,
                                hint_min: None,
                                hint_max: None,
                            }),
                            answer: None,
                        },
                    ),
                    (
                        QUESTION_ID_3.to_owned(),
                        AssessmentQuestion {
                            id: QUESTION_ID_3.to_owned(),
                            title: "Test Two".to_owned(),
                            body: QuestionBody::Select(SelectBody { yes: None, no: None }),
                            answer: None,
                        },
                    ),
                ]),
                scales: IndexMap::from([(
                    SCALE_ID.to_owned(),
                    Scale {
                        id: SCALE_ID.to_string(),
                        title: "Test Scale".to_string(),
                        description: None,
                        body: ScaleBody::Scale {
                            min: 1,
                            max: 5,
                            reference: None,
                        },
                        mode: Mode::Average,
                        items: vec![
                            Item {
                                id: QUESTION_ID_1.to_owned(),
                                reverse: false,
                            },
                            Item {
                                id: QUESTION_ID_2.to_owned(),
                                reverse: false,
                            },
                        ],
                    },
                )]),
                weight: None,
                hidden: false,
            },
        )]),
    });

    #[test]
    fn test_build_scale_answers() {
        let answers = build_scale_answers(
            ASSESSMENT_CONFIG.get(ASSESSMENT_ID).unwrap(),
            &[
                Answer {
                    assessment_session_id: Uuid::new_v4(),
                    answer_type: AnswerType::Int,
                    question: QUESTION_ID_1.to_string(),
                    data: 3.to_string(),
                },
                Answer {
                    assessment_session_id: Uuid::new_v4(),
                    answer_type: AnswerType::Int,
                    question: QUESTION_ID_2.to_string(),
                    data: 5.to_string(),
                },
            ],
        )
        .unwrap();

        assert!((answers[0].value - 4.0).abs() < f64::EPSILON);
    }
}

use crate::AppConfig;
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
use hikari_config::assessment::question::Question;
use hikari_config::assessment::question::QuestionBody;
use hikari_config::assessment::question::QuestionExt;
use hikari_config::assessment::scale::Mode;
use hikari_db::assessment::answer::QuestionAnswer;
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

trait HasSeaOrmAnswerType {
    fn sea_orm_answer_type(&self) -> hikari_entity::assessment::answer::AnswerType;
}

impl HasSeaOrmAnswerType for Question {
    fn sea_orm_answer_type(&self) -> hikari_entity::assessment::answer::AnswerType {
        match self.body {
            QuestionBody::Scale(_) => hikari_entity::assessment::answer::AnswerType::Int,
            QuestionBody::Textfield(_) | QuestionBody::Textarea(_) | QuestionBody::MultiChoice(_) => {
                hikari_entity::assessment::answer::AnswerType::Text
            }
            QuestionBody::Select(_) | QuestionBody::SingleChoice(_) => {
                hikari_entity::assessment::answer::AnswerType::Bool
            }
        }
    }
}

fn answer_value_to_string(val: AnswerValue) -> String {
    match val {
        AnswerValue::Bool { value } => value.to_string(),
        AnswerValue::Text { value } => value,
        AnswerValue::SmallInt { value } => value.to_string(),
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SessionResponse {
    pub(crate) session_id: Uuid,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(crate) struct ListFlags {
    deep: Option<String>,
}

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(list_assessments))
        .route("/sessions", get(list_user_assessments))
        .nest(
            "/{assessment}",
            Router::new().route("/start", post(start)).nest(
                "/sessions/{session}",
                Router::new()
                    .route("/load", get(load))
                    .route("/scales", get(get_scales))
                    .route("/submit", post(submit))
                    .route("/update/{question}", put(update)),
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
    let sessions = hikari_db::assessment::session::Query::load_sessions(&conn, user).await?;
    tracing::debug!(
        user = user.as_hyphenated().to_string(),
        sessions = ?sessions,
        "loaded assessment sessions"
    );
    let mut response = Vec::with_capacity(sessions.len());
    for session in sessions {
        response.push(load_answered_assessment(&conn, session, config).await?);
    }
    Ok(Json(response))
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
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Extension(app_config): Extension<AppConfig>,
    Path(assessment): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let assessment = app_config
        .assessments()
        .get(&assessment)
        .ok_or(Error::AssessmentConfigNotFound)?;
    let session =
        hikari_db::assessment::session::Mutation::new_assessment(&conn, user, assessment.assessment_id.clone()).await?;
    tracing::debug!(
        user_id = %user.as_hyphenated(),
        session_id = %session.id.as_hyphenated(),
        "started assessment session"
    );
    Ok(Json(SessionResponse { session_id: session.id }))
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

pub(crate) async fn load(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Extension(app_config): Extension<AppConfig>,
    Path((assessment, session)): Path<(String, Uuid)>,
) -> Result<impl IntoResponse, Error> {
    let entry = hikari_db::assessment::session::Query::load_session(&conn, user, session).await?;

    if entry.assessment.ne(&assessment) {
        return Err(Error::UnrelatedSessionId);
    }

    let res = load_answered_assessment(&conn, entry, app_config.assessments()).await?;

    Ok(Json(res))
}

// TODO change from 201 to 204
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
    Ok(StatusCode::CREATED.into_response())
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(tag = "type")]
#[schema(example = json!({"question_id": "some-id", "value": true}))]
pub(crate) struct AnswerRequest {
    pub question_id: String,

    #[serde(flatten)]
    pub answer: AnswerValue,
}

// TODO Set response code to 201 when frontend can handle it
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

    let entry = hikari_db::assessment::session::Query::load_session(&conn, user, session).await?;

    tracing::debug!(entry.assessment, "loaded assessment");

    if entry.assessment.ne(&assessment) {
        return Err(Error::UnrelatedSessionId);
    }

    if entry.status != hikari_entity::assessment::session::AssessmentStatus::Running {
        return Err(Error::NotRunning);
    }
    let question_answers = build_assessment_answers_sea_orm(&entry.assessment, app_config.assessments(), body)?;
    hikari_db::assessment::session::Mutation::finish_assessment(&conn, entry.id, question_answers).await?;
    // TODO remove response body when frontend can handle it
    Ok(StatusCode::OK.into_response()) //FIXME check which code is correct
}

#[utoipa::path(
    get,
    path = "/api/v0/assessments/{assessment_id}/sessions/{session}/scales",
    responses(
        (status = OK, body = [ItemValue], description = "Returns all scales"),
    ),
    params(
        ("assessment_id" = String, Path, description = "the assessment id of the assessment, which should be processed"),
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

pub(crate) fn build_assessment_answers_sea_orm(
    assessment: &str,
    config: &AssessmentConfig,
    answers: Vec<AnswerRequest>,
) -> Result<Vec<QuestionAnswer>, Error> {
    let assessment = config.get(assessment).ok_or(Error::AssessmentConfigNotFound)?;

    let mut answers = answers
        .into_iter()
        .map(|a| (a.question_id.clone(), a))
        .collect::<HashMap<_, _>>();
    assessment
        .questions
        .values()
        .map(|question| {
            Ok(QuestionAnswer {
                question: question.id.clone(),
                answer_type: question.sea_orm_answer_type(),
                data: answer_value_to_string(
                    answers
                        .remove(question.id.as_str())
                        .ok_or(Error::MissingAnswer(question.id.clone()))?
                        .answer,
                ),
            })
        })
        .collect::<Result<Vec<_>, Error>>()
}

async fn load_answered_assessment(
    conn: &DatabaseConnection,
    entry: hikari_entity::assessment::session::Model,
    config: &AssessmentConfig,
) -> Result<AssessmentSession, Error> {
    tracing::trace!(assessment_session_id = %entry.id, "load session answers");
    let answers = hikari_db::assessment::answer::Query::load_answers(conn, entry.id).await?;

    let assessment = config.get(&entry.assessment).ok_or(Error::AssessmentConfigNotFound)?;

    generate_answered_assessment(&answers, assessment, &entry)
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
        question::{LikertScaleBody, Question, SelectBody},
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
                        Question {
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
                        Question {
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
                        Question {
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
                        body: ScaleBody::Scale { min: 1, max: 5 },
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

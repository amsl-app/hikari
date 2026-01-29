use axum::extract::State;
use axum::routing::post;
use axum::{Extension, Json, Router};
use chrono::{DateTime, FixedOffset, NaiveDate};
use error::EndpointError;
use hikari_config::global::GlobalConfig;
use hikari_core::journal::summarize::summarize;
use hikari_core::llm_config::LlmConfig;
use hikari_db::config;
use hikari_db::sea_orm::DatabaseConnection;
use hikari_model::user::{Gender, User};
use hikari_model_tools::convert::{IntoDbModel, TryFromDbModel};
use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::json;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::Debug;
use std::num::ParseIntError;
use std::sync::Arc;
use strum::IntoStaticStr;
use utoipa::ToSchema;
use uuid::Uuid;

mod error;

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct EndpointRequest {
    client: Client,
    #[serde(flatten)]
    data: Data,
}

#[allow(dead_code)] // We are deserializing csml data here
#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct Client {
    // bot_id: String,
    user_id: Uuid,
    // channel_id: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct Name {
    name: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct Birthday {
    date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct Subject {
    subject: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct RawSemester {
    semester: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct Onboarding {
    onboarding: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct SetConfig {
    key: String,
    value: Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct ReadConfig {
    key: String,
}

#[derive(Debug)]
struct Semester {
    semester: Option<u8>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct GenderWrapper {
    gender: Option<Gender>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct Summarize {
    time: Option<DateTime<FixedOffset>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct SortByKeys {
    data: HashMap<String, HashMap<String, Option<f64>>>,
    keys: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct SortMaps {
    data: Vec<HashMap<String, Option<f64>>>,
}

#[derive(Debug, Deserialize, ToSchema, IntoStaticStr)]
#[serde(tag = "function_id", content = "data", rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
pub(crate) enum Data {
    SetName(Name),

    SetBirthday(Birthday),

    SetSemester(RawSemester),

    SetSubject(Subject),

    SetGender(GenderWrapper),

    SetOnboarding(Onboarding),

    #[serde(rename = "set_config_value")]
    SetConfig(SetConfig),

    #[serde(rename = "read_config_value")]
    ReadConfig(ReadConfig),

    JournalSummarize(Summarize),

    SortByKeys(SortByKeys),

    SortMaps(SortMaps),
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EndpointResponse {
    data: Value,
}

pub struct RouterState {
    pub config: GlobalConfig,
    pub llm_config: LlmConfig,
}

pub fn create_router<S>(config: GlobalConfig, llm_config: LlmConfig) -> Router<S> {
    let state = Arc::new(RouterState { config, llm_config });
    Router::new().route("/endpoint", post(endpoint)).with_state(state)
}

struct EndpointConfig<'a> {
    pub(crate) hikari: &'a GlobalConfig,
    pub(crate) llm_config: &'a LlmConfig,
}

async fn handle_request(
    conn: DatabaseConnection,
    client: Client,
    data: Data,
    config: EndpointConfig<'_>,
) -> Result<Value, EndpointError> {
    match data {
        Data::SetName(request) => update_user_name(client, request, &conn).await,
        Data::SetBirthday(request) => update_user_birthday(client, request, &conn).await,
        Data::SetSemester(request) => update_user_semester(client, parse_semester(request)?, &conn).await,
        Data::SetSubject(request) => update_user_subject(client, request, &conn).await,
        Data::SetGender(request) => update_user_gender(client, request, &conn).await,
        Data::SetOnboarding(request) => update_user_onboarding(client, request, &conn).await,
        Data::SetConfig(request) => set_config(&conn, config.hikari, client, request).await,
        Data::ReadConfig(request) => read_config(&conn, client, request).await,
        Data::JournalSummarize(request) => {
            let summary = summarize(conn, client.user_id, Arc::new(config.llm_config.clone()), request.time)
                .await
                .map_err(|error| EndpointError::Other(error.to_string()))?;
            serde_json::to_value(summary).map_err(Into::into)
        }
        Data::SortByKeys(request) => {
            let data = request.data;
            let mut data: Vec<_> = data.into_iter().collect();
            for key in &request.keys {
                data.sort_by(|a, b| {
                    let key_a = a.1.get(key).copied().flatten().unwrap_or(0.0);
                    let key_b = b.1.get(key).copied().flatten().unwrap_or(0.0);
                    key_a.partial_cmp(&key_b).unwrap_or(Ordering::Equal)
                });
            }
            Ok(serde_json::to_value(
                data.into_iter().map(|(item, _)| item).collect::<Vec<_>>(),
            )?)
        }
        Data::SortMaps(request) => {
            let sort_data = request.data;
            let mut data: Vec<_> = sort_data
                .iter()
                .flat_map(|dict| dict.keys())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            for weights in &sort_data {
                data.sort_by(|&a, &b| {
                    let key_a = weights.get(a).copied().flatten().unwrap_or(0.0);
                    let key_b = weights.get(b).copied().flatten().unwrap_or(0.0);
                    key_a.partial_cmp(&key_b).unwrap_or(Ordering::Equal)
                });
            }
            Ok(serde_json::to_value(data.into_iter().collect::<Vec<_>>())?)
        }
    }
}

/// Csml endpoint for the App function
#[utoipa::path(
    get,
    path = "/api/v0/csml/endpoint",
    request_body = EndpointRequest,
    responses(
        (status = OK, body = EndpointResponse, example = json!( EndpointResponse { data: json!({"foo": "bar"}) } )),
    ),
    tag = "csml"
)]
async fn endpoint(
    State(state): State<Arc<RouterState>>,
    Extension(conn): Extension<DatabaseConnection>,
    Json(data): Json<Value>,
) -> Result<Json<EndpointResponse>, EndpointError> {
    tracing::debug!("received app endpoint request");

    let request: EndpointRequest = serde_json::from_value(data).map_err(|error| {
        tracing::error!(
            error = &error as &dyn Error,
            "failed to deserializing app endpoint request"
        );
        EndpointError::Json(error)
    })?;
    let user_id = request.client.user_id;
    sentry::configure_scope(|scope| {
        scope.set_user(Some(sentry::User {
            id: Some(user_id.to_string()),
            ..Default::default()
        }));
    });

    tracing::debug!(user_id = %user_id, app = Into::<&'static str>::into(&request.data), "handling csml endpoint request");
    let res = handle_request(
        conn,
        request.client,
        request.data,
        EndpointConfig {
            hikari: &state.config,
            llm_config: &state.llm_config,
        },
    )
    .await;

    let data = res.inspect_err(|error| {
        tracing::error!(error = error as &dyn Error, %user_id, "error during csml endpoint request");
    })?;
    Ok(Json(EndpointResponse { data }))
}

fn parse_semester(data: RawSemester) -> Result<Semester, ParseIntError> {
    let semester = data.semester.map(|raw| raw.parse()).transpose()?;
    Ok(Semester { semester })
}

macro_rules! impl_update_value {
    ($func:ident, $t:ty, $field:ident, $req:ident, $mapping_func:expr_2021) => {
        async fn $func<C: ConnectionTrait> (
            client: Client,
            data: $t,
            db: &C,
        ) -> Result<Value, EndpointError> {
            let res = hikari_db::user::Mutation::$func(db, client.user_id, data.$req.map($mapping_func)).await;
            let model = match res {
                Ok(model) => model,
                Err(err) => {
                    tracing::error!(error = &err as &dyn Error, concat!("failed to update ", stringify!($field)));
                    return Err(EndpointError::SeaOrm(err));
                }
            };

            tracing::trace!(user_id = %client.user_id, value = ?model.$field, stringify!(set $field));
            Ok(serde_json::to_value(User::try_from_db_model(model)?.$field)?)
        }
    };
    ($func:ident, $t:ty, $field:ident, $req:ident) => {
        impl_update_value!($func, $t, $field, $req, Into::into);
    };
    ($func:ident, $t:ty, $field:ident) => {
        impl_update_value!($func, $t, $field, $field, Into::into);
    };
}

impl_update_value!(update_user_name, Name, name);
impl_update_value!(update_user_birthday, Birthday, birthday, date);
impl_update_value!(update_user_subject, Subject, subject);
impl_update_value!(update_user_semester, Semester, semester, semester);
impl_update_value!(
    update_user_gender,
    GenderWrapper,
    gender,
    gender,
    IntoDbModel::into_db_model
);

async fn update_user_onboarding<C: ConnectionTrait>(
    client: Client,
    data: Onboarding,
    conn: &C,
) -> Result<Value, EndpointError> {
    let res = hikari_db::user::Mutation::update_user_onboarding(conn, client.user_id, data.onboarding).await;
    let model = res.map_err(|error| {
        tracing::error!(error = &error as &dyn Error, "failed to update onboarding");
        EndpointError::SeaOrm(error)
    })?;

    tracing::debug!(user_id = %client.user_id, value = %model.onboarding, "set onboarding");
    Ok(serde_json::to_value(model.onboarding)?)
}

async fn set_config<C: ConnectionTrait>(
    conn: &C,
    cfg: &GlobalConfig,
    client: Client,
    data: SetConfig,
) -> Result<Value, EndpointError> {
    let user_id = client.user_id;
    let body = data;
    let allowed = cfg.config().allowed_keys.contains(&body.key);
    if !allowed {
        return Err(EndpointError::Config(format!("key {} not allowed", body.key)));
    }

    config::Mutation::set_config_value(conn, user_id, body.key, serde_json::to_string(&body.value)?).await?;
    Ok(Value::Null)
}

async fn read_config<C: ConnectionTrait>(conn: &C, client: Client, body: ReadConfig) -> Result<Value, EndpointError> {
    let res = config::Query::get_config_value(conn, client.user_id, &body.key).await?;
    let Some(value) = res else {
        return Ok(Value::Null);
    };
    let value = serde_json::from_str(&value)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize() {
        let json = r#"{
            "client": {
                "user_id": "00000000-0000-0000-0000-000000000000"
            },
            "function_id": "set_name",
            "data": {
                "name": "test"
            }
        }"#;
        let request: EndpointRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.client.user_id, Uuid::nil());
        let Data::SetName(Name { name: Some(name) }) = request.data else {
            panic!("Request contains unexpected data");
        };
        assert_eq!(name, "test");
    }

    #[test]
    fn test_empty() {
        assert!(matches!(serde_json::to_value(()).unwrap(), Value::Null));
    }
}

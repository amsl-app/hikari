// TODO (LOW) replace german with english
use crate::AppConfig;
use crate::data::csml::{Bots, bot_info_from_csml, message_from_csml_message_data};
use crate::permissions::Permission;
use crate::routes::api::v0::bots::error::BotError;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, Utc};
use csml_engine::Client as CsmlClient;
use csml_engine::data::AsyncDatabase;
use csml_engine::data::models::BotOpt;
use error::MessageError;
use hikari_common::csml_utils::init_request;
use hikari_config::module::ModuleCategory;
use hikari_db::config;
use hikari_entity::config::Model as UserConfigModel;
use hikari_model::chat::{BotInfo, MessageResponse, Payload, RequestMetadata};
use hikari_model::user::User;
use protect_axum::protect;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::error::Error;

pub(crate) mod conversations;
pub(crate) mod error;
pub(crate) mod flows;
pub(crate) mod message;

#[derive(Deserialize, utoipa::ToSchema)]
pub(crate) struct Conversations {
    client_id: String,
}

#[derive(Serialize)]
pub(crate) struct Metadata {
    pub(crate) user: MetaUser,
    pub(crate) time: DateTime<FixedOffset>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) modules: Vec<ModuleMetadata>,
}

#[derive(Serialize)]
pub(crate) struct ModuleMetadata {
    pub(crate) id: String,
    pub(crate) category: ModuleCategory,
    pub(crate) completion: Option<NaiveDateTime>,
}
impl Metadata {
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(
        user: User,
        configs: Map<String, Value>,
        request_metadata: RequestMetadata,
        modules: Vec<ModuleMetadata>,
    ) -> Self {
        let semester = user.semester;
        Self {
            user: MetaUser {
                name: user.name,
                birthday: user.birthday,
                subject: user.subject,
                semester,
                gender: user.gender.map(|g| g.to_string()),
                groups: user.groups,
                configs: Value::Object(configs),
            },
            time: request_metadata.time.unwrap_or_else(|| Utc::now().fixed_offset()),
            modules,
        }
    }
}

#[derive(Serialize)]
pub(crate) struct MetaUser {
    pub(crate) name: Option<String>,
    pub(crate) birthday: Option<NaiveDate>,
    pub(crate) subject: Option<String>,
    pub(crate) semester: Option<u8>,
    pub(crate) gender: Option<String>,
    pub(crate) groups: Vec<String>,
    pub(crate) configs: Value,
}

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(list_bots))
        .nest(
            "/{id}",
            Router::new()
                .route("/message", post(message::message))
                .nest("/conversations", conversations::create_router())
                .nest("/flows", flows::create_router()),
        )
        .with_state(())
}

#[utoipa::path(
    get,
    path = "/api/v0/bots",
    responses(
        (status = OK, description = "List of bots and their flow ids", body = [BotInfo]),
    ),
    tag = "v0/bots",
    security(
        ("token" = [])
    )
)]
#[protect(
    "Permission::Basic
",
    ty = "Permission"
)]

pub(crate) async fn list_bots(Extension(app_config): Extension<AppConfig>) -> impl IntoResponse {
    let data = app_config
        .bots()
        .iter()
        .map(bot_info_from_csml)
        .collect::<Vec<BotInfo>>();

    // Have to call into_response() because of lifetime issues
    Json(data).into_response()
}

pub(crate) fn configs_into_map(configs: Vec<UserConfigModel>) -> Result<Map<String, Value>, serde_json::Error> {
    configs
        .into_iter()
        .map(|us| Ok((us.key, serde_json::from_str(&us.value)?)))
        .collect()
}

pub(crate) async fn start_conversation(
    user: User,
    bot_id: &str,
    channel: String,
    payload: Payload,
    metadata: RequestMetadata,
    bots: &Bots,
    db: &DatabaseConnection,
) -> Result<MessageResponse<Payload>, MessageError> {
    let payload = serde_json::to_value(payload)?;

    let csml_bot = bots.find(bot_id).ok_or_else(|| {
        //TODO maybe include more information, ex. user-information/ip, etc.
        tracing::warn!(%bot_id, "csml bot does not exist");
        BotError::BotNotFound
    })?;

    let client = csml_engine::Client {
        user_id: user.id.as_hyphenated().to_string(),
        // TODO (MED) Load bot from db
        bot_id: String::from(bot_id),
        channel_id: channel,
    };

    let run_opt = BotOpt::CsmlBot(Box::new(csml_bot.clone()));

    let configs = config::Query::get_user_config(db, user.id).await?;
    let configs = configs_into_map(configs)?;

    let metadata = Metadata::new(user.clone(), configs, metadata, Vec::new());
    let metadata = serde_json::to_value(metadata)?;

    let mut csml_db = AsyncDatabase::sea_orm(db);
    let request = init_request(client.clone(), payload, metadata);
    let request_id = request.request_id.clone();

    let (conversation, messages) = csml_engine::future::start_conversation_db(request, run_opt, &mut csml_db)
        .await
        .inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, "error trying to start conversation");
        })?;

    let messages = messages.into_iter().map(message_from_csml_message_data).collect();

    Ok(MessageResponse {
        client: Some(client),
        request_id: Some(request_id),
        conversation_id: Some(conversation.id),
        messages,
        conversation_end: conversation.is_closed(),
        history: false,
        created_entities: vec![],
    })
}

// TODO (MED) Tests

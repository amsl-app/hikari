use crate::data::modules::session::get_session;
use crate::data::modules::{self};
use crate::permissions::Permission;
use crate::user::{ExtractUser, ExtractUserId};
use crate::{AppConfig, db};
use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use axum::{Extension, Router};
use csml_engine::data::AsyncDatabase;
use error::ModuleError;
use futures::future::try_join_all;
use futures::future::try_join3;
use hikari_db::module::session::status;
use hikari_db::util::{FlattenTransactionResultExt, InspectTransactionError};
use hikari_model::history::{HistoryEntry, HistoryEntryType};
use hikari_model::module::ModuleFull;
use hikari_model::module::group::ModuleGroupeFull;
use hikari_model::module::session::SessionFull;
use hikari_model::module::session::instance::SessionInstance;
use hikari_model_tools::convert::IntoModel;
use hikari_utils::loader::LoaderTrait;
use http::{HeaderValue, StatusCode, header};
use protect_axum::protect;
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde_derive::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::iter;
use utoipa::ToSchema;

pub(crate) mod assessment;
pub(crate) mod error;
pub(crate) mod messaging;
pub(crate) mod quiz;

#[derive(Serialize, ToSchema)]
pub(crate) struct ModuleContainer<'a> {
    modules: Vec<ModuleFull<'a>>,
    onboarding: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ModuleFlags {
    pub deep: Option<String>,
}

pub(crate) fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(list_modules))
        .route("/history", get(history))
        .route("/groups", get(list_groups))
        .route("/sources/{source}", get(get_source))
        .nest(
            "/{module}",
            Router::new()
                .route("/", get(get_module))
                .nest("/assessments/{pre_post}", assessment::create_router())
                .nest("/quizzes", quiz::create_router())
                .nest(
                    "/sessions",
                    Router::new()
                        .route("/abort", post(abort_all_sessions))
                        .route("/finished", get(list_finished_modules))
                        .nest(
                            "/{session}",
                            Router::new()
                                .route("/", get(get_session_data))
                                .route("/flow", get(flow_custom))
                                .route("/next", get(next_session_custom))
                                .route("/finish", post(finish_session))
                                .merge(messaging::create_router()),
                        ),
                ),
        )
        .with_state(())
}

#[utoipa::path(
    get,
    path = "/api/v0/modules",
    responses(
        (status = OK, body = ModuleContainer, description = "List all modules available for the user"),
    ),
    params(
        ("deep" = Option<String>, Query, description = "if set all modules are listed with their sessions"),
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

pub(crate) async fn list_modules(
    ExtractUser(user): ExtractUser,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
    Query(deep): Query<ModuleFlags>,
) -> Result<Response, ModuleError> {
    let conn = &conn;
    let (session_instances, module_status, assessments) = try_join3(
        status::Query::all(conn, user.id),
        hikari_db::module::status::Query::all(conn, user.id),
        hikari_db::module::assessment::Query::all(conn, user.id),
    )
    .await?;
    let session_instances: Vec<_> = session_instances.into_iter().map(IntoModel::into_model).collect();
    let module_completion: HashMap<_, _> = module_status.into_iter().map(|m| (m.module, m.completion)).collect();

    let module_cfg = app_config.module_config();

    let deep = deep.deep.is_some();
    let assessments = assessments
        .iter()
        .map(|ma| (&ma.module, ma.clone().into_model()))
        .collect::<HashMap<_, _>>();

    let res = module_cfg
        .modules_filtered(&user.groups)
        .into_iter()
        .map(|module| {
            ModuleFull::from_config(
                module,
                deep,
                &session_instances,
                assessments.get(&module.id),
                module_completion
                    .get(module.id.as_str())
                    .and_then(|module_status| module_status.as_ref().map(chrono::NaiveDateTime::and_utc)),
            )
        })
        .collect();

    let cfg = app_config.config();
    let onboarding = cfg.onboarding().module();

    let res = ModuleContainer {
        modules: res,
        onboarding,
    };
    Ok(Json(res).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v0/modules/groups",
    responses(
        (status = OK, body = [ModuleGroupeFull], description = "List all module groups"),
    ),
    tag = "v0/modules",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn list_groups(Extension(app_config): Extension<AppConfig>) -> Result<Response, ModuleError> {
    // TODO Maybe move somewhere else

    let module_config = app_config.module_config();
    let groups = app_config.config().module().groups();

    let full_groups = groups
        .iter()
        .map(|group| ModuleGroupeFull::from_config(group, module_config))
        .collect::<Vec<_>>();

    Ok(Json(full_groups).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v0/modules/{module}/sessions/finished",
    responses(
        (status = OK, body = [SessionInstance], description = "Lists all finished modules"),
    ),
    params(
        ("module" = String, Path, description = "module id from which the finished sessions should be listed"),
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

pub(crate) async fn list_finished_modules(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
    Path(module): Path<String>,
) -> Result<impl IntoResponse, ModuleError> {
    let res = status::Query::get_finished_sessions(&conn, user, &module).await?;

    let res: Vec<SessionInstance> = res.into_iter().map(IntoModel::into_model).collect();
    Ok(Json(res))
}

#[utoipa::path(
    get,
    path = "/api/v0/modules/{module}",
    responses(
        (status = OK, body = ModuleFull, description = "Returns the module"),
        (status = NOT_FOUND, description = "Module wasn't found"),
    ),
    params(
        ("module" = String, Path, description = "module id from the module"),
    ),
    tag = "v0/modules",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn get_module(
    ExtractUser(user): ExtractUser,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
    Path(module_id): Path<String>,
) -> Result<impl IntoResponse, ModuleError> {
    let session_instances: Vec<_> = status::Query::for_module(&conn, user.id, &module_id)
        .await?
        .into_iter()
        .map(IntoModel::into_model)
        .collect();

    let assessment = hikari_db::module::assessment::Query::get_for_module(&conn, user.id, &module_id)
        .await?
        .map(IntoModel::into_model);

    let module = app_config
        .module_config()
        .get_for_group(&module_id, &user.groups)
        .ok_or(modules::error::ModuleError::ModuleNotFound)?
        .clone();
    let module_status = hikari_db::module::status::Query::get_for_user(&conn, user.id, &module_id).await?;
    let res: ModuleFull = ModuleFull::from_config(
        &module,
        true,
        &session_instances,
        assessment.as_ref(),
        module_status.and_then(|status| status.completion).map(|c| c.and_utc()),
    );
    Ok(Json(res).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/modules/{module}/sessions/abort",
    responses(
        (status = NO_CONTENT),
    ),
    params(
        ("module" = String, Path, description = "module id from which the sessions should be aborted"),
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

pub(crate) async fn abort_all_sessions(
    ExtractUserId(user): ExtractUserId,
    Path(module_id): Path<String>,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, ModuleError> {
    tracing::debug!(%user, %module_id, "aborting all sessions for module");
    let res = conn
        .transaction(|txn| {
            Box::pin(async move {
                let instances = status::Query::for_module(txn, user, &module_id).await?;
                let abort_actions: Vec<_> = instances
                    .into_iter()
                    .map(|instance| {
                        db::sea_orm::module::session::user_session_status::abort_session_instance(txn, user, instance)
                    })
                    .collect();
                try_join_all(abort_actions).await?;
                Result::<(), ModuleError>::Ok(())
            })
        })
        .await;
    res.inspect_transaction_err(|error| {
        tracing::error!(
            error = error as &dyn std::error::Error,
            "error aborting session instances"
        );
    })
    .flatten_res()
    .map(|()| StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/v0/modules/{module}/sessions/{session}",
    responses(
        (status = OK, body = SessionFull, description = "Returns detailed information about the session"),
        (status = NOT_FOUND, description = "Module or session were deleted from cfg and are no longer available"),
    ),
    params(
        ("module" = String, Path, description = "the module id which should be used for the request"),
        ("session" = String, Path, description = "the session id which should be used for the request"),
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
pub(crate) async fn get_session_data(
    ExtractUser(user): ExtractUser,
    Path((module_id, session_id)): Path<(String, String)>,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, ModuleError> {
    let (_, session) = get_session(&module_id, &session_id, app_config.module_config(), &user.groups)?;

    let session_instances: Vec<_> = status::Query::for_module(&conn, user.id, &module_id)
        .await?
        .into_iter()
        .map(IntoModel::into_model)
        .collect();

    // TODO why should we create a new entry here?

    // let entry = status::Mutation::create(
    //     &conn,
    //     user.id,
    //     module_id,
    //     session_id,
    //     session.get_bot().map(String::from),
    // )
    // .await?
    // .into_model();

    // tracing::debug!("Received User Module {:?} ", entry);

    let res = SessionFull::from_config(session, &module_id, &session_instances);

    Ok(Json(res).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/modules/{module}/sessions/{session}/finish",
    responses(
        (status = NO_CONTENT, description = "Session was marked as finished"),
        (status = NOT_FOUND, description = "Module or session were deleted from cfg and are no longer available"),
    ),
    params(
        ("module" = String, Path, description = "the module id which should be used for the request"),
        ("session" = String, Path, description = "the session id which should be used for the request"),
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

pub(crate) async fn finish_session(
    ExtractUser(user): ExtractUser,
    Path((module_id, session_id)): Path<(String, String)>,
    Extension(app_config): Extension<AppConfig>,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, ModuleError> {
    let (module, session) = get_session(&module_id, &session_id, app_config.module_config(), &user.groups)?;
    let sessions = status::Query::get_finished_sessions(&conn, user.id, &module_id).await?;

    let bots = app_config.bots();

    let sessions_set: HashSet<_> = sessions
        .iter()
        .map(|m| &m.session)
        .chain(iter::once(&session_id))
        .collect();

    let module_completed = module
        .sessions
        .values()
        .all(|session| sessions_set.contains(&session.id));
    let bot_id = match session.get_bot() {
        Some(bot_id_or_name) => Some(
            bots.find(bot_id_or_name)
                .ok_or(ModuleError::ConfigurationError(format!(
                    "Configured bot {bot_id_or_name} does not exist"
                )))?
                .id
                .clone(),
        ),
        None => None,
    };

    let res = conn
        .transaction(|txn| {
            Box::pin(async move {
                let session_entry =
                    status::Mutation::create(txn, user.id, module_id.clone(), session_id.clone(), bot_id.clone())
                        .await?;
                let mut db = AsyncDatabase::sea_orm(txn);
                if let Some(bot_id) = bot_id {
                    let client =
                        messaging::generate_client(user.id, bot_id, &session_entry.module, &session_entry.session);
                    csml_engine::future::db_connectors::conversations::close_all_conversations(&client, &mut db)
                        .await?;
                }

                db::sea_orm::module::session::user_session_status::set_status_as_finished(
                    txn,
                    session_entry,
                    module_completed,
                )
                .await
                .map_err(ModuleError::from)
            })
        })
        .await;

    res.inspect_transaction_err(|error| {
        tracing::error!(error = error as &dyn std::error::Error, "error finishing session");
    })
    .flatten_res()
    .map(|()| StatusCode::NO_CONTENT)
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct NextSession<'a> {
    next_session: Option<&'a str>,
}

#[utoipa::path(
    get,
    path = "/api/v0/modules/{module}/sessions/{session}/next",
    responses(
        (status = OK, body = NextSession, description = "The next session", example = json ! ({ "next-session": "session-id"})),
        (status = NOT_FOUND, description = "Module or session weren't found"),
    ),
    params(
        ("module" = String, Path, description = "the module id which should be used for the request"),
        ("session" = String, Path, description = "the session id which should be used for the request"),
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
pub(crate) async fn next_session_custom(
    ExtractUser(user): ExtractUser,
    Extension(app_config): Extension<AppConfig>,
    Path((module_id, session_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ModuleError> {
    tracing::debug!(module = module_id, session = session_id, "getting next session");

    let (_, session) = get_session(&module_id, &session_id, app_config.module_config(), &user.groups)?;

    let session = session.next_session();

    Ok(Json(NextSession { next_session: session }).into_response())
}

#[derive(Serialize, ToSchema)]
pub(crate) struct Bot<'a> {
    bot: Option<&'a str>,
}

#[utoipa::path(
    get,
    path = "/api/v0/modules/{module}/sessions/{session}/flow",
    responses(
        (status = OK, body = Bot, description = "returns the corresponding bot and optionally flow", example = json ! ({ "bot": "<bot_id>[/<flow_id>]"})),
        (status = NOT_FOUND, description = "Module or session weren't found"),
    ),
    params(
        ("module" = String, Path, description = "the module id which should be used for the request"),
        ("session" = String, Path, description = "the session id which should be used for the request"),
    ),
    tag = "v0/modules",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn flow_custom(
    ExtractUser(user): ExtractUser,
    Extension(app_config): Extension<AppConfig>,
    Path((module_id, session_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ModuleError> {
    let (_, session) = get_session(&module_id, &session_id, app_config.module_config(), &user.groups)?;

    let bot = session.bot_flow();
    Ok(Json(Bot { bot }).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v0/modules/history",
    responses(
        (status = OK, body = [HistoryEntry], description = "returns the history of completed assessments, modules and sessions of current user"),
    ),
    tag = "v0/modules",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
pub(crate) async fn history(
    ExtractUserId(user): ExtractUserId,
    Extension(conn): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse, ModuleError> {
    let data = hikari_db::history::Query::load_history_entries(&conn, user).await?;
    let modules = data.module.into_iter().map(|(history, module)| HistoryEntry {
        completed: history.completed.and_utc(),
        value: HistoryEntryType::Module(module.into_model()),
    });
    let sessions = data.session.into_iter().map(|(history, session)| HistoryEntry {
        completed: history.completed.and_utc(),
        value: HistoryEntryType::Session(session.into_model()),
    });
    let assessments = data.assessment.into_iter().map(|(history, assessment)| HistoryEntry {
        completed: history.completed.and_utc(),
        value: HistoryEntryType::Assessment(assessment.into_model()),
    });
    let mut res = modules.chain(sessions).chain(assessments).collect::<Vec<_>>();
    res.sort_by(|a, b| b.completed.cmp(&a.completed));
    Ok(Json(res))
}

#[utoipa::path(
    get,
    path = "/api/v0/modules/sources/{source}",

    responses(
        (status = OK, description = "The document"),
    ),
    tag = "v0/modules",
    security(
        ("token" = [])
    )
)]
#[protect("Permission::Basic", ty = "Permission")]
async fn get_source(
    Extension(config): Extension<AppConfig>,
    Path(source): Path<String>,
) -> Result<impl IntoResponse, ModuleError> {
    let doc = config
        .llm_data()
        .documents
        .documents
        .get(&source)
        .ok_or_else(|| ModuleError::SourceNotFound(source))?;

    let file = config.llm_data().document_loader.load_file(&doc.file).await?;

    let path_str = doc.file.as_str();

    let file_type = if std::path::Path::new(path_str)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
    {
        "application/pdf"
    } else {
        "text/plain"
    };

    let headers = [
        (header::CONTENT_TYPE, HeaderValue::from_static(file_type)),
        (
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&format!("attachment; filename=\"{path_str}\""))
                .unwrap_or(HeaderValue::from_static("attachment")),
        ),
    ];

    Ok((headers, file.content))
}

extern crate core;
#[macro_use]
extern crate diesel_migrations;
use crate::data::WorkerUrl;
use crate::data::csml::Bots;
use crate::db::error::DbError::UnknownDbType;
use crate::db::migration;
use crate::opt::{Commands, Db, Run};
use anyhow::{Result, anyhow};
use axum::serve;
use clap::Parser;

use hikari_config::assessment::AssessmentConfig;
use hikari_config::constants::collection::ConstantCollection;
use hikari_config::documents::collection::DocumentCollection;
use hikari_config::global::GlobalConfig;
use hikari_config::module::ModuleConfig;
use hikari_core::llm_config::LlmConfig;
use hikari_db::sea_orm::{ConnectOptions, Database};
use hikari_db::tag;
use hikari_llm::builder::LlmStructureConfig;
use hikari_utils::loader::s3::S3Config;
use hikari_utils::loader::{Loader, LoaderHandler};
use hikari_utils::net::create_listener;
use std::env;
use std::error::Error;
use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use url::Url;

mod app;
mod auth;
mod data;
mod db;
mod opt;
mod permissions;
mod routes;
mod setup;
mod user;

const DEFAULT_HOST: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
const DEFAULT_PORT: u16 = 3030;

#[derive(Debug)]
pub(crate) struct InnerAppConfig {
    module_config: ModuleConfig,
    assessments: AssessmentConfig,
    bots: Bots,
    config: GlobalConfig,
    worker_url: WorkerUrl,
    llm_config: LlmConfig,
    llm_data: LlmData,
}

#[derive(Debug)]
pub(crate) struct LlmData {
    structures: LlmStructureConfig,
    constants: ConstantCollection,
    documents: DocumentCollection,
    document_loader: Loader,
}

impl LlmData {
    fn new(
        structures: LlmStructureConfig,
        constants: ConstantCollection,
        documents: DocumentCollection,
        document_loader: Loader,
    ) -> Self {
        Self {
            structures,
            constants,
            documents,
            document_loader,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AppConfig(Arc<InnerAppConfig>);

impl AppConfig {
    #[allow(clippy::too_many_arguments)]
    fn new(
        module_config: ModuleConfig,
        assessments: AssessmentConfig,
        config: GlobalConfig,
        bots: Bots,
        worker_url: WorkerUrl,
        llm_config: LlmConfig,
        llm_data: LlmData,
    ) -> Self {
        Self(Arc::new(InnerAppConfig {
            module_config,
            assessments,
            bots,
            config,
            worker_url,
            llm_config,
            llm_data,
        }))
    }

    pub fn module_config(&self) -> &ModuleConfig {
        &self.0.module_config
    }

    pub fn assessments(&self) -> &AssessmentConfig {
        &self.0.assessments
    }

    pub fn bots(&self) -> &Bots {
        &self.0.bots
    }

    pub fn config(&self) -> &GlobalConfig {
        &self.0.config
    }

    pub fn worker_url(&self) -> &Url {
        &self.0.worker_url.0
    }

    pub fn llm_config(&self) -> &LlmConfig {
        &self.0.llm_config
    }

    pub fn llm_data(&self) -> &LlmData {
        &self.0.llm_data
    }
}

//noinspection SpellCheckingInspection
async fn run(opt: Run) -> Result<()> {
    let _guard = hikari_utils::tracing::setup(
        hikari_utils::tracing::TracingConfig::builder()
            .package(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"))
            .otlp_endpoint(opt.otlp_endpoint)
            .sentry_dsn(opt.sentry_dsn)
            .env(opt.env.clone())
            .log_format(opt.log_format)
            .build(),
    )?;

    //TODO (Prio?) replace by command line argument
    let db_engine_type = env::var("ENGINE_DB_TYPE").map_err(|e| anyhow!("Cant find env: \"DATABASE_URL\" {e:?}"))?;
    let db_url_string = match db_engine_type.as_str() {
        #[cfg(feature = "sqlite")]
        "sqlite" => env::var("SQLITE_URL")?,

        #[cfg(feature = "postgres")]
        "postgresql" => env::var("POSTGRESQL_URL")?,

        _ => return Err(UnknownDbType(db_engine_type).into()),
    };
    let db_url = Url::parse(&db_url_string)?;
    migration(&db_url)
        .await
        .inspect_err(|error| tracing::error!(error = error as &dyn Error, "failed to run migrations"))?;

    let seaorm_pool_options = build_connect_options(&opt.db, db_url);
    let seaorm_pool = Database::connect(seaorm_pool_options).await?;
    let s3_config: Option<S3Config> = opt.s3.map(Into::into);
    let loader_handler = LoaderHandler::new(s3_config);
    let llm_config: LlmConfig = opt.llm_services.into();
    let llm_rag_documents_path = opt.llm_config.llm_collections;

    // ---- Load Bots
    let bots = match &opt.csml {
        Some(csml_path) => setup::load_bots(csml_path, &opt.worker_url, &loader_handler).await?,
        None => {
            tracing::warn!("no csml path provided, using empty bots");
            Bots::default()
        }
    };

    // ---- Load LLM
    let llm_structure_config = match &opt.llm_config.llm_structures {
        Some(llm_structures_path) => setup::load_llm_structures(llm_structures_path, &loader_handler).await?,
        None => {
            tracing::warn!("no llm structures path provided, using empty structures");
            LlmStructureConfig::default()
        }
    };
    let document_collection = setup::load_documents(&llm_rag_documents_path, &loader_handler).await?;
    let constants = setup::load_constants(opt.llm_config.constants.as_ref(), &loader_handler).await?;

    // ---- Load Assessments
    let assessment_config = setup::load_assessments(opt.assessment.as_ref(), &loader_handler).await?;

    // ---- Load Global Config
    let global_config = setup::load_config(opt.global_cfg.as_ref(), &loader_handler).await?;

    // ---- Load Modules
    let module_config = setup::load_modules(&opt.config, &loader_handler, &global_config, &document_collection).await?;

    module_config.validate(
        &assessment_config.ids(),
        &bots.ids(),
        &llm_structure_config.ids(),
        &global_config.module().ids(),
    )?;

    let journal_config = global_config.journal().clone();
    for focus in journal_config.focus {
        tag::Mutation::create_or_update_global_focus(&seaorm_pool, focus.name, focus.icon, false).await?;
    }

    let document_loader = loader_handler.loader(&llm_rag_documents_path)?;

    let llm_data = LlmData::new(
        llm_structure_config,
        constants,
        document_collection.clone(),
        document_loader.clone(),
    );

    // We want to upload the documents in the background so that the server can start quickly.
    let llm_config_clone = llm_config.clone();
    let seaorm_pool_clone = seaorm_pool.clone();

    tokio::spawn(async move {
        setup::upload_documents(
            document_collection,
            llm_config_clone,
            seaorm_pool_clone,
            document_loader,
        )
        .await;
    });

    let Run {
        worker_url,
        host,
        port,
        auth,
        deletable,
        ..
    } = opt;

    let app_config = AppConfig::new(
        module_config,
        assessment_config,
        global_config,
        bots,
        WorkerUrl(worker_url),
        llm_config,
        llm_data,
    );

    let app = app::create_app(app_config, auth, deletable, seaorm_pool).await?;

    let listener = create_listener((host, port), (DEFAULT_HOST, DEFAULT_PORT)).await?;

    let service = app.into_make_service();
    tracing::info!(local_addr = %listener.local_addr()?, "starting app");
    serve::serve(listener, service).await?;
    Ok(())
}

fn build_connect_options(db_options: &Db, db_url: Url) -> ConnectOptions {
    let mut seaorm_pool_options = ConnectOptions::new(db_url);
    if let Some(min_connections) = db_options.db_min_connections {
        seaorm_pool_options.min_connections(min_connections);
    }
    if let Some(max_connections) = db_options.db_max_connections {
        seaorm_pool_options.max_connections(max_connections);
    }
    seaorm_pool_options.sqlx_logging_level(log::LevelFilter::Debug);
    seaorm_pool_options
}

fn main() -> Result<()> {
    unsafe { env::set_var("RUST_BACKTRACE", "1") };

    let main = async {
        let opt = opt::Cli::parse();

        match opt.command {
            Commands::Run(o) => run(o).await?,
        }
        Ok(())
    };

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(main)
}

extern crate core;
#[macro_use]
extern crate diesel_migrations;
use crate::data::csml::Bots;
use crate::data::{WorkerUrl, modules};
use crate::db::error::DbError::UnknownDbType;
use crate::db::migration;
use crate::opt::{Commands, Db, Run};
use anyhow::{Result, anyhow};
use axum::serve;
use clap::Parser;

use csml_interpreter::data::CsmlResult;
use csml_interpreter::validate_bot;
use futures::FutureExt;
use hikari_config::assessment::AssessmentConfig;
use hikari_config::constants::collection::ConstantCollection;
use hikari_config::documents::collection::DocumentCollection;
use hikari_config::documents::document::{DocumentMetadata, DocumentType};
use hikari_config::global::GlobalConfig;
use hikari_config::module::ModuleConfig;
use hikari_core::llm_config::LlmConfig;
use hikari_core::pgvector::documents::{PgVectorDocument, RagDocumentLoaderFn};
use hikari_core::pgvector::{PgVector, upload_document};
use hikari_core::tts::config::TTSConfig;
use hikari_db::sea_orm::{ConnectOptions, Database};
use hikari_db::tag;
use hikari_llm::builder::LlmStructureConfig;
use hikari_utils::loader::s3::S3Config;
use hikari_utils::loader::{Loader, LoaderHandler, LoaderTrait};
use hikari_utils::net::create_listener;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use sea_orm::DatabaseConnection;
use std::env;
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
    tts_config: Option<TTSConfig>,
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
        tts_config: Option<TTSConfig>,
    ) -> Self {
        Self(Arc::new(InnerAppConfig {
            module_config,
            assessments,
            bots,
            config,
            worker_url,
            llm_config,
            llm_data,
            tts_config,
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

    pub fn tts_config(&self) -> Option<&TTSConfig> {
        self.0.tts_config.as_ref()
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
            .build(),
    );

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
        .inspect_err(|error| tracing::error!(error = error as &dyn std::error::Error, "failed to run migrations"))?;

    let seaorm_pool_options = build_connect_options(&opt.db, db_url);
    let seaorm_pool = Database::connect(seaorm_pool_options).await?;
    let s3_config: Option<S3Config> = opt.s3;
    let tts_config: Option<TTSConfig> = opt.tts.map(Into::into);
    let loader_handler = LoaderHandler::new(s3_config);
    let llm_config: LlmConfig = opt.llm_services.into();
    let llm_rag_documents_path = opt.llm_collections;

    // ---- Load Bots
    let bots = load_bots(&opt.csml, &opt.worker_url, &loader_handler).await?;

    // ---- Load LLM
    let llm_structure_config = load_llm_structures(&opt.llm_structures, &loader_handler).await?;
    let document_collection = load_documents(&llm_rag_documents_path, &loader_handler).await?;
    let constants = load_constants(opt.constants.as_ref(), &loader_handler).await?;

    // ---- Load Assessments
    let assessment_config = load_assessments(opt.assessment.as_ref(), &loader_handler).await?;

    // ---- Load Global Config
    let global_config = load_config(opt.global_cfg.as_ref(), &loader_handler).await?;

    // ---- Load Modules
    let module_config = load_modules(&opt.config, &loader_handler, &global_config, &document_collection).await?;

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
        if let Err(e) = upload_documents(
            document_collection,
            llm_config_clone,
            seaorm_pool_clone,
            document_loader,
        )
        .await
        {
            tracing::error!(error = ?e, "Failed to upload collections");
        } else {
            tracing::info!("Successfully uploaded collections");
        }
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
        tts_config,
    );

    let app = app::create_app(app_config, auth, deletable, seaorm_pool).await?;

    let listener = create_listener((host, port), (DEFAULT_HOST, DEFAULT_PORT)).await?;

    let service = app.into_make_service();
    tracing::info!(local_addr = %listener.local_addr()?, "starting app");
    serve::serve(listener, service).await?;
    Ok(())
}

async fn load_config(global_cfg: Option<&Url>, loader_handler: &LoaderHandler) -> Result<GlobalConfig> {
    let Some(url) = global_cfg else {
        return Ok(GlobalConfig::default());
    };
    let config = hikari_config::global::load(loader_handler.loader(url)?).await?;
    Ok(config)
}

async fn load_assessments(assessment_url: Option<&Url>, loader_handler: &LoaderHandler) -> Result<AssessmentConfig> {
    let Some(path) = assessment_url else {
        return Ok(AssessmentConfig::default());
    };
    let assessments = hikari_config::assessment::load(loader_handler.loader(path)?).await?;
    Ok(assessments)
}

async fn load_modules(
    config_url: &Url,
    loader_handler: &LoaderHandler,
    config: &GlobalConfig,
    llm_rag_documents: &DocumentCollection,
) -> Result<ModuleConfig> {
    let module_config =
        hikari_config::module::load_config(loader_handler.loader(config_url)?, llm_rag_documents).await?;

    let module_id = config.onboarding().module();
    if let Some(id) = module_id {
        module_config
            .get(id)
            .ok_or(modules::error::ModuleError::ModuleNotFound)?;
    }
    Ok(module_config)
}

async fn load_llm_structures(structured_url: &Url, loader_handler: &LoaderHandler) -> Result<LlmStructureConfig> {
    let llm_structures = hikari_llm::builder::load(loader_handler.loader(structured_url)?).await?;
    Ok(llm_structures)
}

async fn load_documents(documents_url: &Url, loader_handler: &LoaderHandler) -> Result<DocumentCollection> {
    let documents = hikari_config::documents::load(loader_handler.loader(documents_url)?).await?;
    Ok(documents)
}

async fn load_constants(constants_url: Option<&Url>, loader_handler: &LoaderHandler) -> Result<ConstantCollection> {
    let constants = if let Some(path_or_url) = constants_url {
        hikari_config::constants::load(loader_handler.loader(path_or_url)?).await?
    } else {
        ConstantCollection::default()
    };
    Ok(constants)
}

async fn load_bots(csml_url: &Url, worker_url: &Url, loader_handler: &LoaderHandler) -> Result<Bots> {
    let csml_endpoint = worker_url.join("api/v0/csml/endpoint")?.to_string();
    tracing::info!(%csml_endpoint, "setting endpoint url");
    let bots = data::bots::load_bots(loader_handler.loader(csml_url)?, Some(&csml_endpoint)).await?;

    bots.par_iter().try_for_each(|bot| {
        tracing::info!(bot = bot.id, "validating bot");
        let CsmlResult { warnings, errors, .. } = validate_bot(bot);
        for warning in warnings {
            tracing::warn!(warning = ?warning, bot = bot.id, "bot warning");
        }
        if !errors.is_empty() {
            for error in &errors {
                tracing::error!(error = ?error, bot = bot.id, "bot error");
            }
            return Err(anyhow!("Bot contains errors"));
        }
        Ok(())
    })?;

    let bots = Bots::new(bots);
    Ok(bots)
}

async fn upload_documents(
    documents: DocumentCollection,
    llm_config: LlmConfig,
    seaorm_pool: DatabaseConnection,
    file_loader: Loader,
) -> Result<()> {
    let retriever = PgVector::new(&llm_config, &seaorm_pool);

    for (file_id, document) in documents.documents {
        tracing::info!(document = ?document.file, "Uploading document");
        let file_metadata = document.file_metadata;

        let DocumentMetadata { name, link } = document.metadata;

        let loader_to_move = file_loader.clone();

        let load_file: RagDocumentLoaderFn =
            Box::new(|| async move { loader_to_move.load_file(document.file.clone()).await }.boxed());

        let pgvector_document: Option<PgVectorDocument> = match document.r#type {
            DocumentType::Slides => Some(PgVectorDocument::Slides(
                hikari_core::pgvector::documents::slides::SlidesDocument {
                    id: file_id.clone(),
                    load_fn: Some(load_file),
                    exclude: document.exclude,
                    loaded_file: None,
                    name,
                    link,
                },
            )),
            DocumentType::Text | DocumentType::Book | DocumentType::Paper => Some(PgVectorDocument::Text(
                hikari_core::pgvector::documents::text::TextDocument {
                    id: file_id.clone(),
                    load_fn: Some(load_file),
                    exclude: document.exclude,
                    loaded_file: None,
                    name,
                    link,
                },
            )),
        };
        if let Some(pgvector_document) = pgvector_document {
            upload_document(&retriever, pgvector_document, file_metadata).await?;
        }
        tokio::task::yield_now().await;
    }
    tracing::info!("All rag documents uploaded successfully");
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

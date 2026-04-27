use crate::data;
use crate::data::csml::Bots;
use crate::data::modules;
use anyhow::anyhow;
use csml_interpreter::data::CsmlResult;
use csml_interpreter::validate_bot;
use futures_util::FutureExt;
use hikari_config::assessment::AssessmentConfig;
use hikari_config::constants::collection::ConstantCollection;
use hikari_config::documents::collection::DocumentCollection;
use hikari_config::documents::document::{DocumentMetadata, DocumentType};
use hikari_config::global::GlobalConfig;
use hikari_config::module::ModuleConfig;
use hikari_core::llm_config::LlmConfig;
use hikari_core::pgvector::documents::{ChunkKind, PgVectorDocument, RagDocumentLoaderFn};
use hikari_core::pgvector::error::PgVectorError;
use hikari_core::pgvector::{PgVector, upload_document};
use hikari_llm::builder::LlmStructureConfig;
use hikari_utils::loader::error::LoadingError;
use hikari_utils::loader::{Loader, LoaderHandler, LoaderTrait};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use sea_orm::DatabaseConnection;
use tokio::sync::{Mutex, OnceCell};
use tracing::instrument;
use url::Url;

#[instrument(skip_all)]
pub async fn upload_documents(
    documents: DocumentCollection,
    llm_config: LlmConfig,
    seaorm_pool: DatabaseConnection,
    file_loader: Loader,
) -> Result<(), PgVectorError> {
    let retriever = PgVector::new(&llm_config, &seaorm_pool);

    for (file_id, document) in documents.documents {
        tracing::info!(document = ?document.file, "Uploading document");
        let file_metadata = document.file_metadata;
        let kind = match document.r#type {
            DocumentType::Slides => ChunkKind::Slides,
            DocumentType::Text | DocumentType::Book | DocumentType::Paper => ChunkKind::Text,
        };
        let exclude = document.exclude;
        let file = document.file;
        let DocumentMetadata { name, link } = document.metadata;

        let loader_to_move = file_loader.clone();

        let load_file: RagDocumentLoaderFn = Box::new(|| async move { loader_to_move.load_file(file).await }.boxed());

        let pgvector_document = PgVectorDocument {
            id: file_id.clone(),
            exclude,
            load_fn: Mutex::new(Some(load_file)),
            loaded_file: OnceCell::new(),
            name,
            link,
            kind,
        };
        upload_document(&retriever, pgvector_document, file_metadata).await?;
        tokio::task::yield_now().await;
    }
    tracing::info!("All rag documents uploaded successfully");
    Ok(())
}

pub async fn load_config(global_cfg: Option<&Url>, loader_handler: &LoaderHandler) -> anyhow::Result<GlobalConfig> {
    let Some(url) = global_cfg else {
        return Ok(GlobalConfig::default());
    };
    let config = hikari_config::global::load(loader_handler.loader(url)?).await?;
    Ok(config)
}

pub async fn load_assessments(
    assessment_url: Option<&Url>,
    loader_handler: &LoaderHandler,
) -> anyhow::Result<AssessmentConfig> {
    let Some(path) = assessment_url else {
        return Ok(AssessmentConfig::default());
    };
    let assessments = hikari_config::assessment::load(loader_handler.loader(path)?).await?;
    Ok(assessments)
}

pub async fn load_modules(
    config_url: &Url,
    loader_handler: &LoaderHandler,
    config: &GlobalConfig,
    llm_rag_documents: &DocumentCollection,
) -> anyhow::Result<ModuleConfig> {
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

pub async fn load_llm_structures(
    structured_url: &Url,
    loader_handler: &LoaderHandler,
) -> Result<LlmStructureConfig, LoadingError> {
    let llm_structures = hikari_llm::builder::load(loader_handler.loader(structured_url)?).await?;
    Ok(llm_structures)
}

pub async fn load_documents(
    documents_url: &Url,
    loader_handler: &LoaderHandler,
) -> Result<DocumentCollection, LoadingError> {
    let documents = hikari_config::documents::load(loader_handler.loader(documents_url)?).await?;
    Ok(documents)
}

pub async fn load_constants(
    constants_url: Option<&Url>,
    loader_handler: &LoaderHandler,
) -> Result<ConstantCollection, LoadingError> {
    let constants = if let Some(path_or_url) = constants_url {
        hikari_config::constants::load(loader_handler.loader(path_or_url)?).await?
    } else {
        ConstantCollection::default()
    };
    Ok(constants)
}

pub async fn load_bots(csml_url: &Url, worker_url: &Url, loader_handler: &LoaderHandler) -> anyhow::Result<Bots> {
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

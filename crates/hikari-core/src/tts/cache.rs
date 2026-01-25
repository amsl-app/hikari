use hikari_utils::loader::{LoaderHandler, LoaderTrait};
use sea_orm::DatabaseConnection;
use url::Url;
use xxhash_rust::xxh3::xxh3_64;

use crate::tts::{config::TTSConfig, error::TTSError};

const FOLDER: &str = "v0";

fn hash_string(content: &str) -> String {
    let hash = xxh3_64(content.as_bytes());
    hex::encode(hash.to_le_bytes())
}

pub(crate) async fn get_speech(
    db: &DatabaseConnection,
    config: &TTSConfig,
    text: &str,
) -> Result<Option<Vec<u8>>, TTSError> {
    match &config.cache_config {
        Some(cache_config) => {
            let text_hash = hash_string(text);
            let cache_loader_handler = LoaderHandler::new(Some(cache_config.clone()));
            let cache_url = Url::parse("s3://amsl-audio")?;
            let cache_loader = cache_loader_handler.loader(&cache_url)?;

            let cache = hikari_db::llm::tts::Query::get_path(db, &text_hash).await?;

            match cache {
                Some(path) => {
                    let file = cache_loader.load_file(path).await?;
                    Ok(Some(file.content))
                }
                None => Ok(None),
            }
        }
        None => {
            tracing::warn!("Could not load cache, no cache loader provided");
            Ok(None)
        }
    }
}

pub(crate) async fn cache_speech(
    db: &DatabaseConnection,
    config: &TTSConfig,
    pcm_audio: &[u8],
    text: &str,
) -> Result<(), TTSError> {
    let text_hash = hash_string(text);

    match &config.cache_config {
        Some(cache_config) => {
            let cache_loader_handler = LoaderHandler::new(Some(cache_config.clone()));
            let cache_url = Url::parse("s3://amsl-audio")?;
            let cache_loader = cache_loader_handler.loader(&cache_url)?;
            let path = hikari_db::llm::tts::Query::get_path(db, &text_hash).await?;
            if path.is_some() {
                return Ok(());
            }
            let path = format!("{FOLDER}/{text_hash}.wav");
            cache_loader.store_file(path.clone(), pcm_audio).await?;
            hikari_db::llm::tts::Mutation::insert_path(db, &text_hash, &path).await?;
        }
        None => {
            tracing::warn!("Could not load cache, no cache loader provided");
        }
    }
    Ok(())
}

use csml_engine::data::models::CsmlRequest;
use csml_interpreter::data::{CsmlBot, CsmlFlow};
use csml_interpreter::load_components;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::DirEntry;
use std::path::{Path, PathBuf};
use std::{fs, io};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use crate::error::BotError;

pub fn init_bot(name: String, flows: Vec<CsmlFlow>, endpoint: Option<String>) -> Result<CsmlBot, BotError> {
    let default_flow = flows.first().ok_or(BotError::Empty)?.name.clone();

    Ok(CsmlBot {
        id: name.clone(),
        name,
        apps_endpoint: endpoint,
        flows,
        // TODO (MED) Components are loaded in start_conversation
        //  For us it would probably make sense to write a custom start_conversation that
        //  Tries to use the native_components that are configured here and only fall back
        //  to the default handling if needed. This way we won't have to ship a components folder.
        //  In addition we can persist the components with the bot so when they change we don't
        //  run into issues with components suddenly not existing.
        native_components: Some(load_components()?),
        custom_components: None,
        default_flow,
        bot_ast: None,
        no_interruption_delay: None,
        env: None,
        modules: None,
        multibot: None,
    })
}

pub async fn load_flows_from_dir<P: AsRef<Path>>(dir: &P) -> Result<Vec<CsmlFlow>, BotError> {
    let entries = fs::read_dir(dir)?;
    let flows = futures::future::try_join_all(
        entries
            .into_iter()
            .filter_map(is_csml_file)
            .map(|path| async move { load_flow(&path).await }),
    )
    .await?;

    Ok(flows)
}

pub async fn load_flow(path: &Path) -> Result<CsmlFlow, BotError> {
    let flow_name = path
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or(BotError::FileNameConversion)?;

    tracing::debug!(flow_name, ?path, "loading flow");

    let mut content = String::new();
    File::open(path).await?.read_to_string(&mut content).await?;

    // TODO (LOW) hash the content to use as id when we save them to the database
    // let _hash = sha256::digest(content.as_str());

    Ok(CsmlFlow {
        id: flow_name.to_owned(),
        name: flow_name.to_owned(),
        content,
        commands: vec![],
    })
}

fn is_csml_file(dir: io::Result<DirEntry>) -> Option<PathBuf> {
    if let Ok(dir) = dir {
        let file = dir.path().canonicalize();
        match file {
            Err(error) => tracing::error!(error = &error as &dyn Error, "could not read file"),
            Ok(path) => {
                if path.is_file() && path.extension().is_some_and(|ext| ext.eq("csml")) {
                    return Some(path);
                }
            }
        }
    }
    None
}

#[must_use]
pub fn init_request(
    client: csml_engine::Client,
    payload: serde_json::Value,
    metadata: serde_json::Value,
) -> CsmlRequest {
    CsmlRequest {
        request_id: Uuid::new_v4().as_hyphenated().to_string(),
        client,
        callback_url: None, //Some("http://httpbin.org/post".to_owned()),
        payload,
        metadata,
        ttl_duration: None,
        step_limit: None,
        low_data_mode: None,
    }
}

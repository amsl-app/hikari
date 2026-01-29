use csml_interpreter::data::{CsmlBot, CsmlFlow};
use futures::StreamExt;
use hikari_common::csml_utils::init_bot;
use hikari_utils::loader::error::LoadingError;
use hikari_utils::loader::{Filter, Loader, LoaderTrait};
use std::collections::HashMap;
use std::path::Path;

pub(crate) const CHANNEL_PREFIX: &str = "hikari";
pub(crate) fn generate_channel_name(module: &str, session: &str) -> String {
    format!("{CHANNEL_PREFIX}__{module}__{session}")
}

pub async fn load_bots(loader: Loader, endpoint: Option<&str>) -> Result<Vec<CsmlBot>, LoadingError> {
    tracing::debug!("loading bots");
    let mut flows = HashMap::new();
    let mut stream = loader.load_dir("", Filter::Csml);
    while let Some(Ok(file)) = stream.next().await {
        let (bot_name, flow_name) = bot_flow_from_path(&file.metadata.key)?;
        let flow = CsmlFlow {
            id: flow_name.to_string(),
            name: flow_name.to_string(),
            content: String::from_utf8(file.content)?,
            commands: vec![],
        };
        flows.entry(bot_name.to_string()).or_insert_with(Vec::new).push(flow);
    }

    let bots = futures::future::try_join_all(
        flows
            .into_iter()
            .map(|(name, flows)| async move { init_bot(name, flows, endpoint.map(ToOwned::to_owned)) }),
    )
    .await
    .map_err(|err| LoadingError::Undefined(err.to_string()))?;
    tracing::debug!(bot_count = bots.len(), "bots loaded");
    Ok(bots)
}

fn bot_flow_from_path<P: AsRef<Path>>(path: &P) -> Result<(&str, &str), LoadingError> {
    let path: &Path = path.as_ref();

    if let Some(dir_name) = path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str())
        && let Some(file_name) = path.file_stem().and_then(|s| s.to_str())
    {
        return Ok((dir_name, file_name));
    }
    Err(LoadingError::Undefined(
        "Could not get bot and flow name from path".to_owned(),
    ))
}

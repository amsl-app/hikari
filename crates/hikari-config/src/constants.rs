use futures::StreamExt;
use hikari_utils::loader::error::LoadingError;
use hikari_utils::loader::{Filter, Loader, LoaderTrait};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::constants::collection::ConstantCollection;

pub mod collection;
pub mod constant;
pub mod v01;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[serde(tag = "version")]
pub enum VersionConfig {
    #[serde(rename = "0.1")]
    V01 {
        #[serde(flatten)]
        constants: v01::collection::ConstantCollectionV01,
    },
}

pub async fn load(loader: Loader) -> Result<ConstantCollection, LoadingError> {
    tracing::debug!("loading constants");
    let mut stream = loader.load_dir("", Filter::Yaml);
    let mut all_constants = ConstantCollection::default();

    while let Some(Ok(file)) = stream.next().await {
        let VersionConfig::V01 { constants } = serde_yml::from_slice::<VersionConfig>(&file.content)?;
        let constant_collection: ConstantCollection = constants.into();
        all_constants.constants.extend(constant_collection.constants);
    }
    Ok(all_constants)
}

#[cfg(test)]
mod tests {
    use std::fs::read_to_string;

    use super::*;

    #[test]
    fn test_constants_loading() {
        let constants_file = read_to_string("test_configs/test.constants.yaml").unwrap();
        let VersionConfig::V01 { constants } = serde_yml::from_str::<VersionConfig>(&constants_file).unwrap();
        assert_eq!(constants.constants.len(), 1);
    }
}

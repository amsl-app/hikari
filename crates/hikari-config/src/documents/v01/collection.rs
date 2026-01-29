use schemars::JsonSchema;
use serde::Deserialize;

use crate::documents::v01::document::DocumentConfigV01;

#[derive(Default, Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct DocumentCollectionV01 {
    pub documents: Vec<DocumentConfigV01>,
}

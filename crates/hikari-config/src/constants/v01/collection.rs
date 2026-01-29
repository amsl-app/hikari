use schemars::JsonSchema;
use serde::Deserialize;

use crate::constants::v01::constant::ConstantV01;

#[derive(Default, Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ConstantCollectionV01 {
    pub constants: Vec<ConstantV01>,
}

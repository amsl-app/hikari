use schemars::JsonSchema;
use serde::Deserialize;
use serde_yml::Value;

#[derive(Default, Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Constant {
    /// # The name of the constant.
    pub name: String,
    #[schemars(with = "serde_json::Value")]
    /// # The value of the constant.
    pub value: Value,
}

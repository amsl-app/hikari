use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Deserialize, Clone, Debug, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct AccessConfig {
    /// # A token which the user can use to access new functions and modules
    pub token: String,
    #[serde(default)]
    /// # Approval which must be given by the user to access the system
    pub approvals: Option<AccessApproval>,
    /// # Groups which are added to the user when he adds the token
    pub groups: Vec<GroupAccess>,
}

#[derive(Deserialize, Clone, Debug, Serialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct AccessApproval {
    #[schemars(with = "String")]
    pub declaration_of_consent: Url,
    #[schemars(with = "String")]
    pub privacy_policy: Url,
    #[schemars(with = "Option<String>")]
    pub participant_information: Option<Url>,
}

#[derive(Deserialize, Clone, Debug, JsonSchema)]
#[serde(untagged)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum GroupAccess {
    /// # Single group to add
    Single(String),
    /// # One of multiple groups to add (randomly chosen)
    Random { random: Vec<String> },
}

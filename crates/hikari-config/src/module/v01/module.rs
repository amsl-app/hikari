use std::collections::{HashMap, HashSet};

use crate::{
    generic::{Metadata, Theme},
    module::{
        ModuleCategory,
        v01::{assessment::ModuleAssessmentV01, content::ContentV01, feature::FeatureV01, session::SessionV01},
    },
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_with::{SetPreventDuplicates, serde_as};

#[serde_as]
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ModuleV01 {
    /// # Unique identifier of the module
    pub(crate) id: String,
    /// # Title of the module
    pub(crate) title: String,
    /// # Subtitle of the module
    pub(crate) subtitle: Option<String>,
    /// # Description of the module
    pub(crate) description: Option<String>,
    /// # Icon associated with the module
    pub(crate) icon: Option<String>,
    /// # Banner image associated with the module
    pub(crate) banner: Option<String>,
    #[serde(rename = "default-session")]
    /// # Default session of the module
    pub(crate) default_session: Option<String>,
    /// # Sessions available in the module
    /// A session is a chatbot conversation covering a specific topic within the module
    pub(crate) sessions: Vec<SessionV01>,
    #[serde(default, rename = "self-learning")]
    /// # Self-learning feature of the module
    /// Self learning is a additional session which covers every unlocked content in the module
    /// Self learning sessions are Q&A style sessions in which the user can ask questions about the content
    pub(crate) self_learning: FeatureV01,
    #[serde(default)]
    /// # Whether the module is quizzable
    /// Quizzable modules can be used inside a quiz tool.
    /// Quizzes simulate an exam about the module content.
    pub(crate) quizzable: bool,
    #[serde(default)]
    /// # Whether the module is hidden from the frontend
    pub(crate) hidden: bool,
    #[serde(default)]
    /// # Contents available in the module
    pub(crate) contents: Vec<ContentV01>,
    /// # Theme of the module
    pub(crate) theme: Option<Theme>,
    /// # Assessment configuration of the module
    /// Assessment which is used to evaluate the user before and after finishing the module
    /// Most in cases of courses instead of lectures, since lectures can have quizzes
    pub(crate) assessment: Option<ModuleAssessmentV01>,
    /// # Weight of the module
    /// Used to order modules in the frontend. Higher weight means higher up in the list
    pub(crate) weight: Option<usize>,
    #[serde(default)]
    /// # Category of the module
    /// Default ist "learning" for lecture content or "course" for structured courses with a dedicated order and without quizzable content & self-learning
    pub(crate) category: ModuleCategory,
    #[serde_as(as = "SetPreventDuplicates<_>")]
    #[schemars(with = "Vec::<String>")]
    /// # Module groups the module belongs to
    /// Module groups are used to group modules for display purposes in the frontend
    pub(crate) module_groups: HashSet<String>,
    pub(crate) metadata: Option<Metadata>,
    #[serde(rename = "groups-whitelist", alias = "required-permissions", default)]
    /// # Groups required to access the module
    /// If the user does not have at least one of the groups in the whitelist, the module is not accessible
    pub(crate) groups_whitelist: Vec<String>,
    #[serde(rename = "groups-blacklist", default)]
    /// # Groups which are not allowed to access the module
    /// If the user has at least one of the groups in the blacklist, the module is not accessible
    pub(crate) groups_blacklist: Vec<String>,
    #[schemars(with = "Option<HashMap<String, serde_json::Value>>")]
    pub(crate) custom: Option<HashMap<String, serde_yml::Value>>,
}

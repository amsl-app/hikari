use hikari_utils::id_map::ItemId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Item {
    /// # Unique identifier of a question
    /// This ID is used to reference the question within the scale.
    pub id: String,
    #[serde(default)]
    /// # Reverse scoring flag
    /// Indicates whether the scoring for this item should be reversed.
    pub reverse: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum Mode {
    Sum,
    Average,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Scale {
    /// # Unique identifier for the scale
    /// This ID is used to reference the scale within the assessment.
    pub id: String,
    /// # Title of the scale
    /// A human-readable title for the scale.
    pub title: String,
    #[serde(flatten)]
    /// # Body of the scale
    /// Defines the characteristics and behavior of the scale.
    pub body: ScaleBody,
    /// # Mode of the scale
    /// Determines how the scale's scores are calculated.
    pub mode: Mode,
    /// # Items in the scale
    /// A list of items that make up the scale.
    pub items: Vec<Item>,
}

impl ItemId for Scale {
    type IdType = String;

    fn id(&self) -> Self::IdType {
        self.id.clone()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, JsonSchema)]
#[serde(tag = "type", content = "body")]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum ScaleBody {
    Scale { min: u32, max: u32 },
}

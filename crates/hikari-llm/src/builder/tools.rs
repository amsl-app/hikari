use crate::builder::slot::SlotValuePair;
use crate::builder::slot::paths::SlotPath;
use crate::builder::steps::extractor::ExtractionValues;
use crate::builder::steps::validator::ConversationGoal;
use crate::builder::steps::{InjectionTrait, SlotsTrait};
use crate::execution::tools::extractor::ExtractionTool;
use crate::execution::tools::summarizer::SummarizerTool;
use crate::execution::tools::validation::ValidationTool;

#[derive(Clone)]
pub enum Tool {
    ValidationTool(Vec<ConversationGoal>),
    ExtractionTool(Vec<ExtractionValues>),
    Summarizer,
}

impl SlotsTrait for Tool {
    fn injection_slots(&self) -> Vec<SlotPath> {
        match self {
            Tool::ValidationTool(goals) => goals
                .iter()
                .flat_map(super::steps::SlotsTrait::injection_slots)
                .collect(),
            Tool::ExtractionTool(values) => values
                .iter()
                .flat_map(super::steps::SlotsTrait::injection_slots)
                .collect(),
            Tool::Summarizer => vec![],
        }
    }
}

impl InjectionTrait for Tool {
    fn inject(&self, values: &[SlotValuePair]) -> Self {
        match self {
            Tool::ValidationTool(goals) => Tool::ValidationTool(goals.iter().map(|g| g.inject(values)).collect()),
            Tool::ExtractionTool(extractions) => {
                Tool::ExtractionTool(extractions.iter().map(|e| e.inject(values)).collect())
            }
            Tool::Summarizer => Tool::Summarizer,
        }
    }
}

impl Tool {
    #[must_use]
    pub fn to_langchain_tool(&self) -> Box<dyn hikari_core::openai::tools::Tool> {
        match self {
            Tool::ValidationTool(goals) => Box::new(ValidationTool::new(goals.clone())),
            Tool::ExtractionTool(values) => Box::new(ExtractionTool::new(values.clone())),
            Tool::Summarizer => Box::new(SummarizerTool::new()),
        }
    }
}

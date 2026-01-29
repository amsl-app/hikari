use std::collections::HashMap;

use crate::builder::steps::validator::ConversationGoal;
use hikari_core::openai::tools::Tool;
use sea_orm::prelude::async_trait::async_trait;
use serde_json::{Value, json};

pub struct ValidationTool {
    goals: Vec<ConversationGoal>,
}

impl ValidationTool {
    pub fn new(goals: Vec<ConversationGoal>) -> Self {
        Self { goals }
    }
}
#[async_trait]

impl Tool for ValidationTool {
    fn name(&self) -> &'static str {
        "ValidationTool"
    }

    fn description(&self) -> &'static str {
        "This tool is used to validate the conversation. Always use this tool when you need to validate a chat. \
        The function receives the input whether the defined goals were achieved or not."
    }

    fn parameters(&self) -> Value {
        let mut map: HashMap<String, Value> = HashMap::new();
        for output in &self.goals {
            let output: HashMap<String, Value> = output.clone().into();
            map.extend(output);
        }

        json!({
            "type": "object",
            "properties": map,
            "required": map.keys().collect::<Vec<&String>>()
        })
    }
}

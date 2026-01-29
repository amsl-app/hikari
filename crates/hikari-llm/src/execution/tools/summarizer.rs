use hikari_core::openai::tools::{OpenApiField, Tool};
use sea_orm::prelude::async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

pub struct SummarizerTool {}

impl SummarizerTool {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Tool for SummarizerTool {
    fn name(&self) -> &'static str {
        "SummarizingTool"
    }

    fn description(&self) -> &'static str {
        "This tool processes and stores the conversation summary. Always use this tool when you need to create a summary for the conversation. \
        The function receives the summary as input"
    }

    fn parameters(&self) -> Value {
        let field = OpenApiField::object()
            .properties(HashMap::from([("summary", OpenApiField::new("string"))]))
            .required(vec!["summary"]);

        serde_json::to_value(field).expect("Serialization failed that should not fail")
    }
}

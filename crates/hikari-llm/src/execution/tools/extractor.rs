use crate::builder::steps::extractor::ExtractionValues;
use hikari_core::openai::tools::{AsOpenApiField, OpenApiField, Tool};
use sea_orm::prelude::async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

pub struct ExtractionTool {
    outputs: Vec<ExtractionValues>,
}

impl ExtractionTool {
    pub fn new(outputs: Vec<ExtractionValues>) -> Self {
        Self { outputs }
    }
}

#[async_trait]
impl Tool for ExtractionTool {
    fn name(&self) -> &'static str {
        "ExtractionTool"
    }

    fn description(&self) -> &'static str {
        "This tool uses information from a conversation. Always use this tool when you need to extract information from a conversation. \
        The function receives the values that could be extracted as input"
    }

    fn parameters(&self) -> Value {
        let map: HashMap<_, _> = self
            .outputs
            .iter()
            .map(|output| (output.target_identifier(), output.openapi_field()))
            .collect();
        let field = OpenApiField::object().properties(map);

        serde_json::to_value(field).expect("Serialization failed that should not fail")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::slot::SaveTarget;
    use crate::builder::slot::paths::{Destination, SlotPath};
    use crate::builder::steps::extractor::ExtractionSchema;

    #[test]
    fn test_parameters() {
        let outputs = vec![
            ExtractionValues {
                schema: ExtractionSchema {
                    description: "extraction_value_1".into(),
                    examples: vec![],
                    r#enum: None,
                    r#items: None,
                    r#properties: None,
                    r#type: "number".to_string(),
                },
                target: SaveTarget::Slot(SlotPath::new("slot_name".to_string(), Destination::default())),
            },
            ExtractionValues {
                schema: ExtractionSchema {
                    description: "extraction_value_2".into(),
                    examples: vec![],
                    r#enum: None,
                    r#items: None,
                    r#properties: None,
                    r#type: "string".to_string(),
                },
                target: SaveTarget::Slot(SlotPath::new("a.b.c".to_string(), Destination::default())),
            },
            ExtractionValues {
                schema: ExtractionSchema {
                    description: "extraction_value_2".into(),
                    examples: vec![],
                    r#enum: Some(vec!["value1".to_string(), "value2".to_string()]),
                    r#items: None,
                    r#properties: None,
                    r#type: "string".to_string(),
                },
                target: SaveTarget::Slot(SlotPath::new("enum_slot".to_string(), Destination::default())),
            },
        ];

        let tool = ExtractionTool::new(outputs);
        let parameters = tool.parameters();

        assert_eq!(parameters["type"], "object");
        assert_eq!(parameters["properties"]["slot_name"]["type"], "number");
        assert_eq!(parameters["properties"]["a.b.c"]["type"], "string");
        assert_eq!(
            parameters["properties"]["enum_slot"]["enum"],
            Value::Array(vec![
                Value::String("value1".to_string()),
                Value::String("value2".to_string()),
            ])
        );
    }
}

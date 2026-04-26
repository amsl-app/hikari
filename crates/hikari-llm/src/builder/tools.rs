use crate::builder::slot::SlotValuePair;
use crate::builder::slot::paths::SlotPath;
use crate::builder::steps::extractor::ExtractionValues;
use crate::builder::steps::validator::ConversationGoal;
use crate::builder::steps::{InjectionTrait, SlotsTrait};
use hikari_core::openai::tools::{AsOpenApiField, OpenApiField, ToolSchema};
use std::collections::HashMap;

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
    pub fn tool_schema(&self) -> ToolSchema {
        match self {
            Tool::ValidationTool(goals) => {
                let mut properties: HashMap<&str, OpenApiField> = HashMap::new();
                let mut required: Vec<&str> = Vec::new();
                for output in goals {
                    let name = output.name.0.as_str().unwrap_or_default();
                    let goal = output.goal.0.as_str().unwrap_or_default();

                        let examples = if output.examples.is_empty() {
                                "".to_string()
                            } else {
                                let example_strings = output
                                    .examples
                                    .iter()
                                    .map(|e| e.to_string())
                                    .collect::<Vec<_>>()
                                    .join("<sep>");
                                format!("\n<examples>\n{}\n</examples>\n", example_strings)
                            };

                        let value_description = format!("True, wenn das Konversationsziel '''{name}''' erfüllt ist: {goal}{examples}");
                        let explaination_description =
                            "Erkläre, deinen Gedanken, warum du so entschieden hast.".to_string();
                        properties.insert(name, OpenApiField::object().properties(
                            HashMap::from([
                                ("decision", OpenApiField::new("string").description(value_description)),
                                ("explaination", OpenApiField::new("string").description(explaination_description))
                            ])
                        ).required(vec!["decision", "explaination"]));
                        required.push(name);
                }

                OpenApiField::object().title("Validation").description("This tool is used to validate the conversation. Always use this tool when you need to validate a chat. The function receives the input whether the defined goals were achieved or not.").properties(properties).required(required).into()
            },
            Tool::ExtractionTool(values) => {
                let prperties: HashMap<&str, OpenApiField> = values
                                                .iter()
                                                .map(|output| (output.target_identifier(), output.openapi_field()))
                                                .collect();
                                    OpenApiField::object()
                                        .title("Extraction")
                                        .description("This tool uses information from a conversation. Always use this tool when you need to extract information from a conversation. The function receives the values that could be extracted as input")
                                        .properties(
                                            prperties
                                        )
                                        .into()}
            Tool::Summarizer => OpenApiField::object()
                        .title("Summary")
                        .description("This tool processes and stores the conversation summary. Always use this tool when you need to create a summary for the conversation. The function receives the summary as input")
                        .properties(
                            HashMap::from([("summary", OpenApiField::new("string"))]))
                        .required(vec!["summary"]).into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::slot::SaveTarget;
    use crate::builder::slot::paths::{Destination, SlotPath};
    use crate::builder::steps::Template;
    use crate::builder::steps::extractor::ExtractionSchema;
    use async_openai::types::chat::ChatCompletionTools;
    use yaml_serde::Value;

    #[test]
    fn test_extractor() {
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

        let tool = Tool::ExtractionTool(outputs).tool_schema();
        let openai_tool: ChatCompletionTools = tool.try_into().expect("Failed to convert to OpenAI tool");
        let openai_tool = match openai_tool {
            ChatCompletionTools::Function(f) => f.function,
            _ => panic!("Expected Function tool"),
        };

        assert_eq!(openai_tool.name, "Extraction");
        assert_eq!(
            openai_tool.description,
            Some("This tool uses information from a conversation. Always use this tool when you need to extract information from a conversation. The function receives the values that could be extracted as input".to_string())
        );
        let properties = openai_tool.parameters.unwrap()["properties"].clone();
        assert_eq!(properties["slot_name"]["type"], "number");
        assert_eq!(properties["slot_name"]["description"], "extraction_value_1");
        assert_eq!(properties["a.b.c"]["type"], "string");
        assert_eq!(properties["a.b.c"]["description"], "extraction_value_2");
        assert_eq!(properties["enum_slot"]["type"], "string");
        assert_eq!(properties["enum_slot"]["description"], "extraction_value_2");
    }

    #[test]
    fn test_validation_tool() {
        let goals = vec![ConversationGoal {
            name: Template(Value::String("goal_1".to_string())),
            goal: Template(Value::String("First goal description".to_string())),
            examples: vec![Template(Value::String("Example 1".to_string()))],
        }];

        let tool = Tool::ValidationTool(goals).tool_schema();
        let openai_tool: ChatCompletionTools = tool.try_into().expect("Failed to convert to OpenAI tool");
        let openai_tool = match openai_tool {
            ChatCompletionTools::Function(f) => f.function,
            _ => panic!("Expected Function tool"),
        };

        assert_eq!(openai_tool.name, "Validation");
        let properties = openai_tool.parameters.unwrap()["properties"].clone();
        assert!(properties.get("goal_1").is_some());
        assert_eq!(properties["goal_1"]["type"], "object");
        let goal_props = properties["goal_1"]["properties"].clone();
        assert!(
            goal_props["decision"]["description"]
                .as_str()
                .unwrap()
                .contains("First goal description")
        );
        assert!(
            goal_props["decision"]["description"]
                .as_str()
                .unwrap()
                .contains("Example 1")
        );
    }

    #[test]
    fn test_summarizer() {
        let tool = Tool::Summarizer.tool_schema();
        let openai_tool: ChatCompletionTools = tool.try_into().expect("Failed to convert to OpenAI tool");
        let openai_tool = match openai_tool {
            ChatCompletionTools::Function(f) => f.function,
            _ => panic!("Expected Function tool"),
        };

        assert_eq!(openai_tool.name, "Summary");
        let properties = openai_tool.parameters.unwrap()["properties"].clone();
        assert!(properties.get("summary").is_some());
        assert_eq!(properties["summary"]["type"], "string");
    }
}

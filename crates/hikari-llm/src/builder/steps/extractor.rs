use super::llm::PromptType;
use super::{LlmModel, Memory};
use crate::builder::error::LlmBuildingError;
use crate::builder::slot::paths::SlotPath;
use crate::builder::slot::{SaveTarget, SlotValuePair};
use crate::builder::steps::{
    Condition, Documents, Flow, InjectionTrait, IntoLlmStep, ParentStep, SlotsTrait, Template, load_prompt_and_temp,
};
use crate::builder::tools::Tool;
use crate::builder::{build_memory_filter, step_id_from_flow};
use crate::execution::core::LlmCore;
use crate::execution::steps::LlmStep;
use crate::execution::steps::value_extractor::ValueExtractor;
use hikari_core::openai::tools::{AsOpenApiField, OpenApiField};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_yml::Value;
use std::collections::HashMap;

const PROMPT_KEY: &str = "EXTRACTOR_PREFIX";
const TEMPERATURE_KEY: &str = "EXTRACTOR_TEMPERATURE";

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ExtractorBuilder {
    /// # Values to extract from the conversation
    pub values: Vec<ExtractionValues>,
    #[serde(default)]
    pub prompts: Vec<PromptType>,
    #[serde(flatten)]
    pub memory: Memory,
    pub success: Flow,
    pub fail: Flow,
    #[serde(flatten)]
    pub model: LlmModel,
    #[serde(default)]
    pub skip_prefix: bool,
}

impl SlotsTrait for ExtractorBuilder {
    fn injection_slots(&self) -> Vec<SlotPath> {
        let mut slots = self
            .values
            .iter()
            .flat_map(super::SlotsTrait::injection_slots)
            .collect::<Vec<_>>();
        slots.extend(self.prompts.iter().flat_map(super::SlotsTrait::injection_slots));
        slots
    }
}

impl IntoLlmStep for ExtractorBuilder {
    fn into_llm_step(
        mut self,
        parent_steps: Vec<ParentStep>,
        mut conditions: Vec<Condition>,
        id: String,
        constants: HashMap<String, Value>,
        _documents: Documents,
    ) -> Result<LlmStep, LlmBuildingError> {
        self.prompts.iter_mut().for_each(|p| {
            p.insert_constant(&constants);
        });

        // insert_constants must be called before we extract the slots

        let slots: Vec<SlotPath> = self.injection_slots();

        let ExtractorBuilder {
            values,
            mut prompts,
            memory: Memory {
                memory_limit,
                memory: memory_selection,
            },
            success,
            fail,
            model,
            skip_prefix,
        } = self;

        let (prefix, temperature) = load_prompt_and_temp(&constants, PROMPT_KEY, TEMPERATURE_KEY)?;
        if !skip_prefix {
            prompts.insert(0, PromptType::System(prefix.into()));
        }

        let goto_on_success = step_id_from_flow(success, &parent_steps);
        let goto_on_fail = step_id_from_flow(fail, &parent_steps);

        for step in parent_steps {
            conditions.extend(step.conditions);
        }

        let target_slots = values.iter().map(|task| task.target.clone()).collect();
        let memory_filter = build_memory_filter(&memory_selection, &id);

        let core = LlmCore::new(
            prompts,
            model.with_default_temperature(temperature),
            slots,
            memory_filter,
            memory_limit,
            Some(Tool::ExtractionTool(values)),
        );

        let value_extractor = LlmStep::ValueExtractor(ValueExtractor::new(
            id,
            core,
            target_slots,
            goto_on_success,
            goto_on_fail,
            conditions,
        ));
        Ok(value_extractor)
    }
}

fn default_type() -> String {
    "string".to_string()
}
#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ExtractionValues {
    pub target: SaveTarget,
    #[serde(flatten)]
    pub schema: ExtractionSchema,
}

impl ExtractionValues {
    #[must_use]
    pub fn target_identifier(&self) -> &str {
        match &self.target {
            SaveTarget::Slot(SlotPath { name, .. }) => name,
        }
    }
}

impl SlotsTrait for ExtractionValues {
    fn injection_slots(&self) -> Vec<SlotPath> {
        self.schema.injection_slots()
    }
}

impl InjectionTrait for ExtractionValues {
    fn inject(&self, values: &[SlotValuePair]) -> Self {
        ExtractionValues {
            target: self.target.clone(),
            schema: self.schema.inject(values),
        }
    }
}

impl AsOpenApiField<'_> for ExtractionValues {
    fn openapi_field(&self) -> OpenApiField<'_> {
        self.schema.openapi_field()
    }
}

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ExtractionSchema {
    /// # Description of the value to extract
    pub description: Template,
    #[serde(default)]
    /// # Examples for few shot prompting
    pub examples: Vec<Template>,
    #[serde(rename = "type", default = "default_type")]
    pub r#type: String,
    #[serde(default)]
    pub r#items: Option<Box<ExtractionSchema>>,
    #[serde(default)]
    pub r#properties: Option<HashMap<String, ExtractionSchema>>,
    #[serde(default)]
    pub r#enum: Option<Vec<String>>,
}

impl SlotsTrait for ExtractionSchema {
    fn injection_slots(&self) -> Vec<SlotPath> {
        let mut slots = self.description.injection_slots();
        slots.extend(self.examples.iter().flat_map(super::SlotsTrait::injection_slots));
        slots.extend(self.r#items.as_ref().map_or(vec![], |item| item.injection_slots()));
        slots.extend(self.r#properties.as_ref().map_or(vec![], |props| {
            props.values().flat_map(super::SlotsTrait::injection_slots).collect()
        }));
        slots
    }
}

impl InjectionTrait for ExtractionSchema {
    fn inject(&self, values: &[SlotValuePair]) -> Self {
        ExtractionSchema {
            description: self.description.inject(values),
            examples: self.examples.iter().map(|e| e.inject(values)).collect(),
            r#items: self.r#items.as_ref().map(|item| Box::new(item.inject(values))),
            r#properties: self
                .r#properties
                .as_ref()
                .map(|props| props.iter().map(|(k, v)| (k.clone(), v.inject(values))).collect()),
            r#type: self.r#type.clone(),
            r#enum: self.r#enum.clone(),
        }
    }
}

impl<'a> AsOpenApiField<'a> for ExtractionSchema {
    fn openapi_field(&'a self) -> OpenApiField<'a> {
        let description = format!(
            "{} (e.g.:\n{})",
            self.description,
            self.examples
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<String>>()
                .join("\n")
        );
        let mut field = OpenApiField::new(self.r#type.as_str()).description(description);
        if let Some(items) = &self.r#items {
            field = field.items(items.openapi_field());
        }

        if let Some(props) = &self.r#properties {
            let properties = props
                .iter()
                .map(|(k, v)| (k.as_str(), v.openapi_field()))
                .collect::<HashMap<&'a str, OpenApiField<'a>>>();
            field = field.properties(properties);
        }

        if let Some(r#enum) = &self.r#enum {
            field = field.r#enum(r#enum.iter().map(String::as_str).collect::<Vec<_>>());
        }

        field
    }
}

#[cfg(test)]
mod tests {
    use crate::builder::slot::paths::Destination;

    use super::*;

    #[test]
    fn test_extraction_value_openapi() {
        let values = ExtractionValues {
            schema: ExtractionSchema {
                description: "extraction_value".into(),
                r#type: "string".into(),
                r#enum: None,
                r#items: None,
                r#properties: None,
                examples: vec!["Beispiel 1".into(), "Beispiel 2".into()],
            },
            target: SaveTarget::Slot(SlotPath::new("slot_target".into(), Destination::Conversation)),
        };

        let field = values.openapi_field();

        assert_eq!(field.r#type, "string");
        assert_eq!(
            &field.description.unwrap(),
            "extraction_value (e.g.:\nBeispiel 1\nBeispiel 2)"
        );
    }
}

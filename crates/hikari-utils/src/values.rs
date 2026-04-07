use serde_json_path::JsonPath;
use serde_yml::Value;
use std::panic;

use crate::values::error::ValuesError;

pub mod error;

pub trait QueryYaml {
    fn query(&self, path: &str) -> Result<Value, ValuesError>;
}

impl QueryYaml for Value {
    fn query(&self, path: &str) -> Result<Value, ValuesError> {
        let json_value = serde_json::to_value(self)?;
        let value = json_value.query(path)?;
        let yaml_value = serde_yml::to_value(&value)?;
        Ok(yaml_value)
    }
}

pub trait QueryJson {
    fn query(&self, path: &str) -> Result<serde_json::Value, ValuesError>;
}

impl QueryJson for serde_json::Value {
    fn query(&self, path: &str) -> Result<serde_json::Value, ValuesError> {
        let jsonpath = JsonPath::parse(path)?;
        let nodes = jsonpath.query(self).all();
        if nodes.is_empty() {
            Ok(serde_json::Value::Null)
        } else if let &[node] = nodes.as_slice() {
            Ok(node.clone())
        } else {
            let values = serde_json::to_value(&nodes)?;
            Ok(values)
        }
    }
}

pub trait ValueDecoder {
    fn decode(str: &str) -> Value;
    fn encode(&self) -> String;
}

impl ValueDecoder for Value {
    fn decode(str: &str) -> Value {
        // FIX: Sometimes the value cannot be parsed and panics (e.g. bad indentation or incomplete structure)
        if let Ok(parsed) =
            panic::catch_unwind(|| serde_yml::from_str(str).unwrap_or_else(|_| panic!("failed to decode value: {str}")))
        {
            parsed
        } else {
            Value::String(str.to_string())
        }
    }

    fn encode(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            other => serde_yml::to_string(other).unwrap_or_default(),
        }
    }
}

pub trait JsonToYaml {
    fn to_yaml(&self) -> Result<Value, ValuesError>;
    fn to_yaml_string(&self) -> Result<String, ValuesError> {
        let yaml_value = self.to_yaml()?;
        let yaml_string = yaml_value.encode();
        Ok(yaml_string)
    }
}

impl JsonToYaml for serde_json::Value {
    fn to_yaml(&self) -> Result<Value, ValuesError> {
        // Convert Json Value to Yaml Value
        let yaml_value: Value = serde_yml::to_value(self.clone())?;
        Ok(yaml_value)
    }
}

pub trait YamlToJson {
    fn to_json(&self) -> Result<serde_json::Value, ValuesError>;

    fn to_json_string(&self) -> Result<String, ValuesError> {
        let json_value = self.to_json()?;
        let json_string = serde_json::to_string(&json_value)?;
        Ok(json_string)
    }
}

impl YamlToJson for Value {
    fn to_json(&self) -> Result<serde_json::Value, ValuesError> {
        // Convert Yaml Value to Json Value
        let json_value: serde_json::Value = serde_json::to_value(self.clone())?;
        Ok(json_value)
    }
}

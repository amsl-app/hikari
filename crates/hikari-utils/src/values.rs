use serde_json_path::JsonPath;
use yaml_serde::Value;

use crate::values::error::ValuesError;

pub mod error;

pub trait QueryYaml {
    fn query(&self, path: &str) -> Result<Value, ValuesError>;
}

impl QueryYaml for Value {
    fn query(&self, path: &str) -> Result<Value, ValuesError> {
        let json_value = serde_json::to_value(self)?;
        let value = json_value.query(path)?;
        let yaml_value = yaml_serde::to_value(&value)?;
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
        tracing::debug!(?str, "Attempting to decode YAML value");
        if let Ok(parsed) = yaml_serde::from_str(str) {
            tracing::debug!("Successfully decoded YAML value");
            parsed
        } else {
            tracing::warn!(
                ?str,
                "Failed to decode YAML value, falling back to string representation"
            );
            Value::String(str.to_string())
        }
    }

    fn encode(&self) -> String {
        tracing::debug!("Attempting to encode value to string");
        match self {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            other => match yaml_serde::to_string(other) {
                Ok(encoded) => {
                    tracing::debug!("Successfully encoded YAML value");
                    encoded
                }
                Err(err) => {
                    tracing::warn!(?err, "Failed to encode YAML value, returning empty string");
                    String::new()
                }
            },
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
        let yaml_value: Value = yaml_serde::to_value(self.clone())?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use yaml_serde::Value;

    #[test]
    fn test_decode_valid_yaml() {
        let yaml_str = "key: value";
        let val = Value::decode(yaml_str);
        assert!(matches!(val, Value::Mapping(_)), "Expected mapping, got {:?}", val);

        let yaml_list = "- item1\n- item2";
        let val_list = Value::decode(yaml_list);
        assert!(
            matches!(val_list, Value::Sequence(_)),
            "Expected sequence, got {:?}",
            val_list
        );

        let yaml_num = "42";
        let val_num = Value::decode(yaml_num);
        assert!(
            matches!(val_num, Value::Number(_)),
            "Expected number, got {:?}",
            val_num
        );

        let yaml_bool = "true";
        let val_bool = Value::decode(yaml_bool);
        assert!(matches!(val_bool, Value::Bool(_)), "Expected bool, got {:?}", val_bool);
    }

    #[test]
    fn test_decode_valid_json() {
        let json_str = r#"{"key": "value"}"#;
        let val = Value::decode(json_str);
        assert!(matches!(val, Value::Mapping(_)), "Expected mapping, got {:?}", val);

        let json_list = r#"["item1", "item2"]"#;
        let val_list = Value::decode(json_list);
        assert!(
            matches!(val_list, Value::Sequence(_)),
            "Expected sequence, got {:?}",
            val_list
        );
    }

    #[test]
    fn test_decode_invalid_yaml_json_fallback() {
        // Invalid YAML / JSON structures that should trigger the fallback to string
        let invalid_yaml = "[1, 2";
        let val = Value::decode(invalid_yaml);
        assert_eq!(val, Value::String("[1, 2".to_string()));

        let invalid_json = r#"{"key": "value""#;
        let val = Value::decode(invalid_json);
        assert_eq!(val, Value::String(r#"{"key": "value""#.to_string()));

        let unclosed_quote = "\"unclosed";
        let val = Value::decode(unclosed_quote);
        assert_eq!(val, Value::String("\"unclosed".to_string()));

        // Invalid indentation YAML
        let invalid_indentation = "key:\nvalue";
        let val = Value::decode(invalid_indentation);
        assert_eq!(val, Value::String("key:\nvalue".to_string()));

        let inconsistent_indentation = "key:\n  - item1\n - item2";
        let val = Value::decode(inconsistent_indentation);
        assert_eq!(val, Value::String("key:\n  - item1\n - item2".to_string()));
    }

    #[test]
    fn test_query_yaml_valid_path() {
        let yaml_str = "
a:
  b: 42
  c:
    - 1
    - 2
    - 3
";
        let val = Value::decode(yaml_str);

        let res = val.query("$.a.b").unwrap();
        assert_eq!(res, Value::decode("42"));

        let res_arr = val.query("$.a.c[1]").unwrap();
        assert_eq!(res_arr, Value::decode("2"));
    }

    #[test]
    fn test_query_yaml_invalid_path() {
        let val = Value::decode("key: value");

        let res = val.query("$.[invalid path");
        assert!(res.is_err(), "Expected error for invalid json path");
    }

    #[test]
    fn test_query_arbitraty_yaml() {
        let string_val = Value::String("just a string".to_string());
        let res = string_val.query("$.a").unwrap();
        assert_eq!(res, Value::Null);
        let res_root = string_val.query("$").unwrap();
        assert_eq!(res_root, Value::String("just a string".to_string()));

        let number_val = Value::Number(yaml_serde::Number::from(42));
        let res_num = number_val.query("$.a").unwrap();
        assert_eq!(res_num, Value::Null);
        let res_num_root = number_val.query("$").unwrap();
        assert_eq!(res_num_root, Value::Number(yaml_serde::Number::from(42))); 

        let bool_val = Value::Bool(true);
        let res_bool = bool_val.query("$.a").unwrap();
        assert_eq!(res_bool, Value::Null);
        let res_bool_root = bool_val.query("$").unwrap();
        assert_eq!(res_bool_root, Value::Bool(true)); 
    }

    #[test]
    fn test_query_yaml_not_found() {
        let val = Value::decode("key: value");

        let res = val.query("$.nonexistent").unwrap();
        assert_eq!(res, Value::Null);
    }
}

use chrono::{DateTime, NaiveDateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, Copy, ToSchema)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum LockedUntil {
    Time(DateTime<Utc>),
    Undefined,
}

#[derive(Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[derive(Debug, Clone, Copy)]
pub enum UnlockTriggerWait {
    Days(u32),
    Seconds(u32),
}
#[derive(Serialize, Deserialize, ToSchema, Debug, Clone, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", untagged)]
pub enum UnlockTriggerTimeFormat {
    Local(NaiveDateTime),
    Utc(DateTime<Utc>),
}

fn deserialize_string_to_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    use serde_json::Value;

    let value = Value::deserialize(deserializer)?;
    match value {
        Value::String(s) => Ok(vec![s]),
        _ => Err(Error::custom("Expected string value")),
    }
}

#[derive(Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[derive(Debug, Clone)]
pub enum UnlockTrigger {
    /// # Unlock after a specific time
    Time { after: UnlockTriggerTimeFormat },
    /// # Unlock after completion of specific sessions in the same module
    Completion {
        #[serde(deserialize_with = "deserialize_string_to_vec")]
        #[schemars(with = "String")]
        /// # Sesssion ID of the sessions which need to be completed to unlock
        after: Vec<String>,
        /// # Optional wait time after completion before unlocking
        wait: Option<UnlockTriggerWait>,
    },
}

#[derive(Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[derive(Debug, Clone, Copy, Default)]
pub enum UnlockTriggerMode {
    #[default]
    All,
    Any,
}

#[derive(Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[derive(Debug, Clone)]
pub struct Unlock {
    /// # Conditions to unlock
    pub triggers: Vec<UnlockTrigger>,
    #[serde(default)]
    /// # Mode to evaluate the unlock triggers
    pub trigger_mode: UnlockTriggerMode,
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDateTime, TimeZone};

    use super::*;

    #[test]
    fn test_session_unlock_trigger_time_serialization_local() {
        let local_time = NaiveDateTime::default();
        let local_time_trigger = UnlockTrigger::Time {
            after: UnlockTriggerTimeFormat::Local(local_time),
        };

        let serialized_local_time_trigger = serde_json::to_string(&local_time_trigger).unwrap();
        assert_eq!(
            serialized_local_time_trigger,
            r#"{"time":{"after":"1970-01-01T00:00:00"}}"#
        );

        let deserialized_local_time_trigger: UnlockTrigger =
            serde_json::from_str(&serialized_local_time_trigger).unwrap();
        if let UnlockTrigger::Time {
            after: UnlockTriggerTimeFormat::Local(time),
        } = deserialized_local_time_trigger
        {
            assert_eq!(time, NaiveDateTime::default());
        } else {
            panic!("Deserialize NaiveDateTime failed");
        }
    }

    #[test]
    fn test_session_unlock_trigger_time_serialization_utc() {
        let utc_time = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let utc_time_trigger = UnlockTrigger::Time {
            after: UnlockTriggerTimeFormat::Utc(utc_time),
        };

        let serialized_utc_time_trigger = serde_json::to_string(&utc_time_trigger).unwrap();
        assert_eq!(
            serialized_utc_time_trigger,
            r#"{"time":{"after":"2024-01-01T12:00:00Z"}}"#
        );

        let deserialized_utc_time_trigger: UnlockTrigger = serde_json::from_str(&serialized_utc_time_trigger).unwrap();
        if let UnlockTrigger::Time {
            after: UnlockTriggerTimeFormat::Utc(time),
        } = deserialized_utc_time_trigger
        {
            assert_eq!(time, Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap());
        } else {
            panic!("Deserialize UTC failed");
        }
    }
}

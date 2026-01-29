use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fmt::Debug;
use utoipa::openapi::{RefOr, Schema};
use utoipa::{PartialSchema, ToSchema, schema};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Status {
    pub database: Value,
    pub worker: Value,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ToSchema, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentState {
    Ok,
    Error,
}

impl From<StatusCode> for ComponentState {
    fn from(value: StatusCode) -> Self {
        if value.is_success() { Self::Ok } else { Self::Error }
    }
}

#[derive(Debug, Clone)]
pub struct ComponentStatus {
    state: ComponentState,
    message: Option<Value>,
}

impl PartialSchema for ComponentStatus {
    fn schema() -> RefOr<Schema> {
        schema!(String).into()
    }
}

impl ToSchema for ComponentStatus {}

impl Serialize for ComponentStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self.message {
            Some(message) => message.serialize(serializer),
            None => self.state.serialize(serializer),
        }
    }
}

impl<T, E> From<Result<T, E>> for ComponentStatus {
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(_) => Self::ok(),
            Err(_) => Self::error(),
        }
    }
}

impl ComponentStatus {
    pub fn new<S: Into<ComponentState>>(state: S, message: Option<Value>) -> Self {
        Self {
            state: state.into(),
            message,
        }
    }

    #[must_use]
    pub fn ok() -> Self {
        Self::new(ComponentState::Ok, None)
    }

    #[must_use]
    pub fn from_ok_text(message: &str) -> Self {
        Self::new(ComponentState::Ok, Some(json!(message)))
    }

    #[must_use]
    pub fn error() -> Self {
        Self::new(ComponentState::Error, None)
    }

    #[must_use]
    pub fn from_error_text(message: &str) -> Self {
        Self::new(ComponentState::Error, Some(json!(message)))
    }

    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.state == ComponentState::Ok
    }

    #[must_use]
    pub fn into_message(self) -> Value {
        match self.message {
            Some(message) => message,
            // This is safe because the serialization can never fail.
            None => serde_json::to_value(self.state).expect("failed to serialize component status"),
        }
    }
}

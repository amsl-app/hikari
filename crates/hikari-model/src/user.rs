use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, Display, AsRefStr)]
pub enum Gender {
    #[serde(rename = "OTHER", alias = "other", alias = "Other")]
    Other,
    #[serde(rename = "MALE", alias = "male", alias = "Male")]
    Male,
    #[serde(rename = "FEMALE", alias = "female", alias = "Female")]
    Female,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, ToSchema)]
pub struct User {
    pub id: Uuid,
    #[schema(example = "username")]
    pub onboarding: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birthday: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semester: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<Gender>,
    #[serde(skip_serializing)]
    pub current_module: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        assert_eq!(r#""OTHER""#, serde_json::to_string(&Gender::Other).unwrap());
        assert_eq!(r#""MALE""#, serde_json::to_string(&Gender::Male).unwrap());
        assert_eq!(r#""FEMALE""#, serde_json::to_string(&Gender::Female).unwrap());
        let id = Uuid::new_v4();
        assert_eq!(
            format!(r#"{{"id":"{id}","onboarding":false,"gender":"FEMALE"}}"#),
            serde_json::to_string(&User {
                id,
                gender: Some(Gender::Female),
                ..Default::default()
            })
            .unwrap()
        );
    }

    #[test]
    fn test_display() {
        assert_eq!("Male", format!("{}", Gender::Male));
        assert_eq!("Female", format!("{}", Gender::Female));
        assert_eq!("Other", format!("{}", Gender::Other));
    }
}

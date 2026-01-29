use jsonwebtoken::errors::ErrorKind;
use std::collections::{HashMap, HashSet};

use hikari_oidc::{JwkClient, JwkError, ValidationOptions};
use jsonwebtoken::TokenData;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum AuthError {
    #[error("Invalid JWT claims")]
    InvalidJWTClaim,
    #[error("User in unauthorized")]
    Unauthorized,
    #[error("Missing claim: {0}")]
    MissingClaim(String),
    #[error("Claim {0} has invalid value {1}")]
    InvalidClaimValue(String, String),

    #[error("Wrong group")]
    WrongGroup,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct UserToken {
    // issued at
    pub iat: i64,
    pub sub: String,
    pub aud: String,

    #[serde(flatten)]
    pub values: serde_json::Map<String, Value>,
}

pub(crate) fn get_groups(claims: &mut UserToken, group_claims: Option<&String>) -> Result<HashSet<String>, AuthError> {
    let claims_groups = group_claims
        .and_then(|claim| claims.values.remove(claim))
        .map(|claims| match claims {
            Value::Array(groups) => group_values_to_string(groups),
            Value::String(group) => Ok(HashSet::from([group])),
            _ => {
                tracing::error!("groups claim has the wrong type");
                Err(AuthError::InvalidJWTClaim)
            }
        })
        .transpose()?
        .unwrap_or_default();
    Ok(claims_groups)
}

fn group_values_to_string(values: Vec<Value>) -> Result<HashSet<String>, AuthError> {
    values
        .into_iter()
        .map(|val| {
            if let Value::String(val) = val {
                Ok(val)
            } else {
                tracing::error!("group value has the wrong type");
                Err(AuthError::InvalidJWTClaim)
            }
        })
        .collect()
}

type ValidationResult = (String, HashSet<String>);

pub(crate) async fn validate_jwt(
    token: &str,
    audience: &HashSet<String>,
    required_claims: &HashMap<String, Option<Value>>,
    group_claim: Option<&String>,
    state_groups: &HashSet<String>,
    jwk_client: &JwkClient,
) -> Result<Option<ValidationResult>, AuthError> {
    let options = ValidationOptions {
        audience: Some(audience.iter().cloned().collect()),
    };
    match jwk_client.decode(token, options) {
        Ok(token) => {
            let token: TokenData<UserToken> = token;
            let mut claims = token.claims;

            for (required_claim, required_value) in required_claims {
                let Some(claim) = claims.values.get(required_claim) else {
                    return Err(AuthError::MissingClaim(required_claim.clone()));
                };
                if let Some(required_value) = required_value
                    && claim != required_value
                {
                    return Err(AuthError::InvalidClaimValue(
                        required_claim.clone(),
                        required_value.to_string(),
                    ));
                }
            }
            let claims_groups = get_groups(&mut claims, group_claim)?;
            if !claims_groups.is_superset(state_groups) {
                tracing::warn!(required_groups = ?state_groups, actual_groups = ?claims_groups, "missing groups");
                return Err(AuthError::WrongGroup);
            }

            return Ok(Some((claims.sub, claims_groups)));
        }
        Err(error) => {
            if let JwkError::Jwk(error) = error
                && !matches!(
                    error.kind(),
                    ErrorKind::InvalidToken | ErrorKind::ExpiredSignature | ErrorKind::ImmatureSignature
                )
            {
                return Err(AuthError::Unauthorized);
            }
        }
    }

    Ok(None)
}

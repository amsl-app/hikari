use crate::refresh::{Refresh, RefreshableValue, Refresher};
use jsonwebtoken::jwk::{JwkSet, PublicKeyUse};
use jsonwebtoken::{DecodingKey, TokenData, Validation};
use reqwest::{Client, IntoUrl};
use serde::de::DeserializeOwned;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::pin::Pin;
use std::time::Duration;
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum JwkError {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Jwk(#[from] jsonwebtoken::errors::Error),
    #[error("Key with specified id does not exist")]
    KeyNotFound,
    #[error("Key is missing field {0}")]
    MissingField(&'static str),
}

pub struct Config {
    pub(crate) url: Url,
}

pub trait JwkHttpClient<Error> {
    fn get<T: DeserializeOwned + 'static, U: IntoUrl + Send + Sync>(
        &self,
        url: U,
    ) -> impl Future<Output = Result<T, Error>> + Send + Sync;
}

impl<Error> JwkHttpClient<Error> for Client
where
    Error: From<JwkError>,
{
    async fn get<T: DeserializeOwned, U: IntoUrl + Send + Sync>(&self, url: U) -> Result<T, Error> {
        let json = self
            .get(url)
            .send()
            .await
            .map_err(JwkError::Reqwest)?
            .json()
            .await
            .map_err(JwkError::Reqwest)?;
        Ok(json)
    }
}

struct JwkRefresher<C> {
    client: C,
    url: Url,
}

impl<C> Refresher for JwkRefresher<C>
where
    C: JwkHttpClient<JwkError> + Clone + Send + Sync + 'static,
{
    type Error = JwkError;
    type Output = HashMap<String, DecodingKey>;
    type Future = Pin<Box<dyn Future<Output = Result<Refresh<Self::Output>, Self::Error>> + Send + Sync>>;

    fn refresh(&self) -> Self::Future {
        let client = self.client.clone();
        let url = self.url.clone();
        Box::pin(async move {
            let jwk_set: JwkSet = client.get(url).await?;

            let signing_keys = jwk_set
                .keys
                .into_iter()
                .filter(|jwk| jwk.is_supported() && matches!(jwk.common.public_key_use, Some(PublicKeyUse::Signature)));
            let key_map = signing_keys
                .filter_map(|jwk| {
                    let decoding_key = DecodingKey::from_jwk(&jwk);
                    jwk.common.key_id.map(|id| decoding_key.map(|key| (id, key)))
                })
                .collect::<Result<_, _>>()
                .map_err(JwkError::Jwk)?;
            Ok(Refresh::new(key_map, Duration::from_mins(5), Duration::from_mins(5)))
        })
    }
}

pub struct JwkClient<Client = reqwest::Client>
where
    Client: JwkHttpClient<JwkError> + Default + Clone + Send + Sync + 'static,
{
    keys: RefreshableValue<HashMap<String, DecodingKey>, JwkRefresher<Client>, JwkError>,
}

pub type DefaultJwkClient = JwkClient<Client>;

pub struct ValidationOptions {
    pub audience: Option<HashSet<String>>,
}

impl<HttpClient> JwkClient<HttpClient>
where
    HttpClient: JwkHttpClient<JwkError> + Default + Clone + Send + Sync + 'static,
{
    pub async fn new(config: Config) -> Result<Self, JwkError> {
        let refresher = JwkRefresher {
            client: HttpClient::default(),
            url: config.url,
        };
        Ok(Self {
            keys: RefreshableValue::new(refresher).await?,
        })
    }

    pub fn refresh(&self) -> bool {
        self.keys.refresh()
    }

    pub fn decode<T: DeserializeOwned>(
        &self,
        token: impl AsRef<[u8]>,
        options: ValidationOptions,
    ) -> Result<TokenData<T>, JwkError> {
        let keys = &self.keys.get_unchecked().value;
        let token = token.as_ref();
        let header = jsonwebtoken::decode_header(token).map_err(JwkError::Jwk)?;
        let Some(kid) = header.kid else {
            return Err(JwkError::MissingField("kid"));
        };
        let Some(key) = keys.get(&kid) else {
            return Err(JwkError::KeyNotFound);
        };
        let mut validation = Validation::new(header.alg);
        if let Some(audience) = options.audience {
            validation.aud = Some(audience);
        }
        jsonwebtoken::decode(token, key, &validation).map_err(JwkError::Jwk)
    }
}

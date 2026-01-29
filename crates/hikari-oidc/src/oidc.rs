use crate::jwks::{Config, JwkError, JwkHttpClient};
use reqwest::Client;
use serde::Deserialize;
use std::marker::PhantomData;
use typed_builder::TypedBuilder;
use url::Url;

#[derive(Deserialize, Debug)]
pub struct OidcWellKnown {
    jwks_uri: Url,
}

#[derive(Debug, TypedBuilder)]
pub struct OidcConfig<C = Client> {
    jwk_set_url: Url,
    #[builder(default)]
    _client: PhantomData<C>,
}

pub type DefaultOidcConfig = OidcConfig<Client>;

impl<C> OidcConfig<C>
where
    C: JwkHttpClient<JwkError> + Default,
{
    pub async fn builder_from_discovery_url(
        discovery_url: Url,
    ) -> Result<OidcConfigBuilder<C, ((Url,), ())>, JwkError> {
        let client = C::default();

        let oidc_config: OidcWellKnown = client.get(discovery_url).await?;
        let builder = Self::builder().jwk_set_url(oidc_config.jwks_uri);
        Ok(builder)
    }

    pub async fn from_issuer_url(issuer_url: Url) -> Result<OidcConfigBuilder<C, ((Url,), ())>, JwkError> {
        let discovery_url = issuer_url
            .join(".well-known/openid-configuration")
            .expect("failed to parse well known path");
        Self::builder_from_discovery_url(discovery_url).await
    }
}

impl<C> From<OidcConfig<C>> for Config {
    fn from(config: OidcConfig<C>) -> Self {
        Self {
            url: config.jwk_set_url,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jwks::{JwkClient, JwkError, JwkHttpClient};
    use crate::test_utils::load_json;
    use jsonwebtoken::jwk::JwkSet;
    use reqwest::IntoUrl;
    use serde::de::DeserializeOwned;
    use std::any::TypeId;
    use test_log::test;

    #[derive(Clone, Default)]
    struct MockClient {}

    impl MockClient {
        fn new() -> Self {
            Self {}
        }
    }

    impl JwkHttpClient<JwkError> for MockClient {
        async fn get<T: DeserializeOwned + 'static, U: IntoUrl>(&self, _: U) -> Result<T, JwkError> {
            let type_id = TypeId::of::<T>();
            if type_id == TypeId::of::<JwkSet>() {
                let jwk_set = load_json("jwks.json");
                return Ok(jwk_set);
            };
            if type_id == TypeId::of::<OidcWellKnown>() {
                let jwk_set = load_json("oidc-configuration.json");
                return Ok(jwk_set);
            };
            todo!()
        }
    }

    #[test(tokio::test)]
    async fn test_refresh() {
        let config = OidcConfig::<MockClient>::builder_from_discovery_url("https://example.com/".parse().unwrap())
            .await
            .unwrap()
            .build();
        let client: JwkClient<MockClient> = JwkClient::new(config.into()).await.unwrap();
    }
}

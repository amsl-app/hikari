use crate::client::base::{BaseClient, Config, SecureClient};
use crate::error::{Error, HttpError};
use async_trait::async_trait;
use chrono::Utc;
use futures_retry_policies::retry_policies::RetryPolicies;
use futures_retry_policies::tokio::RetryFutureExt;
use hikari_http::{BaseHttpClient, HttpClient};
use openidconnect::core::CoreClient;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use hikari_http::retry::MaybeRetry;
use hikari_model::login::Token;
use openidconnect::http::Method;
use openidconnect::{
    EndpointMaybeSet, EndpointNotSet, EndpointSet, ResourceOwnerPassword, ResourceOwnerUsername, Scope, TokenResponse,
};
use retry_policies::policies::ExponentialBackoff;
use uri_url::url_to_uri;

pub struct User {
    pub username: String,
    pub password: String,
}

pub type OidcClientType =
    CoreClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointMaybeSet, EndpointMaybeSet>;

pub struct OidcClient {
    pub oidc_client: OidcClientType,
    pub user: User,
    pub config: Config,
    pub http_client: HttpClient,
    token: Arc<RwLock<Option<String>>>,
}

impl OidcClient {
    #[must_use]
    pub fn new(oidc_client: OidcClientType, user: User, config: Config) -> Self {
        Self {
            oidc_client,
            user,
            config,
            http_client: HttpClient::default(),
            token: Arc::new(RwLock::new(None)),
        }
    }

    async fn get_oidc_token(&self) -> Result<String, Error> {
        let username = ResourceOwnerUsername::new(self.user.username.clone());
        let password = ResourceOwnerPassword::new(self.user.password.clone());
        let token_request = self
            .oidc_client
            .exchange_password(&username, &password)
            .map_err(|err| Error::Oidc(err.to_string()))?
            .add_scope(Scope::new("openid".to_owned()));

        let token_response = token_request
            .request_async(&self.http_client)
            .await
            .map_err(|err| Error::Oidc(err.to_string()))?;
        token_response
            .id_token()
            .map(std::string::ToString::to_string)
            .ok_or(Error::Oidc("Missing id Token in token response".to_owned()))
    }

    async fn get_hikari_token(&self, token: String) -> Result<String, Error> {
        let token_url = self.config.base_url.token()?;
        let do_request = move || {
            let token = token.clone();
            let token_url = token_url.clone();

            async move {
                tracing::debug!("Performing token request.");

                let request = http::request::Request::builder()
                    .method(Method::POST)
                    .uri(
                        url_to_uri(&token_url)
                            .map_err(Into::<HttpError>::into)
                            .map_err(MaybeRetry::NoRetry)?,
                    )
                    .body(token.into())
                    .map_err(HttpError::from)
                    .map_err(MaybeRetry::NoRetry)?;
                let response = self
                    .http_client
                    .request_text(request)
                    .await
                    .map_err(|err| MaybeRetry::MaybeRetry(err.into()))?;
                Result::<_, MaybeRetry<HttpError>>::Ok(response)
            }
        };
        let backoff_builder =
            ExponentialBackoff::builder().retry_bounds(Duration::from_millis(500), Duration::from_secs(600));
        let total_timeout = Duration::from_secs(60 * 15);
        let backoff = backoff_builder
            .build_with_total_retry_duration_and_max_retries(total_timeout)
            .for_task_started_at(Utc::now());
        let retry_policy = RetryPolicies::new(backoff);
        let response = do_request.retry(retry_policy).await;

        let response = response.map_err(MaybeRetry::into_inner)?;

        Ok(serde_json::from_str::<Token>(response.body())?.access_token)
    }
}

impl BaseClient for OidcClient {
    fn get_http_client(&self) -> &HttpClient {
        &self.http_client
    }

    fn get_config(&self) -> &Config {
        &self.config
    }
}

#[async_trait]
impl SecureClient for OidcClient {
    fn get_token(&self) -> Arc<RwLock<Option<String>>> {
        Arc::clone(&self.token)
    }

    async fn fetch_token(&self) -> Result<String, Error> {
        // Blocks are important to make sure the guard gets dropped and unlocked
        {
            let guard = self.token.read().await;
            if let Some(token) = &*guard {
                return Ok(token.clone());
            }
            drop(guard);
        }
        {
            let mut guard = self.token.write().await;
            if let Some(token) = &*guard {
                Ok(token.clone())
            } else {
                let oidc_token = self.get_oidc_token().await?;
                let res = self.get_hikari_token(oidc_token).await;
                if let Ok(token) = &res {
                    *guard = Some(token.clone());
                }
                res
            }
        }
    }
}

use crate::error::{Error, HttpError, InternalError};
use async_trait::async_trait;
use hikari_http::{BaseHttpClient, HttpClient, HttpRequest, HttpResponse};
use hikari_model::chat::{MessageResponse, Payload};
use hikari_model::status::Status;
use openidconnect::http;
use openidconnect::http::{HeaderMap, HeaderValue, Method};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tokio::sync::RwLock;
use uri_url::url_to_uri;
use url::Url;

#[derive(Debug)]
pub struct Config {
    pub base_url: ApiUrl,
}

impl Config {
    #[must_use]
    pub fn new(base_url: Url) -> Self {
        Self {
            base_url: ApiUrl { url: base_url },
        }
    }
}

#[derive(Debug)]
pub struct ApiUrl {
    pub url: Url,
}

impl From<ApiUrl> for Url {
    fn from(api_url: ApiUrl) -> Self {
        api_url.url
    }
}

impl ApiUrl {
    // pub fn base(&self) -> &Url {
    //     &self.url
    // }

    pub fn for_api(&self, api_path: &str) -> Result<Url, InternalError> {
        self.url.join("api/v0/")?.join(api_path).map_err(Into::into)
    }

    pub fn token(&self) -> Result<Url, InternalError> {
        self.url.join("login/token").map_err(Into::into)
    }
}

#[async_trait]
pub trait BaseClient {
    fn get_http_client(&self) -> &HttpClient;
    fn get_config(&self) -> &Config;

    async fn api_send_request<T: DeserializeOwned>(&self, request: HttpRequest) -> Result<HttpResponse<T>, HttpError> {
        tracing::debug!(method = ?request.method(), uri = ?request.uri(), "Sending API request");
        self.get_http_client()
            .request_json(request)
            .await
            .map_err(HttpError::from)
    }

    async fn api_request<T: DeserializeOwned>(&self, method: Method, path: &str) -> Result<T, Error> {
        let url = self.get_config().base_url.for_api(path)?;
        let request = http::request::Request::builder()
            .method(method)
            .uri(url_to_uri(&url).map_err(HttpError::from)?)
            .body(vec![])
            .map_err(HttpError::from)?;
        let res = self.api_send_request(request).await?;
        Ok(res.into_body())
    }
}

#[async_trait]
pub trait PublicClient: BaseClient {
    async fn get_status(&self) -> Result<Status, Error> {
        self.api_request(Method::GET, "status").await
    }
}

impl<T> PublicClient for T where T: BaseClient {}

#[async_trait]
pub trait SecureClient: PublicClient {
    fn get_token(&self) -> Arc<RwLock<Option<String>>>;

    async fn fetch_token(&self) -> Result<String, Error>;

    async fn auth_headers(&self) -> Result<HeaderMap, Error> {
        let mut headers = HeaderMap::new();
        let token = self.fetch_token().await?;
        let mut value = HeaderValue::from_str(&format!("Bearer {token}")).map_err(InternalError::from)?;
        value.set_sensitive(true);
        headers.insert(http::header::AUTHORIZATION, value);
        Ok(headers)
    }

    async fn api_authenticated_request(&self, method: Method, path: &str) -> Result<MessageResponse<Payload>, Error> {
        let url = self.get_config().base_url.for_api(path)?;
        let headers = self.auth_headers().await?;
        let mut request_builder = http::request::Request::builder()
            .method(method)
            .uri(url_to_uri(&url).map_err(HttpError::from)?);
        let request_headers = request_builder.headers_mut();
        if let Some(header_map) = request_headers {
            header_map.extend(headers);
        }
        let res = self
            .api_send_request(request_builder.body(vec![]).map_err(HttpError::from)?)
            .await?;
        Ok(res.into_body())
    }

    async fn start_session(&self, module: &str, session: &str) -> Result<MessageResponse<Payload>, Error> {
        self.api_authenticated_request(Method::POST, &format!("modules/{module}/sessions/{session}/start"))
            .await
    }
}

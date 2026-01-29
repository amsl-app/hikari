use async_trait::async_trait;
use chrono::Utc;
use futures_retry_policies::retry_policies::RetryPolicies;
use futures_retry_policies::tokio::RetryFutureExt;
use openidconnect::AsyncHttpClient;
use std::future::Future;
use std::ops::ControlFlow;
use std::pin::Pin;
use std::time::Duration;

use crate::core::{BaseHttpClient, HttpRequest, HttpResponse};
use crate::error::Error;
use crate::retry::MaybeRetry;
use reqwest::{Request, Response};
use retry_policies::policies::ExponentialBackoff;
use serde::de::DeserializeOwned;
use tokio::time::timeout;

#[derive(Clone)]
pub struct ReqwestHttpClient {
    client: reqwest::Client,
}

impl Default for ReqwestHttpClient {
    fn default() -> Self {
        Self::new().expect("failed to create default client")
    }
}

struct RetryReqwest<T> {
    inner: T,
}

impl<R> RetryReqwest<R> {
    fn new(inner: R) -> Self {
        Self { inner }
    }
}

impl<R, E, T: futures_retry_policies::RetryPolicy<Result<R, MaybeRetry<E>>>>
    futures_retry_policies::RetryPolicy<Result<R, MaybeRetry<E>>> for RetryReqwest<T>
{
    fn should_retry(&mut self, result: Result<R, MaybeRetry<E>>) -> ControlFlow<Result<R, MaybeRetry<E>>, Duration> {
        tracing::debug!("retrying request");
        self.inner.should_retry(result)
    }
}

impl ReqwestHttpClient {
    pub fn new() -> Result<Self, Error> {
        let mut client_builder = reqwest::ClientBuilder::new();
        client_builder = client_builder.redirect(reqwest::redirect::Policy::none());

        let client = client_builder.build()?;

        Ok(Self { client })
    }

    fn build_request(&self, request: HttpRequest) -> Result<Request, Error> {
        let mut request_builder = self.client.request(request.method().clone(), request.uri().path());
        for (name, value) in request.headers() {
            request_builder = request_builder.header(name.as_str(), value.as_bytes());
        }
        request_builder.body(request.into_body()).build().map_err(Into::into)
    }

    async fn execute_request(&self, request: HttpRequest) -> Result<Response, Error> {
        let backoff_builder =
            ExponentialBackoff::builder().retry_bounds(Duration::from_millis(500), Duration::from_secs(600));
        let total_timeout = Duration::from_secs(60 * 15);
        let backoff = backoff_builder
            .build_with_total_retry_duration_and_max_retries(total_timeout)
            .for_task_started_at(Utc::now());
        let retry_policy = RetryReqwest::new(RetryPolicies::new(backoff));

        let do_request = move || {
            let request = request.clone();
            async move {
                let request: Request = self.build_request(request).map_err(MaybeRetry::NoRetry)?;
                let response: Result<Response, _> = self
                    .client
                    .execute(request)
                    .await
                    .map_err(Error::from)
                    .map_err(MaybeRetry::MaybeRetry);
                tracing::debug!("Performing login request.");
                response
            }
        };

        timeout(total_timeout, do_request.retry(retry_policy))
            .await
            .map_err(|_| Error::Timeout)?
            .map_err(MaybeRetry::into_inner)
    }

    pub async fn request_oidc(&self, request: HttpRequest) -> Result<HttpResponse<Vec<u8>>, Error> {
        let response = self.execute_request(request).await?;

        let status_code = response.status();
        let headers = response.headers().clone();
        let chunks = response.bytes().await?;
        let mut http_response = http::response::Response::builder().status(status_code);
        if let Some(header_map) = http_response.headers_mut() {
            header_map.extend(headers.into_iter());
        }
        http_response.body(chunks.to_vec()).map_err(Into::into)
    }
}

impl<'c> AsyncHttpClient<'c> for ReqwestHttpClient {
    type Error = crate::Error;
    type Future = Pin<Box<dyn Future<Output = Result<openidconnect::HttpResponse, Self::Error>> + Send + Sync + 'c>>;

    fn call(&'c self, request: openidconnect::HttpRequest) -> Self::Future {
        Box::pin(async move { self.request_oidc(request).await })
    }
}

#[async_trait]
impl BaseHttpClient for ReqwestHttpClient {
    type Error = Error;

    async fn request_text(&self, request: HttpRequest) -> Result<HttpResponse<String>, Self::Error> {
        let response = self.client.execute(self.build_request(request)?).await?;

        if response.status().is_success() {
            let status_code = response.status();
            let headers = response.headers().clone();
            let text = response.text().await?;

            let mut http_response = http::response::Response::builder().status(status_code);
            if let Some(header_map) = http_response.headers_mut() {
                header_map.extend(headers);
            }

            let response = http_response.body(text.clone())?;

            Ok(response)
        } else {
            Err(Error::StatusCode(Box::new(response)))
        }
    }

    async fn request_json<T: DeserializeOwned>(&self, request: HttpRequest) -> Result<HttpResponse<T>, Self::Error> {
        let response = self.client.execute(self.build_request(request)?).await?;

        if response.status().is_success() {
            let status_code = response.status();
            let headers = response.headers().clone();
            let text = response.json().await?;
            let mut http_response = http::response::Response::builder().status(status_code);
            if let Some(header_map) = http_response.headers_mut() {
                header_map.extend(headers);
            }
            http_response.body(text).map_err(Into::into)
        } else {
            Err(Error::StatusCode(Box::new(response)))
        }
    }
}

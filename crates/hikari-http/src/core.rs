use async_trait::async_trait;
use serde::de::DeserializeOwned;

pub type HttpRequest = http::request::Request<Vec<u8>>;

pub type HttpResponse<T> = http::response::Response<T>;

#[async_trait]
pub trait BaseHttpClient: Send + Default + Clone {
    type Error;

    async fn request_text(&self, request: HttpRequest) -> Result<HttpResponse<String>, Self::Error>;

    async fn request_json<T: DeserializeOwned>(&self, request: HttpRequest) -> Result<HttpResponse<T>, Self::Error>;
}

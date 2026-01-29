use crate::client::base::{BaseClient, Config};
use hikari_http::HttpClient;

pub struct SimpleClient {
    config: Config,
    http_client: HttpClient,
}

impl SimpleClient {
    #[must_use]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            http_client: HttpClient::default(),
        }
    }
}

impl BaseClient for SimpleClient {
    fn get_http_client(&self) -> &HttpClient {
        &self.http_client
    }

    fn get_config(&self) -> &Config {
        &self.config
    }
}

use crate::opt::Auth;
use crate::permissions::extract;
use crate::{AppConfig, routes};
use axum::routing::get;
use axum::{Extension, Router};
use axum_prometheus::PrometheusMetricLayerBuilder;

use hikari_oidc::{DefaultJwkClient, DefaultOidcConfig, JwkClient};
use hikari_utils::tower::otel;
use http::{Method, header};
use protect_axum::GrantsLayer;
use sea_orm::DatabaseConnection;
use sentry_tower::NewSentryLayer;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::{task, time};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

pub(crate) struct InnerAuthConfig {
    jwk_client: Arc<DefaultJwkClient>,
    required_claims: HashMap<String, Option<Value>>,
    audience: HashSet<String>,
    groups: HashSet<String>,
    groups_claim: Option<String>,
}

impl InnerAuthConfig {
    pub(crate) fn jwk(&self) -> &JwkClient {
        &self.jwk_client
    }

    pub(crate) fn required_claims(&self) -> &HashMap<String, Option<Value>> {
        &self.required_claims
    }

    pub(crate) fn audience(&self) -> &HashSet<String> {
        &self.audience
    }

    pub(crate) fn groups(&self) -> &HashSet<String> {
        &self.groups
    }

    pub(crate) fn groups_claim(&self) -> Option<&String> {
        self.groups_claim.as_ref()
    }
}

#[derive(Clone)]
pub(crate) struct AuthConfig(Arc<InnerAuthConfig>);

impl AuthConfig {
    fn new(
        jwk_client: Arc<DefaultJwkClient>,
        required_claims: HashMap<String, Option<Value>>,
        audience: HashSet<String>,
        groups: HashSet<String>,
        groups_claim: Option<String>,
    ) -> Self {
        Self(Arc::new(InnerAuthConfig {
            jwk_client,
            required_claims,
            audience,
            groups,
            groups_claim,
        }))
    }
}

impl AsRef<InnerAuthConfig> for AuthConfig {
    fn as_ref(&self) -> &InnerAuthConfig {
        &self.0
    }
}

pub async fn create_app(
    app_config: AppConfig,
    auth: Auth,
    deletable: bool,
    seaorm_pool: DatabaseConnection,
) -> anyhow::Result<Router> {
    let (prometheus_layer, metric_handle) = PrometheusMetricLayerBuilder::new()
        .with_prefix("api")
        .with_default_metrics()
        .build_pair();

    //TODO (MED) handle multiple parallel instances

    let config = DefaultOidcConfig::from_issuer_url(auth.oidc_issuer_url).await?.build();
    let jwk_client = Arc::new(DefaultJwkClient::new(config.into()).await?);

    let refresh_jwk_client = Arc::clone(&jwk_client);
    task::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if refresh_jwk_client.refresh() {
                tracing::debug!("refreshing jwk set");
            }
        }
    });

    tracing::info!(audiences = ?auth.audience, "allowing audiences");

    let required_claims: HashMap<_, _> = auth.required_claims.into_iter().map(|v| (v.name, v.value)).collect();
    if !required_claims.is_empty() {
        tracing::info!(?required_claims, "requiring claims");
    }

    if !auth.groups.is_empty() {
        tracing::info!(groups = ?auth.groups, "requiring groups");
    }

    // CORS for login routes - users don't have credentials yet during authentication
    let login_cors = CorsLayer::new()
        .allow_origin(
            auth.origins
                .iter()
                .map(|origin| origin.parse())
                .collect::<Result<Vec<_>, _>>()?,
        )
        .allow_headers([
            header::ACCEPT,
            header::CONTENT_TYPE,
            header::COOKIE,
            header::ORIGIN
        ])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::OPTIONS,
        ])
        .allow_credentials(true) // No credentials needed for login flow
        .max_age(Duration::from_secs(3600));

    // CORS for API routes - users have credentials for authenticated endpoints
    let api_cors = CorsLayer::new()
        .allow_origin(
            auth.origins
                .iter()
                .map(|origin| origin.parse())
                .collect::<Result<Vec<_>, _>>()?,
        )
        .allow_headers([
            header::ACCEPT,
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ORIGIN,
            header::UPGRADE,
            header::SEC_WEBSOCKET_KEY,
            header::SEC_WEBSOCKET_VERSION,
            header::SEC_WEBSOCKET_EXTENSIONS,
        ])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .max_age(Duration::from_secs(3600));

    let mut app = Router::new()
        .merge(routes::swagger::create_router())
        .merge(routes::global::create_router());

    app = app.merge(routes::login::create_router().layer(login_cors));

    let app = app
        .nest(
            "/api/v0",
            Router::new()
                .nest("/status", routes::api::v0::status::create_router())
                .nest("/user", routes::api::v0::user::create_router(deletable))
                .nest("/bots", routes::api::v0::bots::create_router())
                .nest("/modules", routes::api::v0::modules::create_router())
                .nest("/assessments", routes::api::v0::assessment::create_router())
                .nest("/journal", routes::api::v0::journal::create_router())
                .nest("/llm", routes::api::v0::llm::create_router())
                .nest("/quizzes", routes::api::v0::quiz::create_router())
                .nest("/ws", routes::api::v0::ws::create_router())
                .layer(api_cors), // Use API-specific CORS for authenticated routes
        )
        .route("/metrics", get(|| async move { metric_handle.render() }))
        .layer(
            // Router layers are called bottom to top
            // ServiceBuilder layers are called top to bottom
            ServiceBuilder::new()
                .layer(NewSentryLayer::new_from_top())
                .layer(sentry_tower::SentryHttpLayer::new().enable_transaction())
                .layer(prometheus_layer)
                .layer(otel::Layer::new())
                .layer(Extension(app_config))
                .layer(Extension(AuthConfig::new(
                    jwk_client,
                    required_claims,
                    auth.audience.into_iter().collect(),
                    auth.groups.into_iter().collect(),
                    auth.groups_claim,
                )))
                .layer(Extension(seaorm_pool))
                .layer(GrantsLayer::with_extractor(extract)),
        )
        .with_state(());
    Ok(app)
}

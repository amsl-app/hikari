use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

use anyhow::Result;
use axum::routing::get;
use axum::{Extension, Router, serve};
use axum_prometheus::PrometheusMetricLayer;
use clap::Parser;
use hikari_config::global::GlobalConfig;
use hikari_core::llm_config::LlmConfig;
use sea_orm::{ConnectOptions, Database};
use sentry_tower::{NewSentryLayer, SentryHttpLayer};
use tower::ServiceBuilder;

use crate::opt::{Commands, Run};
use crate::routes::api::v0;
use hikari_utils::loader::s3::S3Config;
use hikari_utils::net::create_listener;
use hikari_utils::tower::otel;

mod opt;
mod routes;

const DEFAULT_HOST: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
const DEFAULT_PORT: u16 = 3035;

async fn run(opt: Run) -> Result<()> {
    let _guard = hikari_utils::tracing::setup(
        hikari_utils::tracing::TracingConfig::builder()
            .package(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"))
            .sentry_dsn(opt.sentry_dsn.clone())
            .env(opt.env.clone())
            .otlp_endpoint(opt.otlp_endpoint.clone())
            .build(),
    );
    tracing::info!("starting hikari worker");
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();

    let mut seaorm_pool_options = ConnectOptions::new(opt.db_url.clone());
    if let Some(min_connections) = opt.db_min_connections {
        seaorm_pool_options.min_connections(min_connections);
    }
    if let Some(max_connections) = opt.db_max_connections {
        seaorm_pool_options.max_connections(max_connections);
    }
    seaorm_pool_options.sqlx_logging_level(log::LevelFilter::Debug);

    let s3_config: Option<S3Config> = opt.s3;

    let loader_handler = hikari_utils::loader::LoaderHandler::new(s3_config);

    let config = if let Some(path) = &opt.config {
        hikari_config::global::load(loader_handler.loader(path)?).await?
    } else {
        GlobalConfig::default()
    };

    let llm_config: LlmConfig = opt.llm_services.into();
    tracing::info!("connecting to database");
    let db = Database::connect(seaorm_pool_options).await?;

    let mut app = Router::new()
        .merge(routes::swagger::create_router())
        .nest("/api/v0/csml", v0::csml::create_router(config, llm_config.clone()))
        .nest("/api/v0/journal", v0::journal::create_router(Arc::new(llm_config)))
        .route("/api/v0/health", get(v0::get_health));

    app = app
        .layer(Extension(db))
        .route("/metrics", get(|| async move { metric_handle.render() }))
        .layer(
            ServiceBuilder::new()
                .layer(NewSentryLayer::new_from_top())
                .layer(SentryHttpLayer::new().enable_transaction())
                .layer(prometheus_layer)
                .layer(otel::Layer::new()),
        );

    let listener = create_listener((opt.host, opt.port), (DEFAULT_HOST, DEFAULT_PORT)).await?;

    tracing::info!("listening on {}", listener.local_addr()?);
    serve::serve(listener, app.into_make_service()).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt = opt::Cli::parse();

    match opt.command {
        Commands::Run(opt) => run(opt).await?,
    }

    Ok(())
}

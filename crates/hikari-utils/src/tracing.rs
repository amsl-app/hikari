use opentelemetry::trace::TracerProvider;
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::{ExporterBuildError, MetricExporter, SpanExporter, WithExportConfig};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::Temporality;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::trace::{RandomIdGenerator, Sampler, SdkTracerProvider, TraceError};
use opentelemetry_semantic_conventions::SCHEMA_URL;
use opentelemetry_semantic_conventions::resource::{DEPLOYMENT_ENVIRONMENT_NAME, SERVICE_NAME, SERVICE_VERSION};
use sentry::ClientInitGuard;
use sentry_tracing::EventFilter;
use std::borrow::Cow;
use std::time::Duration;
use thiserror::Error;
use tracing_core::{Level, LevelFilter};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use typed_builder::TypedBuilder;

#[derive(TypedBuilder, Debug)]
pub struct TracingConfig {
    // Have to initialize that in the actual library or else the sentry release will be wrong
    pub package: &'static str,
    pub version: &'static str,
    #[builder(default)]
    pub sentry_dsn: Option<String>,
    #[builder(setter(into), default = String::from("dev"))]
    pub env: String,
    #[builder(default)]
    pub otlp_endpoint: Option<String>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Trace(#[from] TraceError),
    #[error(transparent)]
    Metrics(#[from] ExporterBuildError),
    #[error(transparent)]
    TracingInit(#[from] tracing_subscriber::util::TryInitError),
}

pub struct TracingGuard {
    _sentry: ClientInitGuard,
    providers: Option<(SdkTracerProvider, SdkMeterProvider)>,
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        if let Some((tracer_provider, meter_provider)) = &self.providers {
            if let Err(err) = tracer_provider.shutdown() {
                eprintln!("Error during tracer provider shutdown:\n{err:?}");
            }
            if let Err(err) = meter_provider.shutdown() {
                eprintln!("Error during meter provider shutdown:\n{err:?}");
            }
        }
    }
}

fn init_meter_provider(resource: Resource, config: String) -> Result<SdkMeterProvider, ExporterBuildError> {
    let builder = MetricExporter::builder()
        .with_tonic()
        .with_endpoint(config)
        .with_temporality(Temporality::default());
    let exporter = builder.build()?;

    let reader = PeriodicReader::builder(exporter)
        .with_interval(Duration::from_secs(30))
        .build();

    // For debugging in development
    // let stdout_reader = PeriodicReader::builder(
    //     opentelemetry_stdout::MetricsExporter::default(),
    //     runtime::Tokio,
    // )
    //     .build();

    let meter_provider = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(reader)
        .build();

    global::set_meter_provider(meter_provider.clone());

    Ok(meter_provider)
}

pub fn setup(config: TracingConfig) -> Result<TracingGuard, Error> {
    let guard = sentry::init((
        config.sentry_dsn.clone(),
        sentry::ClientOptions {
            release: Some(Cow::Owned(format!("{}@{}", config.package, config.version))),
            debug: true,
            environment: Some(Cow::Owned(config.env.clone())),
            ..Default::default()
        },
    ));

    let sentry_layer = sentry_tracing::layer().event_filter(|md| match *md.level() {
        Level::ERROR => EventFilter::Event,
        Level::TRACE => EventFilter::Ignore,
        _ => EventFilter::Breadcrumb,
    });

    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with(sentry_layer);
    let providers = if let Some(otlp_endpoint) = config.otlp_endpoint {
        global::set_text_map_propagator(opentelemetry_sdk::propagation::TraceContextPropagator::new());
        let keys = vec![
            KeyValue::new(SERVICE_NAME, config.package),
            KeyValue::new(SERVICE_VERSION, config.version),
            KeyValue::new(DEPLOYMENT_ENVIRONMENT_NAME, config.env),
        ];

        let resource = Resource::builder().with_schema_url(keys.clone(), SCHEMA_URL).build();
        let meter_provider = init_meter_provider(resource.clone(), otlp_endpoint.clone())?;

        let span_exporter = SpanExporter::builder()
            .with_tonic()
            .with_endpoint(otlp_endpoint)
            .build()?;

        let tracer_provider = SdkTracerProvider::builder()
            .with_batch_exporter(span_exporter)
            .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(1.0))))
            .with_id_generator(RandomIdGenerator::default())
            .with_resource(resource)
            .build();
        global::set_tracer_provider(tracer_provider.clone());
        Some((tracer_provider, meter_provider))
    } else {
        None
    };
    let subscriber = if let Some((tracer_provider, meter_provider)) = &providers {
        let tracer = TracerProvider::tracer(tracer_provider, config.package);
        subscriber
            .with(Some(MetricsLayer::new(meter_provider.clone())))
            .with(Some(OpenTelemetryLayer::new(tracer)))
    } else {
        subscriber.with(None).with(None)
    };
    subscriber.try_init()?;
    Ok(TracingGuard {
        _sentry: guard,
        providers,
    })
}

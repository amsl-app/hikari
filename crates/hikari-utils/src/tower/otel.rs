// Source: https://github.com/slickbench/tower-opentelemetry

#![warn(clippy::pedantic)]
use std::{borrow::Cow, error::Error as StdError, future::Future, pin::Pin, sync::Arc, task::Poll};

use futures_util::future::FutureExt;
use http::{
    HeaderValue, Method, Request, Response, Version,
    header::{self, HeaderName},
};
use opentelemetry::{
    Context, InstrumentationScope, KeyValue, global,
    propagation::{Extractor, Injector},
    trace::{FutureExt as OtelFutureExt, SpanId, SpanKind, Status, TraceContextExt, TraceId, Tracer},
};
use opentelemetry_semantic_conventions::trace::{
    HTTP_REQUEST_METHOD, HTTP_RESPONSE_STATUS_CODE, NETWORK_PROTOCOL_NAME, SERVER_ADDRESS, URL_FULL, URL_PATH,
    URL_QUERY, USER_AGENT_ORIGINAL,
};
use sysinfo::System;

#[inline]
fn http_method_str(method: &Method) -> Cow<'static, str> {
    match method {
        &Method::OPTIONS => "OPTIONS".into(),
        &Method::GET => "GET".into(),
        &Method::POST => "POST".into(),
        &Method::PUT => "PUT".into(),
        &Method::DELETE => "DELETE".into(),
        &Method::HEAD => "HEAD".into(),
        &Method::TRACE => "TRACE".into(),
        &Method::CONNECT => "CONNECT".into(),
        &Method::PATCH => "PATCH".into(),
        other => other.to_string().into(),
    }
}

#[inline]
fn http_flavor(version: Version) -> Cow<'static, str> {
    match version {
        Version::HTTP_09 => "0.9".into(),
        Version::HTTP_10 => "1.0".into(),
        Version::HTTP_11 => "1.1".into(),
        Version::HTTP_2 => "2.0".into(),
        Version::HTTP_3 => "3.0".into(),
        other => format!("{other:?}").into(),
    }
}

/// [`Layer`] that adds high level [opentelemetry propagation] to a [`Service`].
///
/// [`Layer`]: tower_layer::Layer
/// [opentelemetry propagation]: https://opentelemetry.io/docs/java/manual_instrumentation/#context-propagation
/// [`Service`]: tower_service::Service
#[derive(Debug, Copy, Clone, Default)]
pub struct Layer {}

impl Layer {
    /// Create a new [`TraceLayer`] using the given [`MakeClassifier`].
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl<S> tower_layer::Layer<S> for Layer
where
    S: Clone,
{
    type Service = Service<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Service::new(inner)
    }
}

/// Middleware [`Service`] that propagates the opentelemetry trace header, configures a span for
/// the request, and records any exceptions.
///
/// [`Service`]: tower_service::Service
#[derive(Clone)]
pub struct Service<S: Clone> {
    inner: S,
    tracer: Arc<global::BoxedTracer>,
}

impl<S> Service<S>
where
    S: Clone,
{
    fn new(inner: S) -> Self {
        let scope = InstrumentationScope::builder(env!("CARGO_PKG_NAME"))
            .with_version(env!("CARGO_PKG_VERSION"))
            .build();
        Self {
            inner,
            tracer: Arc::new(global::tracer_with_scope(scope)),
        }
    }
}

type CF<R, E> = dyn Future<Output = Result<R, E>> + Send;
impl<B, ResBody, S> tower_service::Service<Request<B>> for Service<S>
where
    S: tower_service::Service<Request<B>, Response = Response<ResBody>>,
    S::Future: 'static + Send,
    B: 'static,
    S::Error: std::fmt::Debug + StdError,
    S: Clone,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<CF<Self::Response, Self::Error>>>;

    #[inline]
    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        let parent_context =
            global::get_text_map_propagator(|propagator| propagator.extract(&HeaderCarrier::new(req.headers_mut())));
        // let conn_info = req.connection_info();
        let uri = req.uri();
        let mut builder = self
            .tracer
            .span_builder(uri.path().to_string())
            .with_kind(SpanKind::Server);
        if let Some(trace_id) = parent_context.get::<TraceId>() {
            builder = builder.with_trace_id(*trace_id);
        }
        if let Some(span_id) = parent_context.get::<SpanId>() {
            builder = builder.with_span_id(*span_id);
        }
        let mut attributes: Vec<KeyValue> = Vec::with_capacity(11);
        attributes.push(KeyValue::new(HTTP_REQUEST_METHOD, http_method_str(req.method())));
        attributes.push(KeyValue::new(NETWORK_PROTOCOL_NAME, http_flavor(req.version())));
        attributes.push(KeyValue::new(URL_FULL, uri.to_string()));

        if let Some(host_name) = System::host_name() {
            attributes.push(KeyValue::new(SERVER_ADDRESS, host_name));
        }

        attributes.push(KeyValue::new(URL_PATH, uri.path().to_string()));
        if let Some(query) = uri.query() {
            attributes.push(KeyValue::new(URL_QUERY, query.to_string()));
        }
        if let Some(user_agent) = req.headers().get(header::USER_AGENT).and_then(|s| s.to_str().ok()) {
            attributes.push(KeyValue::new(USER_AGENT_ORIGINAL, user_agent.to_string()));
        }
        builder.attributes = Some(attributes);
        let span = self.tracer.build(builder);
        let cx = Context::current_with_span(span);
        let attachment = cx.clone().attach();

        let fut = self.inner.call(req).with_context(cx.clone()).map(move |res| match res {
            Ok(mut ok_res) => {
                global::get_text_map_propagator(|propagator| {
                    propagator.inject(&mut HeaderCarrier::new(ok_res.headers_mut()));
                });
                let span = cx.span();
                span.set_attribute(KeyValue::new(
                    HTTP_RESPONSE_STATUS_CODE,
                    i64::from(ok_res.status().as_u16()),
                ));
                if ok_res.status().is_server_error() {
                    span.set_status(Status::error(
                        ok_res
                            .status()
                            .canonical_reason()
                            .map(ToString::to_string)
                            .unwrap_or_default(),
                    ));
                }
                span.end();
                Ok(ok_res)
            }
            Err(error) => {
                let span = cx.span();
                span.set_status(Status::error(format!("{error:?}")));
                span.record_error(&error);
                span.end();
                Err(error)
            }
        });

        drop(attachment);
        Box::pin(fut)
    }
}

struct HeaderCarrier<'a> {
    headers: &'a mut http::HeaderMap,
}

impl<'a> HeaderCarrier<'a> {
    fn new(headers: &'a mut http::HeaderMap) -> Self {
        HeaderCarrier { headers }
    }
}

impl Extractor for HeaderCarrier<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.headers.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.headers.keys().map(HeaderName::as_str).collect()
    }
}

impl Injector for HeaderCarrier<'_> {
    fn set(&mut self, key: &str, value: String) {
        self.headers.insert(
            HeaderName::from_bytes(key.as_bytes()).expect("invalid header name"),
            HeaderValue::from_str(&value).expect("invalid header value"),
        );
    }
}

#[cfg(test)]
mod tests {}

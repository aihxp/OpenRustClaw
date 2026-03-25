//! Prometheus metrics endpoint for the OpenRustClaw gateway.
//!
//! Provides a `/metrics` endpoint that exposes metrics in Prometheus exposition format.
//!
//! # Example Output
//!
//! ```text
//! # HELP openrustclaw_requests_total Total number of HTTP/WebSocket requests
//! # TYPE openrustclaw_requests_total counter
//! openrustclaw_requests_total{method="GET",endpoint="/health",status="200"} 42
//!
//! # HELP openrustclaw_request_duration_seconds HTTP/WebSocket request duration
//! # TYPE openrustclaw_request_duration_seconds histogram
//! openrustclaw_request_duration_seconds_bucket{method="GET",endpoint="/health",le="0.1"} 38
//! ...
//! ```

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::get,
};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use metrics_util::MetricKindMask;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tracing::{debug, error, instrument};

static PROMETHEUS_HANDLE: OnceLock<Arc<PrometheusHandle>> = OnceLock::new();

/// State for the metrics endpoint.
#[derive(Clone)]
pub struct MetricsState {
    pub handle: Arc<PrometheusHandle>,
}

impl MetricsState {
    /// Create a new metrics state with the given Prometheus handle.
    pub fn new(handle: Arc<PrometheusHandle>) -> Self {
        Self { handle }
    }
}

impl std::fmt::Debug for MetricsState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetricsState").finish()
    }
}

/// Install the Prometheus metrics exporter and return a handle.
///
/// This should be called once at application startup.
///
/// # Example
///
/// ```rust
/// use openrustclaw_gateway::metrics_endpoint::install_metrics;
///
/// #[tokio::main]
/// async fn main() {
///     let handle = install_metrics();
///     // Use handle to create metrics endpoint...
/// }
/// ```
pub fn install_metrics() -> Arc<PrometheusHandle> {
    PROMETHEUS_HANDLE
        .get_or_init(|| {
            let builder = PrometheusBuilder::new()
                .set_buckets_for_metric(
                    metrics_exporter_prometheus::Matcher::Suffix("_duration_seconds".to_string()),
                    &[
                        0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
                    ],
                )
                .expect("Failed to set duration buckets")
                .set_buckets_for_metric(
                    metrics_exporter_prometheus::Matcher::Suffix("_bytes".to_string()),
                    &[
                        64.0, 256.0, 1024.0, 4096.0, 16384.0, 65536.0, 262144.0, 1048576.0,
                    ],
                )
                .expect("Failed to set bytes buckets");

            let recorder = builder.build_recorder();
            let handle = Arc::new(recorder.handle());

            metrics::set_global_recorder(recorder).expect("Failed to set global metrics recorder");

            handle
        })
        .clone()
}

/// Install the Prometheus metrics exporter with custom configuration.
///
/// # Arguments
///
/// * `idle_timeout` - How long to keep idle metrics before removing them
pub fn install_metrics_with_config(idle_timeout: Option<Duration>) -> Arc<PrometheusHandle> {
    PROMETHEUS_HANDLE
        .get_or_init(|| {
            let mut builder = PrometheusBuilder::new()
                .set_buckets_for_metric(
                    metrics_exporter_prometheus::Matcher::Suffix("_duration_seconds".to_string()),
                    &[
                        0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
                    ],
                )
                .expect("Failed to set duration buckets");

            if let Some(timeout) = idle_timeout {
                builder = builder.idle_timeout(MetricKindMask::ALL, Some(timeout));
            }

            let recorder = builder.build_recorder();
            let handle = Arc::new(recorder.handle());

            metrics::set_global_recorder(recorder).expect("Failed to set global metrics recorder");

            handle
        })
        .clone()
}

/// Create a router with the metrics endpoint.
pub fn metrics_routes(handle: Arc<PrometheusHandle>) -> Router {
    let state = MetricsState::new(handle);

    Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(state)
}

/// Handler for the `/metrics` endpoint.
///
/// Returns metrics in Prometheus exposition format.
#[instrument(skip(state))]
async fn metrics_handler(State(state): State<MetricsState>) -> impl IntoResponse {
    debug!("Handling metrics request");

    let metrics = state.handle.render();

    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(metrics))
        .unwrap_or_else(|_| {
            error!("Failed to build metrics response");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Failed to render metrics"))
                .unwrap()
        })
}

/// Middleware layer that tracks HTTP request metrics.
///
/// This is a tower layer that can be applied to routes to automatically
/// track request counts and latencies.
#[derive(Debug, Clone)]
pub struct MetricsLayer;

impl MetricsLayer {
    /// Create a new metrics layer.
    pub fn new() -> Self {
        Self
    }
}

impl Default for MetricsLayer {
    fn default() -> Self {
        Self::new()
    }
}

/// A service wrapper that tracks metrics.
#[derive(Debug, Clone)]
pub struct MetricsService<S> {
    inner: S,
}

impl<S, ReqBody, ResBody> tower::Service<axum::extract::Request<ReqBody>> for MetricsService<S>
where
    S: tower::Service<
            axum::extract::Request<ReqBody>,
            Response = axum::response::Response<ResBody>,
        > + Clone
        + Send
        + 'static,
    S::Error: Into<axum::BoxError>,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: axum::extract::Request<ReqBody>) -> Self::Future {
        let method = req.method().to_string();
        let path = req.uri().path().to_string();

        let start = std::time::Instant::now();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let response = inner.call(req).await?;
            let status = response.status().as_u16().to_string();
            let duration = start.elapsed();

            // Record metrics
            openrustclaw_observability::metrics::record_request(&method, &path, &status);
            openrustclaw_observability::metrics::record_request_duration(
                &method,
                &path,
                duration.as_secs_f64(),
            );

            Ok(response)
        })
    }
}

impl<S> tower::Layer<S> for MetricsLayer {
    type Service = MetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricsService { inner }
    }
}

/// Create a middleware layer for automatic request metrics tracking.
pub fn metrics_middleware() -> MetricsLayer {
    MetricsLayer::new()
}

/// Render current metrics as a string (useful for debugging).
pub fn render_metrics(handle: &PrometheusHandle) -> String {
    handle.render()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_state() {
        let builder = PrometheusBuilder::new();
        let recorder = builder.build_recorder();
        let handle = Arc::new(recorder.handle());

        let _state = MetricsState::new(handle.clone());
        assert!(Arc::strong_count(&handle) > 1);
    }

    // Note: We can't easily test install_metrics() because it sets a global recorder
    // which would conflict with other tests.

    #[tokio::test]
    async fn test_metrics_layer_service() {
        // This is a simplified test - in reality you'd test with an actual service
        // Just verify the layer can be created
        let _layer = MetricsLayer::new();
    }
}

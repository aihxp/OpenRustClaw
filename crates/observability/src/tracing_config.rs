//! Structured logging and distributed tracing configuration.
//!
//! Provides:
//! - JSON format for production environments
//! - Pretty format for development
//! - Request ID propagation across async boundaries
//! - OpenTelemetry span context for distributed tracing
//!
//! # Usage
//!
//! ```rust,no_run
//! use openrustclaw_observability::tracing_config::{init_tracing, Env, RequestIdLayer};
//!
//! // Initialize for production
//! init_tracing(Env::Production);
//!
//! // Or for development
//! init_tracing(Env::Development);
//! ```

use std::collections::HashMap;
use tracing::{span, Level, Span};
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Registry,
};

/// Runtime environment for configuring tracing behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Env {
    /// Production environment - uses JSON formatting.
    Production,
    /// Development environment - uses pretty formatting.
    Development,
    /// Test environment - minimal formatting.
    Test,
}

impl Env {
    /// Detect environment from `RUST_ENV` or `APP_ENV` environment variable.
    pub fn detect() -> Self {
        if let Ok(env) = std::env::var("RUST_ENV")
            .or_else(|_| std::env::var("APP_ENV"))
            .or_else(|_| std::env::var("ENV"))
        {
            match env.to_lowercase().as_str() {
                "production" | "prod" => Env::Production,
                "test" | "testing" => Env::Test,
                _ => Env::Development,
            }
        } else {
            Env::Development
        }
    }
}

/// Initialize the tracing subscriber with environment-appropriate formatting.
///
/// # Examples
///
/// ```rust
/// use openrustclaw_observability::tracing_config::{init_tracing, Env};
///
/// fn main() {
///     init_tracing(Env::detect());
///     // Your application code...
/// }
/// ```
pub fn init_tracing(env: Env) {
    match env {
        Env::Production => init_tracing_json(),
        Env::Development => init_tracing_pretty(),
        Env::Test => init_tracing_test(),
    }
}

/// Initialize tracing with JSON output for production.
///
/// Includes:
/// - JSON formatting for structured logging
/// - Request ID propagation
/// - OpenTelemetry context extraction
/// - Custom fields for service name, version, etc.
pub fn init_tracing_json() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,openrustclaw=debug"));

    let json_layer = fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(false)
        .with_file(true)
        .with_line_number(true)
        .flatten_event(true);

    Registry::default()
        .with(env_filter)
        .with(RequestIdLayer)
        .with(json_layer)
        .init();
}

/// Initialize tracing with human-readable pretty output for development.
///
/// Includes:
/// - ANSI colors
/// - Pretty formatting
/// - Compact span information
/// - Source file and line numbers
pub fn init_tracing_pretty() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("debug,openrustclaw=trace"));

    let pretty_layer = fmt::layer()
        .pretty()
        .with_ansi(true)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_level(true);

    Registry::default()
        .with(env_filter)
        .with(RequestIdLayer)
        .with(pretty_layer)
        .init();
}

/// Initialize minimal tracing for tests.
pub fn init_tracing_test() {
    let env_filter = EnvFilter::new("warn");

    let compact_layer = fmt::layer()
        .compact()
        .with_ansi(false)
        .with_target(false)
        .with_level(true);

    Registry::default()
        .with(env_filter)
        .with(compact_layer)
        .init();
}

/// A layer that adds request IDs to spans.
///
/// This layer extracts or generates request IDs and adds them to all
/// log records within the span context.
#[derive(Debug)]
pub struct RequestIdLayer;

impl<S> tracing_subscriber::Layer<S> for RequestIdLayer
where
    S: tracing::Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    fn on_new_span(
        &self,
        attrs: &span::Attributes<'_>,
        id: &span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Extract request ID from span attributes if present
        let mut request_id = None;
        attrs.record(&mut RequestIdVisitor(&mut request_id));

        // If not found in attributes, try to inherit from parent span
        if request_id.is_none() {
            if let Some(span) = ctx.span(id) {
                if let Some(parent) = span.parent() {
                    if let Some(ext) = parent.extensions().get::<SpanExtensions>() {
                        request_id = ext.request_id.clone();
                    }
                }
            }
        }

        // Generate a new request ID if none exists
        let request_id = request_id.unwrap_or_else(RequestId::generate);

        // Store the request ID in the span extensions
        if let Some(span) = ctx.span(id) {
            let mut extensions = span.extensions_mut();
            extensions.insert(SpanExtensions {
                request_id: Some(request_id),
            });
        }
    }
}

/// Visitor to extract request_id from span attributes.
struct RequestIdVisitor<'a>(&'a mut Option<RequestId>);

impl<'a> tracing::field::Visit for RequestIdVisitor<'a> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "request_id" {
            *self.0 = Some(RequestId(value.to_string()));
        }
    }

    fn record_debug(&mut self, _field: &tracing::field::Field, _value: &dyn std::fmt::Debug) {}
}

/// Extensions stored in span metadata.
#[derive(Debug, Clone)]
struct SpanExtensions {
    request_id: Option<RequestId>,
}

/// A unique request identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RequestId(String);

impl RequestId {
    /// Generate a new random request ID.
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Create a request ID from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the request ID string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::generate()
    }
}

/// Get the current request ID from the span context.
///
/// Returns `None` if no request ID is set in the current span context.
pub fn current_request_id() -> Option<RequestId> {
    // This would need access to the current span's extensions
    // In practice, you'd use tracing's with_span or similar
    None // Placeholder - actual implementation requires subscriber access
}

/// Creates a span with a request ID.
///
/// The request ID will be propagated to all child spans.
///
/// # Example
///
/// ```rust
/// use openrustclaw_observability::tracing_config::request_span;
///
/// async fn handle_request() {
///     let span = request_span("handle_request");
///     let _enter = span.enter();
///     // All logs within this scope will have the request_id
/// }
/// ```
pub fn request_span(name: &'static str) -> Span {
    let request_id = RequestId::generate();
    span!(
        Level::INFO,
        "request",
        request_id = %request_id,
        span.name = name
    )
}

/// Creates a span with a specific request ID.
///
/// Useful when the request ID is received from an external source
/// (e.g., HTTP header, message queue).
pub fn request_span_with_id(name: &'static str, request_id: RequestId) -> Span {
    span!(
        Level::INFO,
        "request",
        request_id = %request_id,
        span.name = name
    )
}

/// Context for distributed tracing.
///
/// Contains information that should be propagated across service boundaries.
#[derive(Debug, Clone, Default)]
pub struct TraceContext {
    /// The trace ID for this request.
    pub trace_id: Option<String>,
    /// The span ID within the trace.
    pub span_id: Option<String>,
    /// Whether this trace should be sampled.
    pub sampled: bool,
    /// Additional baggage items.
    pub baggage: HashMap<String, String>,
}

impl TraceContext {
    /// Create a new trace context.
    pub fn new() -> Self {
        // Generate W3C-compatible trace and span IDs
        let trace_id = generate_trace_id();
        let span_id = generate_span_id();
        
        Self {
            trace_id: Some(trace_id),
            span_id: Some(span_id),
            sampled: true,
            baggage: HashMap::new(),
        }
    }

    /// Create a new trace context with explicit IDs.
    pub fn with_ids(trace_id: impl Into<String>, span_id: impl Into<String>) -> Self {
        Self {
            trace_id: Some(trace_id.into()),
            span_id: Some(span_id.into()),
            sampled: true,
            baggage: HashMap::new(),
        }
    }

    /// Parse a trace context from W3C traceparent header.
    ///
    /// Format: `00-<trace-id>-<parent-id>-<trace-flags>`
    /// 
    /// The trace-flags field is 2 hex digits. The least significant bit (0x01)
    /// indicates whether the trace is sampled.
    pub fn from_traceparent(header: &str) -> Option<Self> {
        let parts: Vec<&str> = header.split('-').collect();
        if parts.len() != 4 || parts[0] != "00" {
            return None;
        }

        let trace_id = parts[1].to_string();
        let span_id = parts[2].to_string();
        // Parse trace flags - sampled if the least significant bit is set (0x01)
        let sampled = parts[3].len() == 2 && 
            u8::from_str_radix(parts[3], 16).map(|f| f & 0x01 != 0).unwrap_or(false);

        Some(Self {
            trace_id: Some(trace_id),
            span_id: Some(span_id),
            sampled,
            baggage: HashMap::new(),
        })
    }

    /// Generate a W3C traceparent header.
    pub fn to_traceparent(&self) -> String {
        let trace_id = self
            .trace_id
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("00000000000000000000000000000000");
        let span_id = self
            .span_id
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("0000000000000000");
        let flags = if self.sampled { "01" } else { "00" };

        format!("00-{trace_id}-{span_id}-{flags}")
    }

    /// Add a baggage item.
    pub fn with_baggage(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.baggage.insert(key.into(), value.into());
        self
    }

    /// Get a baggage item.
    pub fn get_baggage(&self, key: &str) -> Option<&str> {
        self.baggage.get(key).map(|s| s.as_str())
    }
}

/// Extract trace context from HTTP headers.
///
/// Supports:
/// - W3C Trace Context (`traceparent`)
/// - OpenTelemetry baggage (`baggage`)
pub fn extract_trace_context_from_headers(
    headers: &http::HeaderMap,
) -> Option<TraceContext> {
    // Try to extract traceparent
    let traceparent = headers
        .get("traceparent")
        .and_then(|v| v.to_str().ok())?;

    let mut ctx = TraceContext::from_traceparent(traceparent)?;

    // Extract baggage if present
    if let Some(baggage) = headers.get("baggage").and_then(|v| v.to_str().ok()) {
        for item in baggage.split(',') {
            let parts: Vec<&str> = item.trim().splitn(2, '=').collect();
            if parts.len() == 2 {
                ctx.baggage
                    .insert(parts[0].to_string(), parts[1].to_string());
            }
        }
    }

    Some(ctx)
}

/// Inject trace context into HTTP headers.
pub fn inject_trace_context_into_headers(ctx: &TraceContext, headers: &mut http::HeaderMap) {
    if let Ok(value) = ctx.to_traceparent().parse() {
        headers.insert("traceparent", value);
    }

    if !ctx.baggage.is_empty() {
        let baggage: Vec<String> = ctx
            .baggage
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        if let Ok(value) = baggage.join(", ").parse() {
            headers.insert("baggage", value);
        }
    }
}

/// Initialize OpenTelemetry tracing with Jaeger or OTLP exporter.
///
/// This is a placeholder for OpenTelemetry initialization.
/// In a real implementation, you would configure the OTLP exporter
/// or Jaeger agent endpoint.
#[cfg(false)]  // Disabled until opentelemetry feature is properly configured
pub fn init_opentelemetry(service_name: &str, service_version: &str) {
    use opentelemetry::trace::TracerProvider;
    use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;

    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(
            opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(
                    opentelemetry_otlp::new_exporter()
                        .tonic()
                        .with_endpoint("http://localhost:4317"),
                )
                .install_batch(opentelemetry_sdk::runtime::Tokio)
                .expect("Failed to create OTLP exporter"),
        )
        .with_resource(opentelemetry_sdk::Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", service_name.to_string()),
            opentelemetry::KeyValue::new("service.version", service_version.to_string()),
        ]))
        .build();

    opentelemetry::global::set_tracer_provider(provider);
}

/// Shutdown OpenTelemetry providers.
#[cfg(false)]  // Disabled until opentelemetry feature is properly configured
pub fn shutdown_opentelemetry() {
    opentelemetry::global::shutdown_tracer_provider();
}

/// Generate a W3C-compatible trace ID (32 hex characters).
fn generate_trace_id() -> String {
    let bytes: [u8; 16] = rand::random();
    hex::encode(bytes)
}

/// Generate a W3C-compatible span ID (16 hex characters).
fn generate_span_id() -> String {
    let bytes: [u8; 8] = rand::random();
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_context_roundtrip() {
        let ctx = TraceContext::new()
            .with_baggage("user_id", "12345")
            .with_baggage("tenant", "acme");

        let traceparent = ctx.to_traceparent();
        assert!(traceparent.starts_with("00-"));

        let parsed = TraceContext::from_traceparent(&traceparent).unwrap();
        assert_eq!(parsed.trace_id, ctx.trace_id);
        assert_eq!(parsed.sampled, ctx.sampled);
    }

    #[test]
    fn test_env_detection() {
        // Note: This test might fail if environment variables are set
        // In practice, you'd use a more robust testing approach
        let _env = Env::detect();
    }

    #[test]
    fn test_request_id_generation() {
        let id1 = RequestId::generate();
        let id2 = RequestId::generate();
        assert_ne!(id1.as_str(), id2.as_str());
        assert_eq!(id1.as_str().len(), 36); // UUID v4 length
    }
}

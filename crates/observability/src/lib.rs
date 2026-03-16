//! LangSmith + OpenTelemetry observability for OpenRustClaw.
//!
//! Every graph run and important Rust tool action is traced into LangSmith
//! from the very first slice. Observability is built before feature breadth.

pub mod langsmith;
pub mod metrics;
pub mod tracing_setup;

pub use langsmith::LangSmithClient;
pub use metrics::MetricsCollector;

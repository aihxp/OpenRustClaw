//! Durable scheduler for OpenRustClaw.
//!
//! App-owned Rust scheduler that polls SQLite for due jobs and dispatches
//! them to LangGraph workflows via the Python sidecar. NO cron jobs.

pub mod heartbeat;
pub mod jobs;
pub mod persistence;
pub mod retry;
pub mod timezone;
pub mod triggers;
pub mod worker;

pub use heartbeat::{HeartbeatScheduler, HeartbeatTask, HeartbeatTaskBuilder};
pub use jobs::{Job, JobState};
pub use worker::SchedulerWorker;

/// Result type for scheduler operations
pub type Result<T> = std::result::Result<T, openrustclaw_core::error::SchedulerError>;

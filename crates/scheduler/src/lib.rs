//! Durable scheduler for OpenRustClaw.
//!
//! App-owned Rust scheduler that polls SQLite for due jobs, dispatches
//! Rust-native workflows, and optionally falls back to the sidecar for
//! bounded compatibility-only workflows. NO cron jobs.

pub mod eventing;
pub mod heartbeat;
pub mod jobs;
pub mod persistence;
pub mod retry;
pub mod timezone;
pub mod triggers;
pub mod worker;
pub mod workflow;

pub use eventing::{DurableEventBus, PublishedRuntimeEvent};
pub use heartbeat::{HeartbeatScheduler, HeartbeatTask, HeartbeatTaskBuilder};
pub use jobs::{Job, JobState};
pub use worker::SchedulerWorker;
pub use workflow::{
    ReminderSender, RustWorkflowDispatcher, WorkflowDefinition, WorkflowDispatchResult,
    WorkflowDispatcher, WorkflowTier,
};

/// Result type for scheduler operations
pub type Result<T> = std::result::Result<T, openrustclaw_core::error::SchedulerError>;

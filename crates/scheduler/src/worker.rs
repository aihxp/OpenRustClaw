//! Rust scheduler worker -- poll loop.
//!
//! Polls SQLite for due jobs, acquires leases, dispatches to LangGraph
//! via gRPC, handles success/failure/retry/dead-letter.

use std::time::Duration;

use chrono::Utc;
use tracing::{error, info, warn};

use crate::jobs::{Job, JobState};
use crate::retry;
use crate::triggers;

/// Configuration for the scheduler worker.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub poll_interval: Duration,
    pub lease_duration: Duration,
    pub base_retry_delay_secs: u64,
    pub max_retry_delay_secs: u64,
}

/// The scheduler worker that polls for due jobs.
pub struct SchedulerWorker {
    config: SchedulerConfig,
    worker_id: String,
}

impl SchedulerWorker {
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            config,
            worker_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Process a single due job.
    pub fn process_job_result(&self, job: &mut Job, success: bool, _error_msg: Option<&str>) {
        if success {
            job.consecutive_failures = 0;
            job.run_count += 1;
            job.last_run_at = Some(Utc::now());
            job.lease_owner = None;
            job.lease_expires_at = None;

            // Calculate next run
            job.next_run_at = triggers::calculate_next_run(&job.trigger, job.last_run_at);
            if job.next_run_at.is_none() {
                job.state = JobState::Completed;
            }

            info!(job_id = %job.id, "Job completed successfully");
        } else {
            job.consecutive_failures += 1;

            if retry::should_dead_letter(job.consecutive_failures, job.max_retries) {
                job.state = JobState::DeadLetter;
                error!(
                    job_id = %job.id,
                    failures = job.consecutive_failures,
                    "Job moved to dead-letter queue"
                );
            } else {
                // Schedule retry with backoff
                let delay = retry::backoff_delay(
                    job.consecutive_failures,
                    self.config.base_retry_delay_secs,
                    self.config.max_retry_delay_secs,
                );
                job.next_run_at = Some(
                    Utc::now()
                        + chrono::Duration::from_std(delay)
                            .unwrap_or(chrono::Duration::seconds(60)),
                );
                job.lease_owner = None;
                job.lease_expires_at = None;

                warn!(
                    job_id = %job.id,
                    failures = job.consecutive_failures,
                    retry_in = ?delay,
                    "Job failed, scheduling retry"
                );
            }
        }
    }

    /// Attempt to acquire a lease on a job.
    pub fn try_acquire_lease(&self, job: &mut Job) -> bool {
        if !job.is_lease_expired() {
            return false;
        }
        job.lease_owner = Some(self.worker_id.clone());
        job.lease_expires_at = Some(
            Utc::now()
                + chrono::Duration::from_std(self.config.lease_duration)
                    .unwrap_or(chrono::Duration::seconds(60)),
        );
        true
    }

    /// Get the worker ID.
    pub fn worker_id(&self) -> &str {
        &self.worker_id
    }

    /// Get the poll interval.
    pub fn poll_interval(&self) -> Duration {
        self.config.poll_interval
    }
}

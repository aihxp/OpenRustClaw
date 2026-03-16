//! Job types, idempotency keys, and lease semantics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::triggers::TriggerConfig;

/// A scheduled job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub workflow_id: String,
    pub trigger: TriggerConfig,
    pub idempotency_key: Option<String>,
    pub state: JobState,
    pub lease_owner: Option<String>,
    pub lease_expires_at: Option<DateTime<Utc>>,
    pub timezone: String,
    pub max_retries: u32,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub run_count: u32,
    pub consecutive_failures: u32,
    pub created_at: DateTime<Utc>,
}

/// Job state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Active,
    Paused,
    Completed,
    Failed,
    DeadLetter,
}

/// A completed job run record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRun {
    pub id: String,
    pub job_id: String,
    pub idempotency_key: String,
    pub status: RunStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result: Option<String>,
    pub langsmith_trace_id: Option<String>,
    pub retry_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Success,
    Failure,
    Timeout,
}

impl Job {
    /// Generate a new idempotency key for a run.
    pub fn generate_idempotency_key(&self) -> String {
        format!("{}:{}", self.id, Uuid::new_v4())
    }

    /// Check if the job is due to run.
    pub fn is_due(&self) -> bool {
        if self.state != JobState::Active {
            return false;
        }
        match &self.next_run_at {
            Some(next) => Utc::now() >= *next,
            None => false,
        }
    }

    /// Check if the lease has expired (can be claimed by another worker).
    pub fn is_lease_expired(&self) -> bool {
        match &self.lease_expires_at {
            Some(expires) => Utc::now() >= *expires,
            None => true, // No lease = available
        }
    }
}

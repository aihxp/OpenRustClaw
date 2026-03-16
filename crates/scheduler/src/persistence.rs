//! SQLite persistence helpers for the scheduler.

use crate::jobs::{JobState, RunStatus};

/// Convert a JobState to its database string representation.
pub fn job_state_to_str(state: &JobState) -> &'static str {
    match state {
        JobState::Active => "active",
        JobState::Paused => "paused",
        JobState::Completed => "completed",
        JobState::Failed => "failed",
        JobState::DeadLetter => "dead_letter",
    }
}

/// Parse a JobState from a database string.
pub fn str_to_job_state(s: &str) -> JobState {
    match s {
        "active" => JobState::Active,
        "paused" => JobState::Paused,
        "completed" => JobState::Completed,
        "failed" => JobState::Failed,
        "dead_letter" => JobState::DeadLetter,
        _ => JobState::Failed,
    }
}

/// Convert a RunStatus to its database string representation.
pub fn run_status_to_str(status: &RunStatus) -> &'static str {
    match status {
        RunStatus::Running => "running",
        RunStatus::Success => "success",
        RunStatus::Failure => "failure",
        RunStatus::Timeout => "timeout",
    }
}

//! SQLite persistence helpers for the scheduler.

use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use openrustclaw_core::error::SchedulerError;
use openrustclaw_db::SqlitePool;
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use crate::jobs::{Job, JobRun, JobState, RunStatus};
use crate::triggers::TriggerConfig;
use crate::Result;

/// A persisted job together with its workflow metadata payload.
#[derive(Debug, Clone)]
pub struct PersistedJob {
    pub job: Job,
    pub metadata: Value,
}

impl PersistedJob {
    /// Build the JSON workflow input passed to the sidecar.
    pub fn workflow_input(&self) -> Value {
        self.metadata
            .get("input")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}))
    }

    /// Build sidecar workflow metadata from the stored job metadata.
    pub fn workflow_metadata(&self) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("job_id".to_string(), self.job.id.clone());
        metadata.insert("job_name".to_string(), self.job.name.clone());
        metadata.insert("job_timezone".to_string(), self.job.timezone.clone());

        if let Some(object) = self.metadata.get("workflow_metadata").and_then(|v| v.as_object()) {
            for (key, value) in object {
                let rendered = value
                    .as_str()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| value.to_string());
                metadata.insert(key.clone(), rendered);
            }
        }

        metadata
    }

    /// Build typed configurable values for the sidecar workflow contract.
    pub fn workflow_configurable(&self) -> serde_json::Map<String, Value> {
        let mut configurable = serde_json::Map::new();
        configurable.insert("job_id".to_string(), Value::String(self.job.id.clone()));
        configurable.insert("job_name".to_string(), Value::String(self.job.name.clone()));
        configurable.insert(
            "job_timezone".to_string(),
            Value::String(self.job.timezone.clone()),
        );

        if let Some(object) = self.metadata.get("workflow_metadata").and_then(|v| v.as_object()) {
            for (key, value) in object {
                configurable.insert(key.clone(), value.clone());
            }
        }

        configurable
    }
}

fn parse_rfc3339(raw: &str, field: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| SchedulerError::InvalidTrigger(format!("invalid {field}: {e}")))
}

fn optional_rfc3339(raw: Option<String>, field: &str) -> Result<Option<DateTime<Utc>>> {
    match raw {
        Some(value) if value.trim().is_empty() => Ok(None),
        Some(value) => parse_rfc3339(&value, field).map(Some),
        None => Ok(None),
    }
}

fn parse_trigger(trigger_type: &str, trigger_config: &str) -> Result<TriggerConfig> {
    if let Ok(trigger) = serde_json::from_str::<TriggerConfig>(trigger_config) {
        return Ok(trigger);
    }

    let value: Value = serde_json::from_str(trigger_config).map_err(|e| {
        SchedulerError::InvalidTrigger(format!("invalid trigger JSON for {trigger_type}: {e}"))
    })?;

    match trigger_type {
        "interval" => {
            let interval_secs = value
                .get("interval_secs")
                .or_else(|| value.get("interval_seconds"))
                .and_then(|v| v.as_u64())
                .ok_or_else(|| {
                    SchedulerError::InvalidTrigger(
                        "interval trigger requires interval_secs".to_string(),
                    )
                })?;
            Ok(TriggerConfig::Interval { interval_secs })
        }
        "event" => {
            let event_name = value
                .get("event_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    SchedulerError::InvalidTrigger(
                        "event trigger requires event_name".to_string(),
                    )
                })?;
            Ok(TriggerConfig::Event {
                event_name: event_name.to_string(),
            })
        }
        "webhook" => {
            let webhook_url = value
                .get("webhook_url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    SchedulerError::InvalidTrigger(
                        "webhook trigger requires webhook_url".to_string(),
                    )
                })?;
            Ok(TriggerConfig::Webhook {
                webhook_url: webhook_url.to_string(),
            })
        }
        "dependency" => {
            let depends_on = value
                .get("depends_on")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    SchedulerError::InvalidTrigger(
                        "dependency trigger requires depends_on".to_string(),
                    )
                })?
                .iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect();
            Ok(TriggerConfig::Dependency { depends_on })
        }
        "absolute" => {
            let run_at = value
                .get("run_at")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    SchedulerError::InvalidTrigger("absolute trigger requires run_at".to_string())
                })?;
            Ok(TriggerConfig::Absolute {
                run_at: parse_rfc3339(run_at, "run_at")?,
            })
        }
        other => Err(SchedulerError::InvalidTrigger(format!(
            "unknown trigger_type: {other}"
        ))),
    }
}

fn row_to_persisted_job(row: sqlx::sqlite::SqliteRow) -> Result<PersistedJob> {
    let id: String = row.get("id");
    let trigger_type: String = row.get("trigger_type");
    let trigger_config: String = row.get("trigger_config");
    let metadata_raw: String = row.get("metadata");
    let metadata: Value = serde_json::from_str(&metadata_raw).map_err(|e| {
        SchedulerError::InvalidTrigger(format!("invalid job metadata for {id}: {e}"))
    })?;

    let job = Job {
        id,
        name: row.get("name"),
        description: row.try_get("description").ok(),
        workflow_id: row.get("workflow_id"),
        trigger: parse_trigger(&trigger_type, &trigger_config)?,
        idempotency_key: row.try_get("idempotency_key").ok(),
        state: str_to_job_state(row.get::<String, _>("state").as_str()),
        lease_owner: row.try_get("lease_owner").ok(),
        lease_expires_at: optional_rfc3339(row.try_get("lease_expires_at").ok(), "lease_expires_at")?,
        timezone: row.get("timezone"),
        max_retries: row.get::<i64, _>("max_retries") as u32,
        last_run_at: optional_rfc3339(row.try_get("last_run_at").ok(), "last_run_at")?,
        next_run_at: optional_rfc3339(row.try_get("next_run_at").ok(), "next_run_at")?,
        run_count: row.get::<i64, _>("run_count") as u32,
        consecutive_failures: row.get::<i64, _>("consecutive_failures") as u32,
        created_at: parse_rfc3339(&row.get::<String, _>("created_at"), "created_at")?,
    };

    Ok(PersistedJob { job, metadata })
}

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

/// Load due jobs that should be attempted by a worker.
pub async fn load_due_jobs(pool: &SqlitePool, limit: i64) -> Result<Vec<PersistedJob>> {
    let rows = sqlx::query(
        r#"
        SELECT
            id, name, description, workflow_id, trigger_type, trigger_config,
            idempotency_key, state, lease_owner, lease_expires_at, timezone,
            max_retries, last_run_at, next_run_at, run_count, consecutive_failures,
            metadata, created_at
        FROM scheduled_jobs
        WHERE state = 'active'
          AND next_run_at IS NOT NULL
          AND next_run_at <= ?
          AND (
              lease_owner IS NULL
              OR lease_expires_at IS NULL
              OR lease_expires_at <= ?
          )
        ORDER BY next_run_at ASC
        LIMIT ?
        "#,
    )
    .bind(Utc::now().to_rfc3339())
    .bind(Utc::now().to_rfc3339())
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| SchedulerError::WorkflowFailed(format!("failed to load due jobs: {e}")))?;

    rows.into_iter().map(row_to_persisted_job).collect()
}

/// Atomically acquire a lease on a job if it is still available.
pub async fn acquire_lease(
    pool: &SqlitePool,
    job_id: &str,
    worker_id: &str,
    lease_duration: Duration,
) -> Result<bool> {
    let expires_at = Utc::now()
        + chrono::Duration::from_std(lease_duration)
            .map_err(|e| SchedulerError::WorkflowFailed(format!("invalid lease duration: {e}")))?;

    let result = sqlx::query(
        r#"
        UPDATE scheduled_jobs
        SET lease_owner = ?, lease_expires_at = ?
        WHERE id = ?
          AND state = 'active'
          AND (
              lease_owner IS NULL
              OR lease_expires_at IS NULL
              OR lease_expires_at <= ?
          )
        "#,
    )
    .bind(worker_id)
    .bind(expires_at.to_rfc3339())
    .bind(job_id)
    .bind(Utc::now().to_rfc3339())
    .execute(pool)
    .await
    .map_err(|e| SchedulerError::WorkflowFailed(format!("failed to acquire lease: {e}")))?;

    Ok(result.rows_affected() == 1)
}

/// Insert a new running job run record.
pub async fn start_job_run(
    pool: &SqlitePool,
    job: &Job,
    idempotency_key: &str,
) -> Result<JobRun> {
    let run = JobRun {
        id: Uuid::new_v4().to_string(),
        job_id: job.id.clone(),
        idempotency_key: idempotency_key.to_string(),
        status: RunStatus::Running,
        started_at: Utc::now(),
        completed_at: None,
        result: None,
        langsmith_trace_id: None,
        retry_count: job.consecutive_failures,
    };

    sqlx::query(
        r#"
        INSERT INTO job_runs (
            id, job_id, idempotency_key, status, started_at, retry_count
        ) VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&run.id)
    .bind(&run.job_id)
    .bind(&run.idempotency_key)
    .bind(run_status_to_str(&run.status))
    .bind(run.started_at.to_rfc3339())
    .bind(run.retry_count as i64)
    .execute(pool)
    .await
    .map_err(|e| SchedulerError::WorkflowFailed(format!("failed to insert job run: {e}")))?;

    Ok(run)
}

/// Complete a job run and persist its result.
pub async fn complete_job_run(
    pool: &SqlitePool,
    run_id: &str,
    status: RunStatus,
    result: Option<&str>,
    trace_id: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE job_runs
        SET status = ?, completed_at = ?, result = ?, langsmith_trace_id = ?
        WHERE id = ?
        "#,
    )
    .bind(run_status_to_str(&status))
    .bind(Utc::now().to_rfc3339())
    .bind(result)
    .bind(trace_id)
    .bind(run_id)
    .execute(pool)
    .await
    .map_err(|e| SchedulerError::WorkflowFailed(format!("failed to complete job run: {e}")))?;

    Ok(())
}

/// Persist the latest in-memory job state back into SQLite.
pub async fn persist_job(pool: &SqlitePool, job: &Job) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE scheduled_jobs
        SET state = ?, lease_owner = ?, lease_expires_at = ?, last_run_at = ?,
            next_run_at = ?, run_count = ?, consecutive_failures = ?
        WHERE id = ?
        "#,
    )
    .bind(job_state_to_str(&job.state))
    .bind(job.lease_owner.as_deref())
    .bind(job.lease_expires_at.map(|dt| dt.to_rfc3339()))
    .bind(job.last_run_at.map(|dt| dt.to_rfc3339()))
    .bind(job.next_run_at.map(|dt| dt.to_rfc3339()))
    .bind(job.run_count as i64)
    .bind(job.consecutive_failures as i64)
    .bind(&job.id)
    .execute(pool)
    .await
    .map_err(|e| SchedulerError::WorkflowFailed(format!("failed to persist job: {e}")))?;

    Ok(())
}

/// Push a permanently failed job into the dead-letter queue.
pub async fn insert_dead_letter(
    pool: &SqlitePool,
    job: &Job,
    last_error: &str,
    original_payload: Option<&Value>,
) -> Result<()> {
    let payload = original_payload
        .map(|value| value.to_string())
        .unwrap_or_else(|| "{}".to_string());

    sqlx::query(
        r#"
        INSERT INTO dead_letter_queue (
            id, job_id, last_error, failed_at, retry_count, original_payload
        ) VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&job.id)
    .bind(last_error)
    .bind(Utc::now().to_rfc3339())
    .bind(job.consecutive_failures as i64)
    .bind(payload)
    .execute(pool)
    .await
    .map_err(|e| SchedulerError::WorkflowFailed(format!("failed to insert dead letter: {e}")))?;

    Ok(())
}

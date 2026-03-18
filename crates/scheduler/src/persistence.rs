//! SQLite persistence helpers for the scheduler.

use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use openrustclaw_core::error::SchedulerError;
use openrustclaw_db::SqlitePool;
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use crate::Result;
use crate::jobs::{Job, JobRun, JobState, RunStatus};
use crate::triggers::TriggerConfig;

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

        if let Some(object) = self
            .metadata
            .get("workflow_metadata")
            .and_then(|v| v.as_object())
        {
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

        if let Some(object) = self
            .metadata
            .get("workflow_metadata")
            .and_then(|v| v.as_object())
        {
            for (key, value) in object {
                configurable.insert(key.clone(), value.clone());
            }
        }

        configurable
    }
}

/// A durable runtime event persisted before workflow dispatch.
#[derive(Debug, Clone)]
pub struct RuntimeEventRecord {
    pub id: String,
    pub event_name: String,
    pub event_type: String,
    pub session_id: Option<String>,
    pub payload: Value,
}

/// A queued event-triggered workflow dispatch joined with its target job.
#[derive(Debug, Clone)]
pub struct PersistedEventDispatch {
    pub dispatch_id: String,
    pub event_id: String,
    pub event_name: String,
    pub dispatch_created_at: DateTime<Utc>,
    pub attempts: u32,
    pub payload: Value,
    pub job: Job,
    pub metadata: Value,
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

fn parse_datetime(raw: &str, field: &str) -> Result<DateTime<Utc>> {
    parse_rfc3339(raw, field).or_else(|_| {
        chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S")
            .map(|value| chrono::DateTime::<Utc>::from_naive_utc_and_offset(value, Utc))
            .map_err(|e| SchedulerError::InvalidTrigger(format!("invalid {field}: {e}")))
    })
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
                    SchedulerError::InvalidTrigger("event trigger requires event_name".to_string())
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

fn hook_policy_allows(metadata: &Value, event_name: &str, session_id: Option<&str>) -> bool {
    let Some(policy) = metadata.get("hook_policy") else {
        return true;
    };

    if policy
        .get("enabled")
        .and_then(Value::as_bool)
        .is_some_and(|enabled| !enabled)
    {
        return false;
    }

    if matches!(
        policy.get("mode").and_then(Value::as_str),
        Some("disabled" | "off")
    ) {
        return false;
    }

    if policy
        .get("session_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && session_id.is_none()
    {
        return false;
    }

    if let Some(allowed_hooks) = policy.get("allowed_hooks").and_then(Value::as_array) {
        return allowed_hooks
            .iter()
            .filter_map(Value::as_str)
            .any(|hook| hook == event_name);
    }

    true
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
        lease_expires_at: optional_rfc3339(
            row.try_get("lease_expires_at").ok(),
            "lease_expires_at",
        )?,
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

fn row_to_persisted_event_dispatch(row: sqlx::sqlite::SqliteRow) -> Result<PersistedEventDispatch> {
    let dispatch_id: String = row.get("dispatch_id");
    let event_id: String = row.get("event_id");
    let event_name: String = row.get("event_name");
    let payload_raw: String = row.get("payload");
    let payload = serde_json::from_str(&payload_raw).map_err(|e| {
        SchedulerError::InvalidTrigger(format!(
            "invalid event dispatch payload for {dispatch_id}: {e}"
        ))
    })?;
    let metadata_raw: String = row.get("metadata");
    let metadata = serde_json::from_str(&metadata_raw).map_err(|e| {
        SchedulerError::InvalidTrigger(format!("invalid event job metadata for {dispatch_id}: {e}"))
    })?;

    Ok(PersistedEventDispatch {
        dispatch_id,
        event_id,
        event_name,
        dispatch_created_at: parse_datetime(
            &row.get::<String, _>("dispatch_created_at"),
            "dispatch_created_at",
        )?,
        attempts: row.get::<i64, _>("attempts") as u32,
        payload,
        job: Job {
            id: row.get("job_id"),
            name: row.get("name"),
            description: row.try_get("description").ok(),
            workflow_id: row.get("workflow_id"),
            trigger: parse_trigger(
                &row.get::<String, _>("trigger_type"),
                &row.get::<String, _>("trigger_config"),
            )?,
            idempotency_key: row.try_get("idempotency_key").ok(),
            state: str_to_job_state(row.get::<String, _>("state").as_str()),
            lease_owner: row.try_get("lease_owner").ok(),
            lease_expires_at: optional_rfc3339(
                row.try_get("lease_expires_at").ok(),
                "lease_expires_at",
            )?,
            timezone: row.get("timezone"),
            max_retries: row.get::<i64, _>("max_retries") as u32,
            last_run_at: optional_rfc3339(row.try_get("last_run_at").ok(), "last_run_at")?,
            next_run_at: optional_rfc3339(row.try_get("next_run_at").ok(), "next_run_at")?,
            run_count: row.get::<i64, _>("run_count") as u32,
            consecutive_failures: row.get::<i64, _>("consecutive_failures") as u32,
            created_at: parse_rfc3339(&row.get::<String, _>("created_at"), "created_at")?,
        },
        metadata,
    })
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
              disabled_until IS NULL
              OR disabled_until <= ?
          )
          AND (
              lease_owner IS NULL
              OR lease_expires_at IS NULL
              OR lease_expires_at <= ?
          )
        ORDER BY
          MAX(
              COALESCE(priority, 100) - MIN(CAST((julianday(?) - julianday(next_run_at)) * 24 * 60 / 15 AS INTEGER), 90),
              0
          ) ASC,
          next_run_at ASC
        LIMIT ?
        "#,
    )
    .bind(Utc::now().to_rfc3339())
    .bind(Utc::now().to_rfc3339())
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
pub async fn start_job_run(pool: &SqlitePool, job: &Job, idempotency_key: &str) -> Result<JobRun> {
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

/// Persist a workflow checkpoint for a Rust-native workflow run.
pub async fn save_workflow_checkpoint(
    pool: &SqlitePool,
    workflow_id: &str,
    thread_id: &str,
    step: i64,
    state: &Value,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO workflow_checkpoints (id, workflow_id, thread_id, state, step)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(workflow_id)
    .bind(thread_id)
    .bind(state.to_string())
    .bind(step)
    .execute(pool)
    .await
    .map_err(|e| {
        SchedulerError::WorkflowFailed(format!("failed to save workflow checkpoint: {e}"))
    })?;

    Ok(())
}

/// Persist a durable runtime event and materialize matching event-triggered jobs.
pub async fn queue_runtime_event(
    pool: &SqlitePool,
    event_name: &str,
    event_type: &str,
    session_id: Option<&str>,
    payload: &Value,
    dedupe_key: Option<&str>,
) -> Result<(RuntimeEventRecord, usize)> {
    let event_id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO runtime_events (id, event_name, event_type, session_id, payload, dedupe_key, status)
        VALUES (?, ?, ?, ?, ?, ?, 'pending')
        "#,
    )
    .bind(&event_id)
    .bind(event_name)
    .bind(event_type)
    .bind(session_id)
    .bind(payload.to_string())
    .bind(dedupe_key)
    .execute(pool)
    .await
    .map_err(|e| SchedulerError::WorkflowFailed(format!("failed to persist runtime event: {e}")))?;

    let event_jobs = sqlx::query(
        r#"
        SELECT
            id, name, description, workflow_id, trigger_type, trigger_config,
            idempotency_key, state, lease_owner, lease_expires_at, timezone,
            max_retries, last_run_at, next_run_at, run_count, consecutive_failures,
            metadata, created_at
        FROM scheduled_jobs
        WHERE state = 'active'
          AND trigger_type = 'event'
          AND (
              disabled_until IS NULL
              OR disabled_until <= ?
          )
        "#,
    )
    .bind(Utc::now().to_rfc3339())
    .fetch_all(pool)
    .await
    .map_err(|e| {
        SchedulerError::WorkflowFailed(format!("failed to load event-triggered jobs: {e}"))
    })?;

    let mut queued = 0_usize;
    for row in event_jobs {
        let persisted_job = row_to_persisted_job(row)?;
        match &persisted_job.job.trigger {
            TriggerConfig::Event {
                event_name: configured,
            } if configured == event_name
                && hook_policy_allows(&persisted_job.metadata, event_name, session_id) =>
            {
                let dispatch_id = Uuid::new_v4().to_string();
                let result = sqlx::query(
                    r#"
                    INSERT OR IGNORE INTO event_dispatch_queue (
                        id, event_id, job_id, state, next_attempt_at, payload
                    ) VALUES (?, ?, ?, 'pending', ?, ?)
                    "#,
                )
                .bind(dispatch_id)
                .bind(&event_id)
                .bind(&persisted_job.job.id)
                .bind(Utc::now().to_rfc3339())
                .bind(payload.to_string())
                .execute(pool)
                .await
                .map_err(|e| {
                    SchedulerError::WorkflowFailed(format!("failed to queue event dispatch: {e}"))
                })?;
                queued += result.rows_affected() as usize;
            }
            _ => {}
        }
    }

    Ok((
        RuntimeEventRecord {
            id: event_id,
            event_name: event_name.to_string(),
            event_type: event_type.to_string(),
            session_id: session_id.map(ToString::to_string),
            payload: payload.clone(),
        },
        queued,
    ))
}

/// Load due event-triggered dispatches that should be executed now.
pub async fn load_due_event_dispatches(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<PersistedEventDispatch>> {
    let rows = sqlx::query(
        r#"
        SELECT
            q.id AS dispatch_id,
            q.event_id,
            q.attempts,
            q.payload,
            q.created_at AS dispatch_created_at,
            e.event_name,
            j.id AS job_id,
            j.name,
            j.description,
            j.workflow_id,
            j.trigger_type,
            j.trigger_config,
            j.idempotency_key,
            j.state,
            j.lease_owner,
            j.lease_expires_at,
            j.timezone,
            j.max_retries,
            j.last_run_at,
            j.next_run_at,
            j.run_count,
            j.consecutive_failures,
            j.metadata,
            j.created_at
        FROM event_dispatch_queue q
        JOIN runtime_events e ON e.id = q.event_id
        JOIN scheduled_jobs j ON j.id = q.job_id
        WHERE q.state IN ('pending', 'leased')
          AND q.next_attempt_at <= ?
          AND (
              q.lease_owner IS NULL
              OR q.lease_expires_at IS NULL
              OR q.lease_expires_at <= ?
          )
          AND j.state = 'active'
        ORDER BY q.created_at ASC
        LIMIT ?
        "#,
    )
    .bind(Utc::now().to_rfc3339())
    .bind(Utc::now().to_rfc3339())
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        SchedulerError::WorkflowFailed(format!("failed to load due event dispatches: {e}"))
    })?;

    let mut dispatches: Vec<_> = rows
        .into_iter()
        .map(row_to_persisted_event_dispatch)
        .collect::<Result<Vec<_>>>()?;

    dispatches.sort_by(|left, right| {
        let left_priority = hook_priority(&left.metadata);
        let right_priority = hook_priority(&right.metadata);
        right_priority
            .cmp(&left_priority)
            .then_with(|| match hook_ordering(&left.metadata) {
                HookOrdering::Lifo => right.dispatch_created_at.cmp(&left.dispatch_created_at),
                HookOrdering::Fifo => left.dispatch_created_at.cmp(&right.dispatch_created_at),
            })
    });

    Ok(dispatches)
}

/// Claim a queued event dispatch lease.
pub async fn acquire_event_dispatch_lease(
    pool: &SqlitePool,
    dispatch_id: &str,
    worker_id: &str,
    lease_duration: Duration,
) -> Result<bool> {
    let expires_at = Utc::now()
        + chrono::Duration::from_std(lease_duration)
            .map_err(|e| SchedulerError::WorkflowFailed(format!("invalid lease duration: {e}")))?;

    let result = sqlx::query(
        r#"
        UPDATE event_dispatch_queue
        SET state = 'leased', lease_owner = ?, lease_expires_at = ?
        WHERE id = ?
          AND state IN ('pending', 'leased')
          AND (
              lease_owner IS NULL
              OR lease_expires_at IS NULL
              OR lease_expires_at <= ?
          )
        "#,
    )
    .bind(worker_id)
    .bind(expires_at.to_rfc3339())
    .bind(dispatch_id)
    .bind(Utc::now().to_rfc3339())
    .execute(pool)
    .await
    .map_err(|e| {
        SchedulerError::WorkflowFailed(format!("failed to acquire event dispatch lease: {e}"))
    })?;

    Ok(result.rows_affected() == 1)
}

async fn refresh_runtime_event_status(pool: &SqlitePool, event_id: &str) -> Result<()> {
    let (pending_count, failed_count): (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            SUM(CASE WHEN state IN ('pending', 'leased') THEN 1 ELSE 0 END) AS pending_count,
            SUM(CASE WHEN state IN ('failed', 'dead_letter') THEN 1 ELSE 0 END) AS failed_count
        FROM event_dispatch_queue
        WHERE event_id = ?
        "#,
    )
    .bind(event_id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        SchedulerError::WorkflowFailed(format!("failed to refresh runtime event status: {e}"))
    })?;

    let (status, processed_at) = if pending_count == 0 && failed_count > 0 {
        ("failed", Some(Utc::now().to_rfc3339()))
    } else if pending_count == 0 {
        ("processed", Some(Utc::now().to_rfc3339()))
    } else {
        ("pending", None)
    };

    sqlx::query("UPDATE runtime_events SET status = ?, processed_at = ? WHERE id = ?")
        .bind(status)
        .bind(processed_at)
        .bind(event_id)
        .execute(pool)
        .await
        .map_err(|e| {
            SchedulerError::WorkflowFailed(format!("failed to update runtime event status: {e}"))
        })?;

    Ok(())
}

/// Mark an event dispatch as completed or failed.
pub async fn finalize_event_dispatch(
    pool: &SqlitePool,
    dispatch_id: &str,
    event_id: &str,
    state: &str,
    attempts: u32,
    next_attempt_at: Option<DateTime<Utc>>,
    last_error: Option<&str>,
) -> Result<()> {
    let completed_at =
        matches!(state, "completed" | "dead_letter").then(|| Utc::now().to_rfc3339());
    sqlx::query(
        r#"
        UPDATE event_dispatch_queue
        SET state = ?, attempts = ?, next_attempt_at = ?, last_error = ?,
            lease_owner = NULL, lease_expires_at = NULL, completed_at = ?
        WHERE id = ?
        "#,
    )
    .bind(state)
    .bind(attempts as i64)
    .bind(
        next_attempt_at
            .map(|value| value.to_rfc3339())
            .unwrap_or_else(|| Utc::now().to_rfc3339()),
    )
    .bind(last_error)
    .bind(completed_at)
    .bind(dispatch_id)
    .execute(pool)
    .await
    .map_err(|e| {
        SchedulerError::WorkflowFailed(format!("failed to finalize event dispatch: {e}"))
    })?;

    refresh_runtime_event_status(pool, event_id).await
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HookOrdering {
    Fifo,
    Lifo,
}

fn hook_priority(metadata: &Value) -> i64 {
    metadata
        .get("hook_policy")
        .and_then(|policy| policy.get("priority"))
        .and_then(Value::as_i64)
        .or_else(|| {
            metadata
                .get("task")
                .and_then(|task| task.get("priority"))
                .and_then(Value::as_i64)
        })
        .unwrap_or(0)
}

fn hook_ordering(metadata: &Value) -> HookOrdering {
    match metadata
        .get("hook_policy")
        .and_then(|policy| policy.get("ordering"))
        .and_then(Value::as_str)
    {
        Some("lifo") => HookOrdering::Lifo,
        _ => HookOrdering::Fifo,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openrustclaw_db::{init_pool, run_migrations};

    #[tokio::test]
    async fn queue_runtime_event_honors_hook_policy() {
        let pool = init_pool("sqlite::memory:", 1).await.unwrap();
        run_migrations(&pool).await.unwrap();

        sqlx::query(
            r#"
            INSERT INTO scheduled_jobs (
                id, name, description, workflow_id, trigger_type, trigger_config,
                idempotency_key, state, timezone, max_retries, next_run_at, run_count,
                consecutive_failures, metadata, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, 'active', 'UTC', 3, NULL, 0, 0, ?, ?)
            "#,
        )
        .bind("hook-job")
        .bind("Session end hook")
        .bind("test")
        .bind("scheduler")
        .bind("event")
        .bind(r#"{"type":"event","event_name":"session.end"}"#)
        .bind("hook-job:stable")
        .bind(
            r#"{
                "hook_policy": {
                    "mode": "async",
                    "session_required": true,
                    "allowed_hooks": ["session.end"]
                }
            }"#,
        )
        .bind(Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();

        let (_, queued_without_session) = queue_runtime_event(
            &pool,
            "session.end",
            "session_lifecycle",
            None,
            &serde_json::json!({"reason": "missing_session"}),
            None,
        )
        .await
        .unwrap();
        assert_eq!(queued_without_session, 0);

        let (_, queued_with_session) = queue_runtime_event(
            &pool,
            "session.end",
            "session_lifecycle",
            Some("session-1"),
            &serde_json::json!({"reason": "normal"}),
            None,
        )
        .await
        .unwrap();
        assert_eq!(queued_with_session, 1);
    }

    #[tokio::test]
    async fn load_due_event_dispatches_prioritizes_high_priority_hooks() {
        let pool = init_pool("sqlite::memory:", 1).await.unwrap();
        run_migrations(&pool).await.unwrap();

        for (job_id, priority) in [("hook-low", 1), ("hook-high", 10)] {
            sqlx::query(
                r#"
                INSERT INTO scheduled_jobs (
                    id, name, description, workflow_id, trigger_type, trigger_config,
                    idempotency_key, state, timezone, max_retries, next_run_at, run_count,
                    consecutive_failures, metadata, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, 'active', 'UTC', 3, NULL, 0, 0, ?, ?)
                "#,
            )
            .bind(job_id)
            .bind(job_id)
            .bind("test")
            .bind("scheduler")
            .bind("event")
            .bind(r#"{"type":"event","event_name":"session.end"}"#)
            .bind(format!("{job_id}:stable"))
            .bind(
                serde_json::json!({
                    "hook_policy": {
                        "priority": priority,
                        "ordering": "fifo",
                    }
                })
                .to_string(),
            )
            .bind(Utc::now().to_rfc3339())
            .execute(&pool)
            .await
            .unwrap();
        }

        queue_runtime_event(
            &pool,
            "session.end",
            "session_lifecycle",
            Some("session-1"),
            &serde_json::json!({"reason": "normal"}),
            None,
        )
        .await
        .unwrap();

        let dispatches = load_due_event_dispatches(&pool, 10).await.unwrap();
        assert_eq!(dispatches.len(), 2);
        assert_eq!(dispatches[0].job.id, "hook-high");
        assert_eq!(dispatches[1].job.id, "hook-low");
    }
}

//! Rust scheduler worker -- poll loop.
//!
//! Polls SQLite for due jobs, acquires leases, dispatches to LangGraph
//! via gRPC, handles success/failure/retry/dead-letter.

use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use openrustclaw_db::SqlitePool;
use openrustclaw_langbridge::LangBridgeClient;
use serde_json::Value;
use tracing::{error, info, warn};

use crate::jobs::{Job, JobRun, JobState, RunStatus};
use crate::persistence::{
    PersistedJob, acquire_lease, complete_job_run, insert_dead_letter, load_due_jobs, persist_job,
    start_job_run,
};
use crate::retry;
use crate::triggers;
use crate::Result;

/// Configuration for the scheduler worker.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub poll_interval: Duration,
    pub lease_duration: Duration,
    pub base_retry_delay_secs: u64,
    pub max_retry_delay_secs: u64,
}

/// Outcome returned by a workflow dispatcher.
#[derive(Debug, Clone)]
pub struct WorkflowDispatchResult {
    pub status: String,
    pub output: Value,
    pub error: Option<String>,
    pub trace_id: Option<String>,
}

/// Pluggable execution backend for scheduled workflows.
#[async_trait]
pub trait WorkflowDispatcher {
    async fn dispatch(
        &mut self,
        workflow_id: &str,
        thread_id: &str,
        input: Value,
        metadata: std::collections::HashMap<String, String>,
    ) -> Result<WorkflowDispatchResult>;
}

#[async_trait]
impl WorkflowDispatcher for LangBridgeClient {
    async fn dispatch(
        &mut self,
        workflow_id: &str,
        thread_id: &str,
        input: Value,
        metadata: std::collections::HashMap<String, String>,
    ) -> Result<WorkflowDispatchResult> {
        let response = self
            .execute_workflow_with_metadata(workflow_id, thread_id, input, metadata)
            .await
            .map_err(|e| openrustclaw_core::error::SchedulerError::WorkflowFailed(e.to_string()))?;

        let output = serde_json::from_str(&response.output).unwrap_or_else(|_| serde_json::json!({}));
        let error = if response.error.is_empty() {
            None
        } else {
            Some(response.error)
        };
        let trace_id = if response.trace_id.is_empty() {
            None
        } else {
            Some(response.trace_id)
        };

        Ok(WorkflowDispatchResult {
            status: response.status,
            output,
            error,
            trace_id,
        })
    }
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
                job.lease_owner = None;
                job.lease_expires_at = None;
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

    /// Load and atomically claim due jobs from SQLite.
    pub async fn claim_due_jobs(
        &self,
        pool: &SqlitePool,
        limit: i64,
    ) -> Result<Vec<PersistedJob>> {
        let candidates = load_due_jobs(pool, limit).await?;
        let mut claimed = Vec::new();

        for candidate in candidates {
            if acquire_lease(
                pool,
                &candidate.job.id,
                &self.worker_id,
                self.config.lease_duration,
            )
            .await?
            {
                claimed.push(candidate);
            }
        }

        Ok(claimed)
    }

    /// Run all currently due jobs once.
    pub async fn run_due_jobs_once<D: WorkflowDispatcher + Send>(
        &self,
        pool: &SqlitePool,
        dispatcher: &mut D,
        limit: i64,
    ) -> Result<Vec<JobRun>> {
        let claimed_jobs = self.claim_due_jobs(pool, limit).await?;
        let mut completed_runs = Vec::new();

        for persisted in claimed_jobs {
            let run = self.execute_claimed_job(pool, dispatcher, persisted).await?;
            completed_runs.push(run);
        }

        Ok(completed_runs)
    }

    async fn execute_claimed_job<D: WorkflowDispatcher + Send>(
        &self,
        pool: &SqlitePool,
        dispatcher: &mut D,
        persisted: PersistedJob,
    ) -> Result<JobRun> {
        let workflow_input = persisted.workflow_input();
        let workflow_metadata = persisted.workflow_metadata();
        let original_metadata = persisted.metadata.clone();
        let mut job = persisted.job;
        let idempotency_key = job.generate_idempotency_key();
        let mut run = start_job_run(pool, &job, &idempotency_key).await?;
        let thread_id = format!("scheduled-job-{}", job.id);

        match dispatcher
            .dispatch(&job.workflow_id, &thread_id, workflow_input, workflow_metadata)
            .await
        {
            Ok(dispatch) => {
                let success = !matches!(dispatch.status.as_str(), "error" | "failed")
                    && dispatch.error.is_none();
                let result_json = dispatch.output.to_string();

                self.process_job_result(&mut job, success, dispatch.error.as_deref());
                let run_status = if success {
                    RunStatus::Success
                } else {
                    RunStatus::Failure
                };
                complete_job_run(
                    pool,
                    &run.id,
                    run_status.clone(),
                    Some(&result_json),
                    dispatch.trace_id.as_deref(),
                )
                .await?;
                persist_job(pool, &job).await?;

                if job.state == JobState::DeadLetter {
                    insert_dead_letter(
                        pool,
                        &job,
                        dispatch.error.as_deref().unwrap_or("workflow failed"),
                        Some(&original_metadata),
                    )
                    .await?;
                }

                run.status = run_status;
                run.completed_at = Some(Utc::now());
                run.result = Some(result_json);
                run.langsmith_trace_id = dispatch.trace_id;
                Ok(run)
            }
            Err(e) => {
                let error_message = e.to_string();
                self.process_job_result(&mut job, false, Some(&error_message));
                complete_job_run(pool, &run.id, RunStatus::Failure, Some(&error_message), None)
                    .await?;
                persist_job(pool, &job).await?;

                if job.state == JobState::DeadLetter {
                    insert_dead_letter(pool, &job, &error_message, Some(&original_metadata))
                        .await?;
                }

                run.status = RunStatus::Failure;
                run.completed_at = Some(Utc::now());
                run.result = Some(error_message);
                Ok(run)
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Duration as ChronoDuration;
    use openrustclaw_db::{init_pool, run_migrations};
    use tempfile::tempdir;

    struct StubDispatcher {
        success: Option<WorkflowDispatchResult>,
        error: Option<String>,
    }

    #[async_trait]
    impl WorkflowDispatcher for StubDispatcher {
        async fn dispatch(
            &mut self,
            _workflow_id: &str,
            _thread_id: &str,
            _input: Value,
            _metadata: std::collections::HashMap<String, String>,
        ) -> Result<WorkflowDispatchResult> {
            match (&self.success, &self.error) {
                (Some(result), None) => Ok(result.clone()),
                (_, Some(error)) => Err(openrustclaw_core::error::SchedulerError::WorkflowFailed(
                    error.clone(),
                )),
                _ => Err(openrustclaw_core::error::SchedulerError::WorkflowFailed(
                    "stub dispatcher misconfigured".to_string(),
                )),
            }
        }
    }

    async fn test_pool() -> SqlitePool {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("scheduler.db");
        let url = format!("sqlite://{}", db_path.display());
        let pool = init_pool(&url, 1).await.unwrap();
        run_migrations(&pool).await.unwrap();
        pool
    }

    async fn seed_job(pool: &SqlitePool, id: &str, max_retries: i64) {
        sqlx::query(
            r#"
            INSERT INTO scheduled_jobs (
                id, name, description, workflow_id, trigger_type, trigger_config,
                idempotency_key, state, timezone, max_retries, next_run_at, run_count,
                consecutive_failures, metadata, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, 'active', 'UTC', ?, ?, 0, 0, ?, ?)
            "#,
        )
        .bind(id)
        .bind("Test Job")
        .bind("scheduler test")
        .bind("agent")
        .bind("interval")
        .bind(r#"{"type":"interval","interval_secs":60}"#)
        .bind(format!("{id}:stable"))
        .bind(max_retries)
        .bind((Utc::now() - ChronoDuration::seconds(10)).to_rfc3339())
        .bind(r#"{"input":{"messages":[],"memory_context":"","tool_calls":[],"pending_approval":false,"approval_status":null,"output":null,"error":null}}"#)
        .bind(Utc::now().to_rfc3339())
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn run_due_jobs_once_records_success() {
        let pool = test_pool().await;
        seed_job(&pool, "job-success", 3).await;

        let worker = SchedulerWorker::new(SchedulerConfig {
            poll_interval: Duration::from_secs(30),
            lease_duration: Duration::from_secs(60),
            base_retry_delay_secs: 10,
            max_retry_delay_secs: 60,
        });
        let mut dispatcher = StubDispatcher {
            success: Some(WorkflowDispatchResult {
                status: "completed".to_string(),
                output: serde_json::json!({"status":"completed"}),
                error: None,
                trace_id: Some("trace-123".to_string()),
            }),
            error: None,
        };

        let runs = worker.run_due_jobs_once(&pool, &mut dispatcher, 10).await.unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].status, RunStatus::Success);

        let state: String = sqlx::query_scalar("SELECT state FROM scheduled_jobs WHERE id = ?")
            .bind("job-success")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(state, "active");

        let run_status: String = sqlx::query_scalar("SELECT status FROM job_runs WHERE job_id = ?")
            .bind("job-success")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(run_status, "success");
    }

    #[tokio::test]
    async fn run_due_jobs_once_records_dead_letter() {
        let pool = test_pool().await;
        seed_job(&pool, "job-dead-letter", 0).await;

        let worker = SchedulerWorker::new(SchedulerConfig {
            poll_interval: Duration::from_secs(30),
            lease_duration: Duration::from_secs(60),
            base_retry_delay_secs: 10,
            max_retry_delay_secs: 60,
        });
        let mut dispatcher = StubDispatcher {
            success: None,
            error: Some("boom".to_string()),
        };

        let runs = worker.run_due_jobs_once(&pool, &mut dispatcher, 10).await.unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].status, RunStatus::Failure);

        let state: String = sqlx::query_scalar("SELECT state FROM scheduled_jobs WHERE id = ?")
            .bind("job-dead-letter")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(state, "dead_letter");

        let dead_letter_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM dead_letter_queue WHERE job_id = ?")
                .bind("job-dead-letter")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(dead_letter_count, 1);
    }
}

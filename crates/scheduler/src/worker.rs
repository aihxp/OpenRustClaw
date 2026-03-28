//! Rust scheduler worker -- poll loop.
//!
//! Polls SQLite for due jobs and queued event dispatches, acquires leases,
//! dispatches through the Rust-native workflow runtime, and optionally falls
//! back to the sidecar for bounded compatibility-only workflows.

use std::time::{Duration, Instant};

use chrono::Utc;
use openrustclaw_db::SqlitePool;
use openrustclaw_langbridge::WorkflowInvocation;
use openrustclaw_observability::langsmith::{LangSmithClient, RunType, TraceRun};
use tracing::{error, info, warn};

use crate::Result;
use crate::eventing::DurableEventBus;
use crate::jobs::{Job, JobRun, JobState, RunStatus};
use crate::persistence::{
    PersistedEventDispatch, PersistedJob, acquire_event_dispatch_lease, acquire_lease,
    complete_job_run, finalize_event_dispatch, insert_dead_letter, load_due_event_dispatches,
    load_due_jobs, persist_job, start_job_run,
};
use crate::retry;
use crate::triggers;
use crate::workflow::WorkflowDispatcher;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HookFailureIsolation {
    Strict,
    Ignore,
}

#[derive(Debug, Clone, Copy)]
struct HookExecutionPolicy {
    timeout_ms: Option<u64>,
    failure_isolation: HookFailureIsolation,
}

impl HookExecutionPolicy {
    fn from_metadata(metadata: &serde_json::Value) -> Self {
        let hook_policy = metadata.get("hook_policy");
        let timeout_ms = hook_policy
            .and_then(|policy| policy.get("timeout_ms"))
            .and_then(serde_json::Value::as_u64)
            .filter(|value| *value > 0);
        let failure_isolation = match hook_policy
            .and_then(|policy| policy.get("failure_isolation"))
            .and_then(serde_json::Value::as_str)
        {
            Some("ignore") => HookFailureIsolation::Ignore,
            _ => HookFailureIsolation::Strict,
        };

        Self {
            timeout_ms,
            failure_isolation,
        }
    }
}

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
    langsmith: Option<LangSmithClient>,
    event_bus: Option<DurableEventBus>,
}

impl SchedulerWorker {
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            config,
            worker_id: uuid::Uuid::new_v4().to_string(),
            langsmith: None,
            event_bus: None,
        }
    }

    pub fn with_langsmith(mut self, client: LangSmithClient) -> Self {
        if client.is_enabled() {
            self.langsmith = Some(client);
        }
        self
    }

    pub fn with_event_bus(mut self, event_bus: DurableEventBus) -> Self {
        self.event_bus = Some(event_bus);
        self
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
            if job.next_run_at.is_none()
                && matches!(job.trigger, crate::triggers::TriggerConfig::Absolute { .. })
            {
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
    pub async fn claim_due_jobs(&self, pool: &SqlitePool, limit: i64) -> Result<Vec<PersistedJob>> {
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
            let run = self
                .execute_claimed_job(pool, dispatcher, persisted)
                .await?;
            completed_runs.push(run);
        }

        Ok(completed_runs)
    }

    /// Load and atomically claim due event-triggered dispatches from SQLite.
    pub async fn claim_due_event_dispatches(
        &self,
        pool: &SqlitePool,
        limit: i64,
    ) -> Result<Vec<PersistedEventDispatch>> {
        let candidates = load_due_event_dispatches(pool, limit).await?;
        let mut claimed = Vec::new();

        for candidate in candidates {
            if acquire_event_dispatch_lease(
                pool,
                &candidate.dispatch_id,
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

    /// Run all currently queued event dispatches once.
    pub async fn run_due_event_dispatches_once<D: WorkflowDispatcher + Send>(
        &self,
        pool: &SqlitePool,
        dispatcher: &mut D,
        limit: i64,
    ) -> Result<Vec<JobRun>> {
        let claimed_dispatches = self.claim_due_event_dispatches(pool, limit).await?;
        let mut completed_runs = Vec::new();

        for persisted in claimed_dispatches {
            let run = self
                .execute_claimed_event_dispatch(pool, dispatcher, persisted)
                .await?;
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
        let started_at = Instant::now();
        let workflow_input = persisted.workflow_input();
        let workflow_metadata = persisted.workflow_metadata();
        let workflow_configurable = persisted.workflow_configurable();
        let original_metadata = persisted.metadata.clone();
        let mut job = persisted.job;
        let idempotency_key = job.generate_idempotency_key();
        let mut run = start_job_run(pool, &job, &idempotency_key).await?;
        let thread_id = format!("scheduled-job-{}", job.id);
        let invocation = WorkflowInvocation::new(&job.workflow_id, &thread_id, workflow_input)
            .with_metadata(workflow_metadata)
            .with_configurable(workflow_configurable);
        if let Some(event_bus) = &self.event_bus
            && let Err(error) = event_bus
                .publish(openrustclaw_core::types::Event::SchedulerJobFired {
                    job_id: job.id.clone(),
                    job_name: job.name.clone(),
                })
                .await
        {
            warn!(error = %error, job_id = %job.id, "Failed to publish scheduler job fired event");
        }
        let mut scheduler_trace = self.build_scheduler_trace(&job, &thread_id, &idempotency_key);
        if let (Some(client), Some(trace)) = (&self.langsmith, scheduler_trace.as_ref())
            && let Err(error) = client.trace_run(trace).await
        {
            warn!(error = %error, job_id = %job.id, "Failed to create LangSmith scheduler trace");
        }

        match dispatcher.dispatch(invocation).await {
            Ok(dispatch) => {
                if dispatch.status == "deferred" {
                    openrustclaw_observability::metrics::record_job_execution(
                        &job.workflow_id,
                        "deferred",
                    );
                    openrustclaw_observability::metrics::record_job_duration(
                        &job.workflow_id,
                        started_at.elapsed().as_secs_f64(),
                    );
                    let next_run_at = extract_next_attempt_at(&dispatch.output)
                        .unwrap_or_else(|| Utc::now() + chrono::Duration::minutes(15));
                    job.state = JobState::Active;
                    job.lease_owner = None;
                    job.lease_expires_at = None;
                    job.next_run_at = Some(next_run_at);
                    persist_job(pool, &job).await?;
                    complete_job_run(
                        pool,
                        &run.id,
                        RunStatus::Success,
                        Some(&dispatch.output.to_string()),
                        effective_trace_id(scheduler_trace.as_ref(), dispatch.trace_id.clone())
                            .as_deref(),
                    )
                    .await?;
                    run.status = RunStatus::Success;
                    run.completed_at = Some(Utc::now());
                    run.result = Some(dispatch.output.to_string());
                    run.langsmith_trace_id =
                        effective_trace_id(scheduler_trace.as_ref(), dispatch.trace_id.clone());
                    return Ok(run);
                }

                let success = !matches!(dispatch.status.as_str(), "error" | "failed")
                    && dispatch.error.is_none();
                let result_json = dispatch.output.to_string();
                let persisted_trace_id =
                    effective_trace_id(scheduler_trace.as_ref(), dispatch.trace_id.clone());

                if let (Some(client), Some(trace)) = (&self.langsmith, scheduler_trace.as_mut()) {
                    trace.outputs = Some(dispatch.output.clone());
                    trace.error = dispatch.error.clone();
                    trace.end_time = Some(Utc::now());
                    trace.extra = Some(serde_json::json!({
                        "job_id": job.id,
                        "workflow_id": job.workflow_id,
                        "scheduler_worker_id": self.worker_id,
                        "sidecar_trace_id": dispatch.trace_id,
                        "status": dispatch.status,
                    }));
                    if let Err(error) = client.update_run(trace).await {
                        warn!(error = %error, job_id = %job.id, "Failed to update LangSmith scheduler trace");
                    }
                }

                self.process_job_result(&mut job, success, dispatch.error.as_deref());
                let run_status = if success {
                    RunStatus::Success
                } else {
                    RunStatus::Failure
                };
                openrustclaw_observability::metrics::record_job_execution(
                    &job.workflow_id,
                    if success { "success" } else { "failure" },
                );
                openrustclaw_observability::metrics::record_job_duration(
                    &job.workflow_id,
                    started_at.elapsed().as_secs_f64(),
                );
                complete_job_run(
                    pool,
                    &run.id,
                    run_status.clone(),
                    Some(&result_json),
                    persisted_trace_id.as_deref(),
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
                run.langsmith_trace_id = persisted_trace_id;
                Ok(run)
            }
            Err(e) => {
                let error_message = e.to_string();
                openrustclaw_observability::metrics::record_job_execution(
                    &job.workflow_id,
                    "failure",
                );
                openrustclaw_observability::metrics::record_job_duration(
                    &job.workflow_id,
                    started_at.elapsed().as_secs_f64(),
                );
                if let (Some(client), Some(trace)) = (&self.langsmith, scheduler_trace.as_mut()) {
                    trace.error = Some(error_message.clone());
                    trace.end_time = Some(Utc::now());
                    trace.extra = Some(serde_json::json!({
                        "job_id": job.id,
                        "workflow_id": job.workflow_id,
                        "scheduler_worker_id": self.worker_id,
                    }));
                    if let Err(error) = client.update_run(trace).await {
                        warn!(error = %error, job_id = %job.id, "Failed to update LangSmith scheduler trace");
                    }
                }
                self.process_job_result(&mut job, false, Some(&error_message));
                let persisted_trace_id = effective_trace_id(scheduler_trace.as_ref(), None);
                complete_job_run(
                    pool,
                    &run.id,
                    RunStatus::Failure,
                    Some(&error_message),
                    persisted_trace_id.as_deref(),
                )
                .await?;
                persist_job(pool, &job).await?;

                if job.state == JobState::DeadLetter {
                    insert_dead_letter(pool, &job, &error_message, Some(&original_metadata))
                        .await?;
                }

                run.status = RunStatus::Failure;
                run.completed_at = Some(Utc::now());
                run.result = Some(error_message);
                run.langsmith_trace_id = persisted_trace_id;
                Ok(run)
            }
        }
    }

    async fn execute_claimed_event_dispatch<D: WorkflowDispatcher + Send>(
        &self,
        pool: &SqlitePool,
        dispatcher: &mut D,
        persisted: PersistedEventDispatch,
    ) -> Result<JobRun> {
        let started_at = Instant::now();
        let mut job = persisted.job.clone();
        let original_metadata = persisted.metadata.clone();
        let workflow_input = merge_event_payload_into_job_input(&original_metadata, &persisted);
        let workflow_metadata = build_event_workflow_metadata(&job, &persisted);
        let workflow_configurable = build_event_workflow_configurable(&job, &persisted);
        let idempotency_key = format!("{}:{}", job.id, persisted.event_id);
        let mut run = start_job_run(pool, &job, &idempotency_key).await?;
        let thread_id = format!("event-dispatch-{}-{}", job.id, persisted.event_id);
        let invocation = WorkflowInvocation::new(&job.workflow_id, &thread_id, workflow_input)
            .with_metadata(workflow_metadata)
            .with_configurable(workflow_configurable);
        let hook_policy = HookExecutionPolicy::from_metadata(&original_metadata);
        let mut scheduler_trace = self.build_scheduler_trace(&job, &thread_id, &idempotency_key);

        let dispatch_result = if let Some(timeout_ms) = hook_policy.timeout_ms {
            tokio::time::timeout(
                Duration::from_millis(timeout_ms),
                dispatcher.dispatch(invocation),
            )
            .await
            .map_err(|_| {
                openrustclaw_core::error::SchedulerError::WorkflowFailed(format!(
                    "event hook timed out after {}ms",
                    timeout_ms
                ))
            })?
        } else {
            dispatcher.dispatch(invocation).await
        };

        match dispatch_result {
            Ok(dispatch) => {
                if dispatch.status == "deferred" {
                    openrustclaw_observability::metrics::record_job_execution(
                        &job.workflow_id,
                        "deferred",
                    );
                    openrustclaw_observability::metrics::record_job_duration(
                        &job.workflow_id,
                        started_at.elapsed().as_secs_f64(),
                    );
                    let next_attempt_at = extract_next_attempt_at(&dispatch.output)
                        .unwrap_or_else(|| Utc::now() + chrono::Duration::minutes(15));
                    finalize_event_dispatch(
                        pool,
                        &persisted.dispatch_id,
                        &persisted.event_id,
                        "pending",
                        persisted.attempts + 1,
                        Some(next_attempt_at),
                        None,
                    )
                    .await?;
                    complete_job_run(
                        pool,
                        &run.id,
                        RunStatus::Success,
                        Some(&dispatch.output.to_string()),
                        effective_trace_id(scheduler_trace.as_ref(), dispatch.trace_id.clone())
                            .as_deref(),
                    )
                    .await?;
                    run.status = RunStatus::Success;
                    run.completed_at = Some(Utc::now());
                    run.result = Some(dispatch.output.to_string());
                    run.langsmith_trace_id =
                        effective_trace_id(scheduler_trace.as_ref(), dispatch.trace_id.clone());
                    return Ok(run);
                }

                let success = !matches!(dispatch.status.as_str(), "error" | "failed")
                    && dispatch.error.is_none();
                let result_json = dispatch.output.to_string();
                let persisted_trace_id =
                    effective_trace_id(scheduler_trace.as_ref(), dispatch.trace_id.clone());

                if let (Some(client), Some(trace)) = (&self.langsmith, scheduler_trace.as_mut()) {
                    trace.outputs = Some(dispatch.output.clone());
                    trace.error = dispatch.error.clone();
                    trace.end_time = Some(Utc::now());
                    trace.extra = Some(serde_json::json!({
                        "job_id": job.id,
                        "workflow_id": job.workflow_id,
                        "event_id": persisted.event_id,
                        "event_name": persisted.event_name,
                        "scheduler_worker_id": self.worker_id,
                        "status": dispatch.status,
                    }));
                    if let Err(error) = client.update_run(trace).await {
                        warn!(error = %error, job_id = %job.id, "Failed to update LangSmith event dispatch trace");
                    }
                }

                if success {
                    openrustclaw_observability::metrics::record_job_execution(
                        &job.workflow_id,
                        "success",
                    );
                    openrustclaw_observability::metrics::record_job_duration(
                        &job.workflow_id,
                        started_at.elapsed().as_secs_f64(),
                    );
                    job.consecutive_failures = 0;
                    job.run_count += 1;
                    job.last_run_at = Some(Utc::now());
                    persist_job(pool, &job).await?;
                    finalize_event_dispatch(
                        pool,
                        &persisted.dispatch_id,
                        &persisted.event_id,
                        "completed",
                        persisted.attempts + 1,
                        None,
                        None,
                    )
                    .await?;
                    complete_job_run(
                        pool,
                        &run.id,
                        RunStatus::Success,
                        Some(&result_json),
                        persisted_trace_id.as_deref(),
                    )
                    .await?;
                    run.status = RunStatus::Success;
                } else {
                    openrustclaw_observability::metrics::record_job_execution(
                        &job.workflow_id,
                        "failure",
                    );
                    openrustclaw_observability::metrics::record_job_duration(
                        &job.workflow_id,
                        started_at.elapsed().as_secs_f64(),
                    );
                    let attempts = persisted.attempts + 1;
                    let error_message = dispatch
                        .error
                        .clone()
                        .unwrap_or_else(|| "event dispatch failed".to_string());
                    if hook_policy.failure_isolation == HookFailureIsolation::Ignore {
                        finalize_event_dispatch(
                            pool,
                            &persisted.dispatch_id,
                            &persisted.event_id,
                            "completed",
                            attempts,
                            None,
                            Some(&error_message),
                        )
                        .await?;
                        complete_job_run(
                            pool,
                            &run.id,
                            RunStatus::Success,
                            Some(&result_json),
                            persisted_trace_id.as_deref(),
                        )
                        .await?;
                        run.status = RunStatus::Success;
                        run.completed_at = Some(Utc::now());
                        run.result = Some(result_json);
                        run.langsmith_trace_id = persisted_trace_id;
                        return Ok(run);
                    }
                    if retry::should_dead_letter(attempts, job.max_retries) {
                        finalize_event_dispatch(
                            pool,
                            &persisted.dispatch_id,
                            &persisted.event_id,
                            "dead_letter",
                            attempts,
                            None,
                            Some(&error_message),
                        )
                        .await?;
                        insert_dead_letter(pool, &job, &error_message, Some(&original_metadata))
                            .await?;
                    } else {
                        let delay = retry::backoff_delay(
                            attempts,
                            self.config.base_retry_delay_secs,
                            self.config.max_retry_delay_secs,
                        );
                        finalize_event_dispatch(
                            pool,
                            &persisted.dispatch_id,
                            &persisted.event_id,
                            "pending",
                            attempts,
                            Some(
                                Utc::now()
                                    + chrono::Duration::from_std(delay)
                                        .unwrap_or(chrono::Duration::seconds(60)),
                            ),
                            Some(&error_message),
                        )
                        .await?;
                    }
                    complete_job_run(
                        pool,
                        &run.id,
                        RunStatus::Failure,
                        Some(&result_json),
                        persisted_trace_id.as_deref(),
                    )
                    .await?;
                    run.status = RunStatus::Failure;
                }

                run.completed_at = Some(Utc::now());
                run.result = Some(result_json);
                run.langsmith_trace_id = persisted_trace_id;
                Ok(run)
            }
            Err(error) => {
                openrustclaw_observability::metrics::record_job_execution(
                    &job.workflow_id,
                    "failure",
                );
                openrustclaw_observability::metrics::record_job_duration(
                    &job.workflow_id,
                    started_at.elapsed().as_secs_f64(),
                );
                let attempts = persisted.attempts + 1;
                let error_message = error.to_string();
                let persisted_trace_id = effective_trace_id(scheduler_trace.as_ref(), None);
                if hook_policy.failure_isolation == HookFailureIsolation::Ignore {
                    finalize_event_dispatch(
                        pool,
                        &persisted.dispatch_id,
                        &persisted.event_id,
                        "completed",
                        attempts,
                        None,
                        Some(&error_message),
                    )
                    .await?;
                    complete_job_run(
                        pool,
                        &run.id,
                        RunStatus::Success,
                        Some(&error_message),
                        persisted_trace_id.as_deref(),
                    )
                    .await?;
                    run.status = RunStatus::Success;
                    run.completed_at = Some(Utc::now());
                    run.result = Some(error_message);
                    run.langsmith_trace_id = persisted_trace_id;
                    return Ok(run);
                }
                if retry::should_dead_letter(attempts, job.max_retries) {
                    finalize_event_dispatch(
                        pool,
                        &persisted.dispatch_id,
                        &persisted.event_id,
                        "dead_letter",
                        attempts,
                        None,
                        Some(&error_message),
                    )
                    .await?;
                    insert_dead_letter(pool, &job, &error_message, Some(&original_metadata))
                        .await?;
                } else {
                    let delay = retry::backoff_delay(
                        attempts,
                        self.config.base_retry_delay_secs,
                        self.config.max_retry_delay_secs,
                    );
                    finalize_event_dispatch(
                        pool,
                        &persisted.dispatch_id,
                        &persisted.event_id,
                        "pending",
                        attempts,
                        Some(
                            Utc::now()
                                + chrono::Duration::from_std(delay)
                                    .unwrap_or(chrono::Duration::seconds(60)),
                        ),
                        Some(&error_message),
                    )
                    .await?;
                }
                complete_job_run(
                    pool,
                    &run.id,
                    RunStatus::Failure,
                    Some(&error_message),
                    persisted_trace_id.as_deref(),
                )
                .await?;
                run.status = RunStatus::Failure;
                run.completed_at = Some(Utc::now());
                run.result = Some(error_message);
                run.langsmith_trace_id = persisted_trace_id;
                Ok(run)
            }
        }
    }

    fn build_scheduler_trace(
        &self,
        job: &Job,
        thread_id: &str,
        idempotency_key: &str,
    ) -> Option<TraceRun> {
        let client = self.langsmith.as_ref()?;
        Some(client.new_run(
            "scheduler_dispatch",
            RunType::Chain,
            serde_json::json!({
                "job_id": job.id,
                "job_name": job.name,
                "workflow_id": job.workflow_id,
                "thread_id": thread_id,
                "idempotency_key": idempotency_key,
                "trigger": job.trigger,
            }),
        ))
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

fn extract_next_attempt_at(output: &serde_json::Value) -> Option<chrono::DateTime<Utc>> {
    output
        .get("next_attempt_at")
        .and_then(serde_json::Value::as_str)
        .and_then(|raw| chrono::DateTime::parse_from_rfc3339(raw).ok())
        .map(|value| value.with_timezone(&Utc))
}

fn effective_trace_id(
    scheduler_trace: Option<&TraceRun>,
    sidecar_trace_id: Option<String>,
) -> Option<String> {
    sidecar_trace_id.or_else(|| scheduler_trace.map(|trace| trace.id.clone()))
}

fn merge_event_payload_into_job_input(
    original_metadata: &serde_json::Value,
    persisted: &PersistedEventDispatch,
) -> serde_json::Value {
    let mut input = original_metadata
        .get("input")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if !input.is_object() {
        input = serde_json::json!({ "input": input });
    }

    if let Some(object) = input.as_object_mut() {
        object.insert(
            "event".to_string(),
            serde_json::json!({
                "event_id": persisted.event_id,
                "event_name": persisted.event_name,
                "payload": persisted.payload,
            }),
        );
    }

    input
}

fn build_event_workflow_metadata(
    job: &Job,
    persisted: &PersistedEventDispatch,
) -> std::collections::HashMap<String, String> {
    let mut metadata = std::collections::HashMap::from([
        ("job_id".to_string(), job.id.clone()),
        ("job_name".to_string(), job.name.clone()),
        ("job_timezone".to_string(), job.timezone.clone()),
        ("event_id".to_string(), persisted.event_id.clone()),
        ("event_name".to_string(), persisted.event_name.clone()),
    ]);
    metadata.insert("dispatch_id".to_string(), persisted.dispatch_id.clone());
    metadata
}

fn build_event_workflow_configurable(
    job: &Job,
    persisted: &PersistedEventDispatch,
) -> serde_json::Map<String, serde_json::Value> {
    serde_json::Map::from_iter([
        (
            "job_id".to_string(),
            serde_json::Value::String(job.id.clone()),
        ),
        (
            "job_name".to_string(),
            serde_json::Value::String(job.name.clone()),
        ),
        (
            "job_timezone".to_string(),
            serde_json::Value::String(job.timezone.clone()),
        ),
        (
            "event_id".to_string(),
            serde_json::Value::String(persisted.event_id.clone()),
        ),
        (
            "event_name".to_string(),
            serde_json::Value::String(persisted.event_name.clone()),
        ),
        (
            "dispatch_id".to_string(),
            serde_json::Value::String(persisted.dispatch_id.clone()),
        ),
        ("event_payload".to_string(), persisted.payload.clone()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::WorkflowDispatchResult;
    use async_trait::async_trait;
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
            _invocation: WorkflowInvocation,
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

        let runs = worker
            .run_due_jobs_once(&pool, &mut dispatcher, 10)
            .await
            .unwrap();
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

    #[test]
    fn effective_trace_id_prefers_sidecar_trace() {
        let trace = TraceRun {
            id: "scheduler-trace".to_string(),
            name: "scheduler_dispatch".to_string(),
            run_type: RunType::Chain,
            parent_run_id: None,
            inputs: serde_json::json!({}),
            outputs: None,
            error: None,
            start_time: Utc::now(),
            end_time: None,
            extra: None,
            tags: None,
        };
        assert_eq!(
            effective_trace_id(Some(&trace), Some("sidecar-trace".to_string())),
            Some("sidecar-trace".to_string())
        );
        assert_eq!(
            effective_trace_id(Some(&trace), None),
            Some("scheduler-trace".to_string())
        );
        assert_eq!(effective_trace_id(None, None), None);
    }

    #[tokio::test]
    async fn persisted_job_configurable_preserves_typed_metadata() {
        let pool = test_pool().await;
        sqlx::query(
            r#"
            INSERT INTO scheduled_jobs (
                id, name, description, workflow_id, trigger_type, trigger_config,
                idempotency_key, state, timezone, max_retries, next_run_at, run_count,
                consecutive_failures, metadata, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, 'active', 'UTC', ?, ?, 0, 0, ?, ?)
            "#,
        )
        .bind("job-typed")
        .bind("Typed Metadata Job")
        .bind("scheduler test")
        .bind("scheduler")
        .bind("interval")
        .bind(r#"{"type":"interval","interval_secs":60}"#)
        .bind("job-typed:stable")
        .bind(3_i64)
        .bind((Utc::now() - ChronoDuration::seconds(10)).to_rfc3339())
        .bind(
            r#"{
                "input":{"job_id":"job-typed","job_type":"sync","payload":{}},
                "workflow_metadata":{"limit":25,"enabled":true,"labels":["nightly","critical"]}
            }"#,
        )
        .bind(Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();

        let persisted = load_due_jobs(&pool, 10).await.unwrap().remove(0);
        let configurable = persisted.workflow_configurable();
        assert_eq!(configurable.get("limit").unwrap(), &serde_json::json!(25));
        assert_eq!(
            configurable.get("enabled").unwrap(),
            &serde_json::json!(true)
        );
        assert_eq!(
            configurable.get("labels").unwrap(),
            &serde_json::json!(["nightly", "critical"])
        );
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

        let runs = worker
            .run_due_jobs_once(&pool, &mut dispatcher, 10)
            .await
            .unwrap();
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

    #[tokio::test]
    async fn run_due_jobs_once_reschedules_deferred_dispatch() {
        let pool = test_pool().await;
        seed_job(&pool, "job-deferred", 3).await;

        let worker = SchedulerWorker::new(SchedulerConfig {
            poll_interval: Duration::from_secs(30),
            lease_duration: Duration::from_secs(60),
            base_retry_delay_secs: 10,
            max_retry_delay_secs: 60,
        });
        let next_attempt_at = (Utc::now() + ChronoDuration::minutes(30)).to_rfc3339();
        let mut dispatcher = StubDispatcher {
            success: Some(WorkflowDispatchResult {
                status: "deferred".to_string(),
                output: serde_json::json!({"next_attempt_at": next_attempt_at}),
                error: None,
                trace_id: None,
            }),
            error: None,
        };

        let runs = worker
            .run_due_jobs_once(&pool, &mut dispatcher, 10)
            .await
            .unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].status, RunStatus::Success);

        let stored_next_run: String =
            sqlx::query_scalar("SELECT next_run_at FROM scheduled_jobs WHERE id = ?")
                .bind("job-deferred")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stored_next_run, next_attempt_at);
    }
}

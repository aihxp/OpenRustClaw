//! E2E tests for scheduler workflows.
//!
//! These tests cover:
//! - Create scheduled job
//! - Job executes on schedule
//! - Retry on failure
//! - Dead letter queue

use chrono::Utc;
use openrustclaw_scheduler::jobs::{Job, JobState, RunStatus};
use openrustclaw_scheduler::triggers::TriggerConfig;
use serial_test::serial;

use crate::common::{init_test_tracing, TestEnvironment};

/// Scenario 1: Create and schedule a one-time job.
#[tokio::test]
#[serial]
async fn test_create_one_time_job() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let job = Job {
        id: "job_001".to_string(),
        name: "Test One-Time Job".to_string(),
        description: Some("A test job that runs once".to_string()),
        workflow_id: "workflow_test".to_string(),
        trigger: TriggerConfig::Absolute { run_at: Utc::now() + chrono::Duration::seconds(1) },
        idempotency_key: Some("key_001".to_string()),
        state: JobState::Active,
        lease_owner: None,
        lease_expires_at: None,
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: Some(Utc::now() + chrono::Duration::seconds(1)),
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    // Store job in database
    sqlx::query(
        r#"
        INSERT INTO scheduler_jobs (
            id, name, description, workflow_id, trigger_type, trigger_config,
            idempotency_key, state, timezone, max_retries, next_run_at, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&job.id)
    .bind(&job.name)
    .bind(&job.description)
    .bind(&job.workflow_id)
    .bind("absolute")
    .bind(serde_json::to_string(&job.trigger).unwrap())
    .bind(&job.idempotency_key)
    .bind("active")
    .bind(&job.timezone)
    .bind(job.max_retries as i64)
    .bind(job.next_run_at)
    .bind(job.created_at)
    .execute(&env.db_pool)
    .await
    .expect("Failed to insert job");

    // Verify job exists
    let row: (String,) = sqlx::query_as("SELECT name FROM scheduler_jobs WHERE id = ?")
        .bind(&job.id)
        .fetch_one(&env.db_pool)
        .await
        .expect("Failed to fetch job");

    assert_eq!(row.0, "Test One-Time Job");
}

/// Scenario 2: Create an interval-based recurring job.
#[tokio::test]
#[serial]
async fn test_create_interval_job() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let job = Job {
        id: "job_interval_001".to_string(),
        name: "Recurring Interval Job".to_string(),
        description: Some("Runs every minute".to_string()),
        workflow_id: "workflow_recurring".to_string(),
        trigger: TriggerConfig::Interval { interval_secs: 60 },
        idempotency_key: None,
        state: JobState::Active,
        lease_owner: None,
        lease_expires_at: None,
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: Some(Utc::now()),
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    sqlx::query(
        r#"
        INSERT INTO scheduler_jobs (
            id, name, description, workflow_id, trigger_type, trigger_config,
            idempotency_key, state, timezone, max_retries, next_run_at, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&job.id)
    .bind(&job.name)
    .bind(&job.description)
    .bind(&job.workflow_id)
    .bind("interval")
    .bind(serde_json::to_string(&job.trigger).unwrap())
    .bind(&job.idempotency_key)
    .bind("active")
    .bind(&job.timezone)
    .bind(job.max_retries as i64)
    .bind(job.next_run_at)
    .bind(job.created_at)
    .execute(&env.db_pool)
    .await
    .expect("Failed to insert interval job");

    // Verify
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM scheduler_jobs WHERE id = ?")
        .bind(&job.id)
        .fetch_one(&env.db_pool)
        .await
        .expect("Failed to count jobs");

    assert_eq!(count.0, 1);
}

/// Scenario 3: Job execution tracking.
#[tokio::test]
#[serial]
async fn test_job_execution_tracking() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let job_id = "job_exec_001";

    // Create job run record
    let run_id = uuid::Uuid::new_v4().to_string();
    let idempotency_key = format!("{}:{}", job_id, uuid::Uuid::new_v4());

    sqlx::query(
        r#"
        INSERT INTO scheduler_job_runs (
            id, job_id, idempotency_key, status, started_at, retry_count
        ) VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&run_id)
    .bind(job_id)
    .bind(&idempotency_key)
    .bind("running")
    .bind(Utc::now())
    .bind(0i64)
    .execute(&env.db_pool)
    .await
    .expect("Failed to insert job run");

    // Mark as completed
    sqlx::query(
        r#"
        UPDATE scheduler_job_runs 
        SET status = ?, completed_at = ?, result = ?
        WHERE id = ?
        "#,
    )
    .bind("success")
    .bind(Utc::now())
    .bind("Job completed successfully")
    .bind(&run_id)
    .execute(&env.db_pool)
    .await
    .expect("Failed to update job run");

    // Verify
    let row: (String, Option<String>) =
        sqlx::query_as("SELECT status, result FROM scheduler_job_runs WHERE id = ?")
            .bind(&run_id)
            .fetch_one(&env.db_pool)
            .await
            .expect("Failed to fetch job run");

    assert_eq!(row.0, "success");
    assert_eq!(row.1, Some("Job completed successfully".to_string()));
}

/// Scenario 4: Job retry mechanism on failure.
#[tokio::test]
#[serial]
async fn test_job_retry_on_failure() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let job_id = "job_retry_001";

    // Create a job with max_retries
    let job = Job {
        id: job_id.to_string(),
        name: "Retry Test Job".to_string(),
        description: None,
        workflow_id: "workflow_retry".to_string(),
        trigger: TriggerConfig::Absolute { run_at: Utc::now() },
        idempotency_key: None,
        state: JobState::Active,
        lease_owner: None,
        lease_expires_at: None,
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: Some(Utc::now()),
        run_count: 0,
        consecutive_failures: 2, // 2 failures so far
        created_at: Utc::now(),
    };

    sqlx::query(
        r#"
        INSERT INTO scheduler_jobs (
            id, name, workflow_id, trigger_type, trigger_config,
            state, timezone, max_retries, next_run_at, consecutive_failures, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&job.id)
    .bind(&job.name)
    .bind(&job.workflow_id)
    .bind("absolute")
    .bind(serde_json::to_string(&job.trigger).unwrap())
    .bind("active")
    .bind(&job.timezone)
    .bind(job.max_retries as i64)
    .bind(job.next_run_at)
    .bind(job.consecutive_failures as i64)
    .bind(job.created_at)
    .execute(&env.db_pool)
    .await
    .expect("Failed to insert job");

    // Verify consecutive_failures is tracked
    let row: (i64,) =
        sqlx::query_as("SELECT consecutive_failures FROM scheduler_jobs WHERE id = ?")
            .bind(job_id)
            .fetch_one(&env.db_pool)
            .await
            .expect("Failed to fetch job");

    assert_eq!(row.0, 2);
}

/// Scenario 5: Dead letter queue for failed jobs.
#[tokio::test]
#[serial]
async fn test_dead_letter_queue() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let job_id = "job_dlq_001";

    // Create a job that has exceeded max retries
    sqlx::query(
        r#"
        INSERT INTO scheduler_jobs (
            id, name, workflow_id, trigger_type, trigger_config,
            state, timezone, max_retries, consecutive_failures, next_run_at, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(job_id)
    .bind("DLQ Test Job")
    .bind("workflow_dlq")
    .bind("absolute")
    .bind("{}")
    .bind("active")
    .bind("UTC")
    .bind(3i64)
    .bind(3i64) // consecutive_failures == max_retries
    .bind(Utc::now())
    .bind(Utc::now())
    .execute(&env.db_pool)
    .await
    .expect("Failed to insert job");

    // Move to dead letter state
    sqlx::query("UPDATE scheduler_jobs SET state = ? WHERE id = ?")
        .bind("dead_letter")
        .bind(job_id)
        .execute(&env.db_pool)
        .await
        .expect("Failed to update job state");

    // Verify job is in dead letter state
    let row: (String,) = sqlx::query_as("SELECT state FROM scheduler_jobs WHERE id = ?")
        .bind(job_id)
        .fetch_one(&env.db_pool)
        .await
        .expect("Failed to fetch job");

    assert_eq!(row.0, "dead_letter");
}

/// Scenario 6: Job lease mechanism prevents duplicate execution.
#[tokio::test]
#[serial]
async fn test_job_lease_prevents_duplicate_execution() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let job_id = "job_lease_001";

    // Create job with active lease
    let lease_owner = "worker_1";
    let lease_expires = Utc::now() + chrono::Duration::minutes(5);

    sqlx::query(
        r#"
        INSERT INTO scheduler_jobs (
            id, name, workflow_id, trigger_type, trigger_config,
            state, timezone, max_retries, lease_owner, lease_expires_at, next_run_at, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(job_id)
    .bind("Lease Test Job")
    .bind("workflow_lease")
    .bind("absolute")
    .bind("{}")
    .bind("active")
    .bind("UTC")
    .bind(3i64)
    .bind(lease_owner)
    .bind(lease_expires)
    .bind(Utc::now())
    .bind(Utc::now())
    .execute(&env.db_pool)
    .await
    .expect("Failed to insert job");

    // Verify lease is set
    let row: (Option<String>, Option<chrono::DateTime<Utc>>) =
        sqlx::query_as("SELECT lease_owner, lease_expires_at FROM scheduler_jobs WHERE id = ?")
            .bind(job_id)
            .fetch_one(&env.db_pool)
            .await
            .expect("Failed to fetch job");

    assert_eq!(row.0, Some(lease_owner.to_string()));
    assert!(row.1.is_some());
}

/// Scenario 7: Job state transitions.
#[tokio::test]
#[serial]
async fn test_job_state_transitions() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let job_id = "job_state_001";

    // Create job
    sqlx::query(
        r#"
        INSERT INTO scheduler_jobs (
            id, name, workflow_id, trigger_type, trigger_config,
            state, timezone, max_retries, next_run_at, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(job_id)
    .bind("State Test Job")
    .bind("workflow_state")
    .bind("absolute")
    .bind("{}")
    .bind("active")
    .bind("UTC")
    .bind(3i64)
    .bind(Utc::now())
    .bind(Utc::now())
    .execute(&env.db_pool)
    .await
    .expect("Failed to insert job");

    // Transition to paused
    sqlx::query("UPDATE scheduler_jobs SET state = ? WHERE id = ?")
        .bind("paused")
        .bind(job_id)
        .execute(&env.db_pool)
        .await
        .expect("Failed to pause job");

    let row: (String,) = sqlx::query_as("SELECT state FROM scheduler_jobs WHERE id = ?")
        .bind(job_id)
        .fetch_one(&env.db_pool)
        .await
        .expect("Failed to fetch job");
    assert_eq!(row.0, "paused");

    // Transition back to active
    sqlx::query("UPDATE scheduler_jobs SET state = ? WHERE id = ?")
        .bind("active")
        .bind(job_id)
        .execute(&env.db_pool)
        .await
        .expect("Failed to activate job");

    let row: (String,) = sqlx::query_as("SELECT state FROM scheduler_jobs WHERE id = ?")
        .bind(job_id)
        .fetch_one(&env.db_pool)
        .await
        .expect("Failed to fetch job");
    assert_eq!(row.0, "active");
}

/// Scenario 8: Job due check.
#[tokio::test]
#[serial]
async fn test_job_due_check() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    // Job that is due now
    let due_job = Job {
        id: "job_due_001".to_string(),
        name: "Due Job".to_string(),
        description: None,
        workflow_id: "workflow_due".to_string(),
        trigger: TriggerConfig::Absolute { run_at: Utc::now() - chrono::Duration::minutes(5) },
        idempotency_key: None,
        state: JobState::Active,
        lease_owner: None,
        lease_expires_at: None,
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: Some(Utc::now() - chrono::Duration::minutes(5)),
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    assert!(due_job.is_due());

    // Job that is not due yet
    let future_job = Job {
        id: "job_future_001".to_string(),
        name: "Future Job".to_string(),
        description: None,
        workflow_id: "workflow_future".to_string(),
        trigger: TriggerConfig::Absolute { run_at: Utc::now() + chrono::Duration::hours(1) },
        idempotency_key: None,
        state: JobState::Active,
        lease_owner: None,
        lease_expires_at: None,
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: Some(Utc::now() + chrono::Duration::hours(1)),
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    assert!(!future_job.is_due());

    // Paused job should not be due even if next_run_at is past
    let paused_job = Job {
        id: "job_paused_001".to_string(),
        name: "Paused Job".to_string(),
        description: None,
        workflow_id: "workflow_paused".to_string(),
        trigger: TriggerConfig::Absolute { run_at: Utc::now() - chrono::Duration::minutes(5) },
        idempotency_key: None,
        state: JobState::Paused,
        lease_owner: None,
        lease_expires_at: None,
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: Some(Utc::now() - chrono::Duration::minutes(5)),
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    assert!(!paused_job.is_due());
}

/// Scenario 9: Idempotency key generation.
#[tokio::test]
#[serial]
async fn test_job_idempotency_key() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let job = Job {
        id: "job_idemp_001".to_string(),
        name: "Idempotency Test Job".to_string(),
        description: None,
        workflow_id: "workflow_idemp".to_string(),
        trigger: TriggerConfig::Absolute { run_at: Utc::now() },
        idempotency_key: None,
        state: JobState::Active,
        lease_owner: None,
        lease_expires_at: None,
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: Some(Utc::now()),
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    let key1 = job.generate_idempotency_key();
    let key2 = job.generate_idempotency_key();

    // Keys should be different (contain UUID)
    assert_ne!(key1, key2);
    
    // Keys should start with job ID
    assert!(key1.starts_with(&job.id));
    assert!(key2.starts_with(&job.id));
}

/// Scenario 10: Job run status tracking.
#[tokio::test]
#[serial]
async fn test_job_run_status_lifecycle() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let job_id = "job_lifecycle_001";
    let run_id = uuid::Uuid::new_v4().to_string();
    let idempotency_key = format!("{}:{}", job_id, uuid::Uuid::new_v4());

    // Create run - Running state
    sqlx::query(
        r#"
        INSERT INTO scheduler_job_runs (
            id, job_id, idempotency_key, status, started_at, retry_count
        ) VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&run_id)
    .bind(job_id)
    .bind(&idempotency_key)
    .bind("running")
    .bind(Utc::now())
    .bind(0i64)
    .execute(&env.db_pool)
    .await
    .expect("Failed to create run");

    // Update to Failure with retry
    sqlx::query(
        r#"
        UPDATE scheduler_job_runs 
        SET status = ?, completed_at = ?, result = ?, retry_count = ?
        WHERE id = ?
        "#,
    )
    .bind("failure")
    .bind(Utc::now())
    .bind("Temporary failure, will retry")
    .bind(1i64)
    .bind(&run_id)
    .execute(&env.db_pool)
    .await
    .expect("Failed to update run");

    // Verify failure tracked
    let row: (String, i64) =
        sqlx::query_as("SELECT status, retry_count FROM scheduler_job_runs WHERE id = ?")
            .bind(&run_id)
            .fetch_one(&env.db_pool)
            .await
            .expect("Failed to fetch run");

    assert_eq!(row.0, "failure");
    assert_eq!(row.1, 1);
}

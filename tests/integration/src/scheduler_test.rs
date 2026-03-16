//! Scheduler integration tests.
//!
//! These tests verify:
//! - Job creation and execution
//! - Idempotency
//! - Retry logic
//! - Dead letter queue

use chrono::{Duration, Utc};
use openrustclaw_scheduler::jobs::{Job, JobRun, JobState, RunStatus};
use openrustclaw_scheduler::retry::{backoff_delay, should_dead_letter};
use openrustclaw_scheduler::triggers::{calculate_next_run, TriggerConfig};

use crate::common::init_test_tracing;

use openrustclaw_scheduler::worker::SchedulerConfig;

// ═════════════════════════════════════════════════════════════════════════════
// Job Tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn job_creation() {
    init_test_tracing();

    let trigger = TriggerConfig::Interval { interval_secs: 3600 };

    let job = Job {
        id: "job_123".to_string(),
        name: "Test Job".to_string(),
        description: Some("A test job".to_string()),
        workflow_id: "workflow_1".to_string(),
        trigger,
        idempotency_key: Some("idemp_key_1".to_string()),
        state: JobState::Active,
        lease_owner: None,
        lease_expires_at: None,
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: Some(Utc::now() + Duration::hours(1)),
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    assert_eq!(job.id, "job_123");
    assert_eq!(job.name, "Test Job");
    assert_eq!(job.state, JobState::Active);
}

#[test]
fn job_is_due_when_active_and_past_next_run() {
    init_test_tracing();

    let job = Job {
        id: "job_1".to_string(),
        name: "Due Job".to_string(),
        description: None,
        workflow_id: "wf_1".to_string(),
        trigger: TriggerConfig::Interval { interval_secs: 60 },
        idempotency_key: None,
        state: JobState::Active,
        lease_owner: None,
        lease_expires_at: None,
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: Some(Utc::now() - Duration::minutes(5)), // Past
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    assert!(job.is_due());
}

#[test]
fn job_not_due_when_paused() {
    init_test_tracing();

    let job = Job {
        id: "job_1".to_string(),
        name: "Paused Job".to_string(),
        description: None,
        workflow_id: "wf_1".to_string(),
        trigger: TriggerConfig::Interval { interval_secs: 60 },
        idempotency_key: None,
        state: JobState::Paused,
        lease_owner: None,
        lease_expires_at: None,
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: Some(Utc::now() - Duration::minutes(5)),
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    assert!(!job.is_due());
}

#[test]
fn job_not_due_when_future() {
    init_test_tracing();

    let job = Job {
        id: "job_1".to_string(),
        name: "Future Job".to_string(),
        description: None,
        workflow_id: "wf_1".to_string(),
        trigger: TriggerConfig::Interval { interval_secs: 60 },
        idempotency_key: None,
        state: JobState::Active,
        lease_owner: None,
        lease_expires_at: None,
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: Some(Utc::now() + Duration::hours(1)), // Future
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    assert!(!job.is_due());
}

#[test]
fn job_not_due_when_no_next_run() {
    init_test_tracing();

    let job = Job {
        id: "job_1".to_string(),
        name: "No Next Run".to_string(),
        description: None,
        workflow_id: "wf_1".to_string(),
        trigger: TriggerConfig::Interval { interval_secs: 60 },
        idempotency_key: None,
        state: JobState::Active,
        lease_owner: None,
        lease_expires_at: None,
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: None,
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    assert!(!job.is_due());
}

#[test]
fn job_lease_expired_when_none() {
    init_test_tracing();

    let job = Job {
        id: "job_1".to_string(),
        name: "No Lease".to_string(),
        description: None,
        workflow_id: "wf_1".to_string(),
        trigger: TriggerConfig::Interval { interval_secs: 60 },
        idempotency_key: None,
        state: JobState::Active,
        lease_owner: None,
        lease_expires_at: None,
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: Some(Utc::now() + Duration::hours(1)),
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    assert!(job.is_lease_expired());
}

#[test]
fn job_lease_expired_when_past() {
    init_test_tracing();

    let job = Job {
        id: "job_1".to_string(),
        name: "Expired Lease".to_string(),
        description: None,
        workflow_id: "wf_1".to_string(),
        trigger: TriggerConfig::Interval { interval_secs: 60 },
        idempotency_key: None,
        state: JobState::Active,
        lease_owner: Some("worker_1".to_string()),
        lease_expires_at: Some(Utc::now() - Duration::minutes(5)),
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: Some(Utc::now() + Duration::hours(1)),
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    assert!(job.is_lease_expired());
}

#[test]
fn job_lease_not_expired_when_future() {
    init_test_tracing();

    let job = Job {
        id: "job_1".to_string(),
        name: "Active Lease".to_string(),
        description: None,
        workflow_id: "wf_1".to_string(),
        trigger: TriggerConfig::Interval { interval_secs: 60 },
        idempotency_key: None,
        state: JobState::Active,
        lease_owner: Some("worker_1".to_string()),
        lease_expires_at: Some(Utc::now() + Duration::minutes(5)),
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: Some(Utc::now() + Duration::hours(1)),
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    assert!(!job.is_lease_expired());
}

#[test]
fn job_generate_idempotency_key() {
    init_test_tracing();

    let job = Job {
        id: "job_123".to_string(),
        name: "Test".to_string(),
        description: None,
        workflow_id: "wf_1".to_string(),
        trigger: TriggerConfig::Interval { interval_secs: 60 },
        idempotency_key: None,
        state: JobState::Active,
        lease_owner: None,
        lease_expires_at: None,
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: None,
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    let key1 = job.generate_idempotency_key();
    let key2 = job.generate_idempotency_key();

    assert!(key1.starts_with("job_123:"));
    assert!(key2.starts_with("job_123:"));
    assert_ne!(key1, key2); // Should be unique
}

// ═════════════════════════════════════════════════════════════════════════════
// Job State Tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn job_state_serialization() {
    init_test_tracing();

    let states = vec![
        JobState::Active,
        JobState::Paused,
        JobState::Completed,
        JobState::Failed,
        JobState::DeadLetter,
    ];

    for state in states {
        let json = serde_json::to_string(&state).unwrap();
        let back: JobState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, back);
    }
}

#[test]
fn run_status_serialization() {
    init_test_tracing();

    let statuses = vec![
        RunStatus::Running,
        RunStatus::Success,
        RunStatus::Failure,
        RunStatus::Timeout,
    ];

    for status in statuses {
        let json = serde_json::to_string(&status).unwrap();
        let back: RunStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, back);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Retry Logic Tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn backoff_delay_exponential_growth() {
    init_test_tracing();

    let base_delay_secs = 10;
    let max_delay_secs = 300;

    let delay1 = backoff_delay(1, base_delay_secs, max_delay_secs);
    let delay2 = backoff_delay(2, base_delay_secs, max_delay_secs);
    let delay3 = backoff_delay(3, base_delay_secs, max_delay_secs);

    assert_eq!(delay1.as_secs(), 20); // 10 * 2^1
    assert_eq!(delay2.as_secs(), 40); // 10 * 2^2
    assert_eq!(delay3.as_secs(), 80); // 10 * 2^3
}

#[test]
fn backoff_delay_respects_max() {
    init_test_tracing();

    let base_delay_secs = 100;
    let max_delay_secs = 150;

    let delay1 = backoff_delay(1, base_delay_secs, max_delay_secs);
    let delay2 = backoff_delay(2, base_delay_secs, max_delay_secs);

    assert_eq!(delay1.as_secs(), 150); // Would be 200, but capped at max
    assert_eq!(delay2.as_secs(), 150); // Would be 400, but capped at max
}

#[test]
fn should_dead_letter_at_max_retries() {
    init_test_tracing();

    assert!(!should_dead_letter(0, 3));
    assert!(!should_dead_letter(1, 3));
    assert!(!should_dead_letter(2, 3));
    assert!(should_dead_letter(3, 3));
    assert!(should_dead_letter(4, 3));
}

#[test]
fn should_dead_letter_zero_max_retries() {
    init_test_tracing();

    // If max_retries is 0, should immediately dead letter on first failure
    assert!(should_dead_letter(0, 0));
    assert!(should_dead_letter(1, 0));
}

// ═════════════════════════════════════════════════════════════════════════════
// Trigger Tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn trigger_config_interval() {
    init_test_tracing();

    let trigger = TriggerConfig::Interval { interval_secs: 300 };

    match trigger {
        TriggerConfig::Interval { interval_secs } => {
            assert_eq!(interval_secs, 300);
        }
        _ => panic!("Expected Interval trigger"),
    }
}

#[test]
fn trigger_config_event() {
    init_test_tracing();

    let trigger = TriggerConfig::Event {
        event_name: "user_signup".to_string(),
    };

    match trigger {
        TriggerConfig::Event { event_name } => {
            assert_eq!(event_name, "user_signup");
        }
        _ => panic!("Expected Event trigger"),
    }
}

#[test]
fn trigger_config_webhook() {
    init_test_tracing();

    let trigger = TriggerConfig::Webhook {
        webhook_url: "https://example.com/webhook".to_string(),
    };

    match trigger {
        TriggerConfig::Webhook { webhook_url } => {
            assert_eq!(webhook_url, "https://example.com/webhook");
        }
        _ => panic!("Expected Webhook trigger"),
    }
}

#[test]
fn trigger_config_dependency() {
    init_test_tracing();

    let trigger = TriggerConfig::Dependency {
        depends_on: vec!["job_1".to_string(), "job_2".to_string()],
    };

    match trigger {
        TriggerConfig::Dependency { depends_on } => {
            assert_eq!(depends_on, vec!["job_1", "job_2"]);
        }
        _ => panic!("Expected Dependency trigger"),
    }
}

#[test]
fn trigger_config_absolute() {
    init_test_tracing();

    let run_at = Utc::now() + Duration::hours(2);
    let trigger = TriggerConfig::Absolute { run_at };

    match trigger {
        TriggerConfig::Absolute { run_at: stored } => {
            assert_eq!(stored, run_at);
        }
        _ => panic!("Expected Absolute trigger"),
    }
}

#[test]
fn calculate_next_run_interval_first_run() {
    init_test_tracing();

    let trigger = TriggerConfig::Interval { interval_secs: 3600 };
    let next = calculate_next_run(&trigger, None);

    // First run should be approximately now + interval
    assert!(next.is_some());
    let next_time = next.unwrap();
    let now = Utc::now();
    assert!(next_time >= now);
    assert!(next_time <= now + Duration::seconds(3600));
}

#[test]
fn calculate_next_run_interval_subsequent_run() {
    init_test_tracing();

    let trigger = TriggerConfig::Interval { interval_secs: 3600 };
    let last_run = Utc::now() - Duration::minutes(30);
    let next = calculate_next_run(&trigger, Some(last_run));

    assert!(next.is_some());
    let next_time = next.unwrap();
    // Next run should be last_run + interval
    assert!(next_time >= last_run + Duration::seconds(3600) - Duration::seconds(1));
    assert!(next_time <= last_run + Duration::seconds(3600) + Duration::seconds(1));
}

#[test]
fn calculate_next_run_absolute_first_run() {
    init_test_tracing();

    let run_at = Utc::now() + Duration::days(1);
    let trigger = TriggerConfig::Absolute { run_at };
    let next = calculate_next_run(&trigger, None);

    assert_eq!(next, Some(run_at));
}

#[test]
fn calculate_next_run_absolute_completed() {
    init_test_tracing();

    let run_at = Utc::now() + Duration::days(1);
    let trigger = TriggerConfig::Absolute { run_at };
    let last_run = Utc::now();
    let next = calculate_next_run(&trigger, Some(last_run));

    // Absolute triggers fire once, so no next run after completion
    assert!(next.is_none());
}

#[test]
fn calculate_next_run_event_trigger() {
    init_test_tracing();

    let trigger = TriggerConfig::Event {
        event_name: "test".to_string(),
    };
    let next = calculate_next_run(&trigger, None);

    // Event triggers don't have scheduled times
    assert!(next.is_none());
}

#[test]
fn calculate_next_run_webhook_trigger() {
    init_test_tracing();

    let trigger = TriggerConfig::Webhook {
        webhook_url: "https://example.com".to_string(),
    };
    let next = calculate_next_run(&trigger, None);

    // Webhook triggers don't have scheduled times
    assert!(next.is_none());
}

#[test]
fn calculate_next_run_dependency_trigger() {
    init_test_tracing();

    let trigger = TriggerConfig::Dependency {
        depends_on: vec!["job_1".to_string()],
    };
    let next = calculate_next_run(&trigger, None);

    // Dependency triggers don't have scheduled times
    assert!(next.is_none());
}

// ═════════════════════════════════════════════════════════════════════════════
// JobRun Tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn job_run_creation() {
    init_test_tracing();

    let run = JobRun {
        id: "run_123".to_string(),
        job_id: "job_456".to_string(),
        idempotency_key: "idemp_789".to_string(),
        status: RunStatus::Running,
        started_at: Utc::now(),
        completed_at: None,
        result: None,
        langsmith_trace_id: None,
        retry_count: 0,
    };

    assert_eq!(run.id, "run_123");
    assert_eq!(run.job_id, "job_456");
    assert_eq!(run.status, RunStatus::Running);
    assert!(run.completed_at.is_none());
}

#[test]
fn job_run_completion() {
    init_test_tracing();

    let mut run = JobRun {
        id: "run_123".to_string(),
        job_id: "job_456".to_string(),
        idempotency_key: "idemp_789".to_string(),
        status: RunStatus::Running,
        started_at: Utc::now(),
        completed_at: None,
        result: None,
        langsmith_trace_id: None,
        retry_count: 0,
    };

    run.status = RunStatus::Success;
    run.completed_at = Some(Utc::now());
    run.result = Some("Job completed successfully".to_string());

    assert_eq!(run.status, RunStatus::Success);
    assert!(run.completed_at.is_some());
    assert!(run.result.is_some());
}

// ═════════════════════════════════════════════════════════════════════════════
// Scheduler Worker Tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn scheduler_worker_creation() {
    init_test_tracing();

    let config = SchedulerConfig {
        poll_interval: std::time::Duration::from_secs(30),
        lease_duration: std::time::Duration::from_secs(60),
        base_retry_delay_secs: 10,
        max_retry_delay_secs: 3600,
    };

    let worker = openrustclaw_scheduler::SchedulerWorker::new(config);
    
    // Worker is created with unique ID
    assert!(!worker.worker_id().is_empty());
    assert_eq!(worker.poll_interval(), std::time::Duration::from_secs(30));
}

#[test]
fn scheduler_worker_acquire_lease() {
    init_test_tracing();

    let config = SchedulerConfig {
        poll_interval: std::time::Duration::from_secs(30),
        lease_duration: std::time::Duration::from_secs(60),
        base_retry_delay_secs: 10,
        max_retry_delay_secs: 3600,
    };

    let worker = openrustclaw_scheduler::SchedulerWorker::new(config);
    
    let mut job = Job {
        id: "job_1".to_string(),
        name: "Test".to_string(),
        description: None,
        workflow_id: "wf_1".to_string(),
        trigger: TriggerConfig::Interval { interval_secs: 60 },
        idempotency_key: None,
        state: JobState::Active,
        lease_owner: None,
        lease_expires_at: None,
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: None,
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    // Should be able to acquire lease on unleased job
    assert!(worker.try_acquire_lease(&mut job));
    assert!(job.lease_owner.is_some());
    assert!(job.lease_expires_at.is_some());
    assert_eq!(job.lease_owner.as_ref().unwrap(), worker.worker_id());
}

#[test]
fn scheduler_worker_process_success() {
    init_test_tracing();

    let config = SchedulerConfig {
        poll_interval: std::time::Duration::from_secs(30),
        lease_duration: std::time::Duration::from_secs(60),
        base_retry_delay_secs: 10,
        max_retry_delay_secs: 3600,
    };

    let worker = openrustclaw_scheduler::SchedulerWorker::new(config);
    
    let mut job = Job {
        id: "job_1".to_string(),
        name: "Test".to_string(),
        description: None,
        workflow_id: "wf_1".to_string(),
        trigger: TriggerConfig::Interval { interval_secs: 60 },
        idempotency_key: None,
        state: JobState::Active,
        lease_owner: Some("worker_1".to_string()),
        lease_expires_at: Some(Utc::now() + Duration::minutes(5)),
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: Some(Utc::now()),
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    worker.process_job_result(&mut job, true, None);

    assert_eq!(job.consecutive_failures, 0);
    assert_eq!(job.run_count, 1);
    assert!(job.last_run_at.is_some());
    assert!(job.next_run_at.is_some());
    assert!(job.lease_owner.is_none());
}

#[test]
fn scheduler_worker_process_failure_with_retry() {
    init_test_tracing();

    let config = SchedulerConfig {
        poll_interval: std::time::Duration::from_secs(30),
        lease_duration: std::time::Duration::from_secs(60),
        base_retry_delay_secs: 10,
        max_retry_delay_secs: 3600,
    };

    let worker = openrustclaw_scheduler::SchedulerWorker::new(config);
    
    let mut job = Job {
        id: "job_1".to_string(),
        name: "Test".to_string(),
        description: None,
        workflow_id: "wf_1".to_string(),
        trigger: TriggerConfig::Interval { interval_secs: 60 },
        idempotency_key: None,
        state: JobState::Active,
        lease_owner: Some("worker_1".to_string()),
        lease_expires_at: Some(Utc::now() + Duration::minutes(5)),
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: Some(Utc::now()),
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    worker.process_job_result(&mut job, false, Some("Error message"));

    assert_eq!(job.consecutive_failures, 1);
    assert_eq!(job.state, JobState::Active); // Not dead letter yet
    assert!(job.next_run_at.is_some()); // Scheduled for retry
}

#[test]
fn scheduler_worker_process_failure_dead_letter() {
    init_test_tracing();

    let config = SchedulerConfig {
        poll_interval: std::time::Duration::from_secs(30),
        lease_duration: std::time::Duration::from_secs(60),
        base_retry_delay_secs: 10,
        max_retry_delay_secs: 3600,
    };

    let worker = openrustclaw_scheduler::SchedulerWorker::new(config);
    
    let mut job = Job {
        id: "job_1".to_string(),
        name: "Test".to_string(),
        description: None,
        workflow_id: "wf_1".to_string(),
        trigger: TriggerConfig::Interval { interval_secs: 60 },
        idempotency_key: None,
        state: JobState::Active,
        lease_owner: Some("worker_1".to_string()),
        lease_expires_at: Some(Utc::now() + Duration::minutes(5)),
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: Some(Utc::now()),
        run_count: 0,
        consecutive_failures: 3, // Already at max
        created_at: Utc::now(),
    };

    worker.process_job_result(&mut job, false, Some("Final error"));

    assert_eq!(job.consecutive_failures, 4);
    assert_eq!(job.state, JobState::DeadLetter);
}

// ═════════════════════════════════════════════════════════════════════════════
// Integration Tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn idempotency_key_uniqueness() {
    init_test_tracing();

    let job = Job {
        id: "job_1".to_string(),
        name: "Test".to_string(),
        description: None,
        workflow_id: "wf_1".to_string(),
        trigger: TriggerConfig::Interval { interval_secs: 60 },
        idempotency_key: None,
        state: JobState::Active,
        lease_owner: None,
        lease_expires_at: None,
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: None,
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    let keys: std::collections::HashSet<String> = (0..100)
        .map(|_| job.generate_idempotency_key())
        .collect();

    // All keys should be unique
    assert_eq!(keys.len(), 100);
}

#[test]
fn job_timezone_handling() {
    init_test_tracing();

    let job = Job {
        id: "job_1".to_string(),
        name: "Timezone Test".to_string(),
        description: None,
        workflow_id: "wf_1".to_string(),
        trigger: TriggerConfig::Interval { interval_secs: 60 },
        idempotency_key: None,
        state: JobState::Active,
        lease_owner: None,
        lease_expires_at: None,
        timezone: "America/New_York".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: Some(Utc::now()),
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    assert_eq!(job.timezone, "America/New_York");
}

#[test]
fn job_completed_no_next_run() {
    init_test_tracing();

    let config = SchedulerConfig {
        poll_interval: std::time::Duration::from_secs(30),
        lease_duration: std::time::Duration::from_secs(60),
        base_retry_delay_secs: 10,
        max_retry_delay_secs: 3600,
    };

    let worker = openrustclaw_scheduler::SchedulerWorker::new(config);
    
    // Absolute trigger job that has run once
    let mut job = Job {
        id: "job_1".to_string(),
        name: "One-time Job".to_string(),
        description: None,
        workflow_id: "wf_1".to_string(),
        trigger: TriggerConfig::Absolute { run_at: Utc::now() },
        idempotency_key: None,
        state: JobState::Active,
        lease_owner: Some("worker_1".to_string()),
        lease_expires_at: Some(Utc::now() + Duration::minutes(5)),
        timezone: "UTC".to_string(),
        max_retries: 3,
        last_run_at: None,
        next_run_at: Some(Utc::now()),
        run_count: 0,
        consecutive_failures: 0,
        created_at: Utc::now(),
    };

    worker.process_job_result(&mut job, true, None);

    // Absolute trigger with last_run should have no next run and be completed
    assert_eq!(job.state, JobState::Completed);
    assert!(job.next_run_at.is_none());
}

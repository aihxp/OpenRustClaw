# Scheduler API Reference

This reference documents the durable scheduler for OpenRustClaw.

**Crate**: `openrustclaw-scheduler`

---

## Overview

The scheduler is an app-owned Rust component that:
- Polls SQLite for due jobs
- Dispatches to LangGraph workflows via Python sidecar
- Manages retries with exponential backoff
- Handles dead-letter queue for failed jobs

**No cron jobs** - pure Rust implementation.

---

## SchedulerWorker

The main scheduler worker that polls for due jobs.

```rust
pub struct SchedulerWorker {
    config: SchedulerConfig,
    worker_id: String,
}

pub struct SchedulerConfig {
    pub poll_interval: Duration,
    pub lease_duration: Duration,
    pub base_retry_delay_secs: u64,
    pub max_retry_delay_secs: u64,
}
```

### Methods

| Method | Description |
|--------|-------------|
| `SchedulerWorker::new(config)` | Create worker with configuration |
| `process_job_result(job, success, error_msg)` | Process job execution result |
| `try_acquire_lease(job)` | Attempt to claim job for execution |
| `worker_id()` | Get unique worker ID |
| `poll_interval()` | Get configured poll interval |

**Example**:
```rust
use openrustclaw_scheduler::{SchedulerWorker, SchedulerConfig};
use std::time::Duration;

let config = SchedulerConfig {
    poll_interval: Duration::from_secs(30),
    lease_duration: Duration::from_secs(300),  // 5 min lease
    base_retry_delay_secs: 60,
    max_retry_delay_secs: 3600,
};

let worker = SchedulerWorker::new(config);

// Poll loop
loop {
    // Fetch due jobs from database
    let due_jobs = fetch_due_jobs(&db).await?;
    
    for mut job in due_jobs {
        if worker.try_acquire_lease(&mut job) {
            // Execute job via LangGraph sidecar
            match execute_job(&job).await {
                Ok(()) => {
                    worker.process_job_result(&mut job, true, None);
                }
                Err(e) => {
                    worker.process_job_result(&mut job, false, Some(&e.to_string()));
                }
            }
            
            // Update job in database
            update_job(&db, &job).await?;
        }
    }
    
    tokio::time::sleep(worker.poll_interval()).await;
}
```

---

## Job

A scheduled job definition.

```rust
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
```

### JobState

```rust
pub enum JobState {
    Active,
    Paused,
    Completed,
    Failed,
    DeadLetter,
}
```

### Methods

| Method | Description |
|--------|-------------|
| `generate_idempotency_key()` | Generate unique key for run |
| `is_due()` | Check if job should run now |
| `is_lease_expired()` | Check if lease can be claimed |

---

## TriggerConfig

Trigger types for job scheduling.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerConfig {
    Interval { interval_secs: u64 },
    Event { event_name: String },
    Webhook { webhook_url: String },
    Dependency { depends_on: Vec<String> },
    Absolute { run_at: DateTime<Utc> },
}
```

### Trigger Functions

```rust
use openrustclaw_scheduler::triggers::calculate_next_run;

// Calculate next run time
let next = calculate_next_run(&trigger, last_run_at);
```

**Examples**:

```rust
use openrustclaw_scheduler::triggers::TriggerConfig;
use chrono::{Utc, Duration};

// Run every 5 minutes
let interval_trigger = TriggerConfig::Interval { interval_secs: 300 };

// Run on specific event
let event_trigger = TriggerConfig::Event { 
    event_name: "user_signed_up".to_string() 
};

// Webhook-triggered
let webhook_trigger = TriggerConfig::Webhook { 
    webhook_url: "https://api.example.com/webhooks/run".to_string() 
};

// Dependency-based
let dep_trigger = TriggerConfig::Dependency { 
    depends_on: vec!["job_1".to_string(), "job_2".to_string()] 
};

// One-time at specific time
let absolute_trigger = TriggerConfig::Absolute { 
    run_at: Utc::now() + Duration::hours(24) 
};
```

---

## Retry Policy

Exponential backoff and dead-letter handling.

```rust
use openrustclaw_scheduler::retry;

// Calculate backoff delay
let delay = retry::backoff_delay(
    retry_count: 3,
    base_delay_secs: 60,
    max_delay_secs: 3600,
);
// delay = 60 * 2^3 = 480 seconds (8 minutes)

// Check if should move to dead letter
if retry::should_dead_letter(consecutive_failures: 5, max_retries: 3) {
    // Move to DLQ
}
```

### Retry Functions

| Function | Description |
|----------|-------------|
| `backoff_delay(retry, base, max)` | Calculate exponential backoff |
| `should_dead_letter(failures, max)` | Check if job should go to DLQ |

---

## JobRun

Record of a completed job execution.

```rust
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

pub enum RunStatus {
    Running,
    Success,
    Failure,
    Timeout,
}
```

---

## Timezone Support

```rust
use openrustclaw_scheduler::timezone::SchedulerTimezone;

let tz = SchedulerTimezone::new("America/New_York")?;
let now = tz.now();
let next_run = tz.next_occurrence(&cron_expr)?;
```

---

## Persistence

Job persistence traits and SQLite implementation.

```rust
use openrustclaw_scheduler::persistence::{JobRepository, SqliteJobRepository};

pub trait JobRepository: Send + Sync {
    async fn create(&self, job: &Job) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Option<Job>>;
    async fn update(&self, job: &Job) -> Result<()>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn list_due(&self) -> Result<Vec<Job>>;
    async fn list_active(&self) -> Result<Vec<Job>>;
    async fn list_dead_letter(&self) -> Result<Vec<Job>>;
}

// SQLite implementation
let repo = SqliteJobRepository::new(db_pool);
```

---

## Configuration

### Scheduler Configuration

```toml
[scheduler]
enabled = true
poll_interval_secs = 30
lease_duration_secs = 300
base_retry_delay_secs = 60
max_retry_delay_secs = 3600
max_retries = 3
timezone = "UTC"

[scheduler.sidecar]
grpc_address = "http://127.0.0.1:50051"
workflow_timeout_secs = 300
```

---

## CLI Commands

### Schedule Management

```bash
# List scheduled jobs
openrustclaw schedule list

# Create new job
openrustclaw schedule create \
    --name "daily-report" \
    --workflow "generate_report"

# Pause a job
openrustclaw schedule pause <job-id>

# Resume a job
openrustclaw schedule resume <job-id>
```

---

## Complete Example

```rust
use openrustclaw_scheduler::{
    SchedulerWorker, SchedulerConfig, Job, TriggerConfig, JobState,
    persistence::SqliteJobRepository,
};
use std::time::Duration;
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize database
    let db = sqlx::SqlitePool::connect("sqlite://scheduler.db").await?;
    let repo = SqliteJobRepository::new(db);
    
    // Create scheduler worker
    let config = SchedulerConfig {
        poll_interval: Duration::from_secs(30),
        lease_duration: Duration::from_secs(300),
        base_retry_delay_secs: 60,
        max_retry_delay_secs: 3600,
    };
    let worker = SchedulerWorker::new(config);
    
    // Create a job
    let job = Job {
        id: "job_001".to_string(),
        name: "Hourly Data Sync".to_string(),
        description: Some("Sync data from external API".to_string()),
        workflow_id: "data_sync".to_string(),
        trigger: TriggerConfig::Interval { interval_secs: 3600 },
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
    
    repo.create(&job).await?;
    
    // Main loop
    println!("Scheduler running (worker: {})", worker.worker_id());
    
    loop {
        let due_jobs = repo.list_due().await?;
        
        for mut job in due_jobs {
            if worker.try_acquire_lease(&mut job) {
                println!("Executing job: {}", job.name);
                
                match execute_workflow(&job.workflow_id).await {
                    Ok(()) => {
                        worker.process_job_result(&mut job, true, None);
                        println!("Job completed: {}", job.id);
                    }
                    Err(e) => {
                        worker.process_job_result(&mut job, false, Some(&e.to_string()));
                        println!("Job failed: {} - {}", job.id, e);
                    }
                }
                
                repo.update(&job).await?;
            }
        }
        
        tokio::time::sleep(worker.poll_interval()).await;
    }
}

async fn execute_workflow(workflow_id: &str) -> Result<()> {
    // Call LangGraph sidecar via gRPC
    // ... implementation
    Ok(())
}
```

---

## Error Handling

```rust
use openrustclaw_core::error::{Error, SchedulerError};

match result {
    Err(Error::Scheduler(SchedulerError::JobNotFound(id))) => {
        eprintln!("Job not found: {}", id);
    }
    Err(Error::Scheduler(SchedulerError::LeaseAcquisitionFailed { job_id })) => {
        // Another worker claimed the job, skip
        println!("Job {} already claimed", job_id);
    }
    Err(Error::Scheduler(SchedulerError::IdempotencyConflict { job_id, key })) => {
        // Job already ran with this key
        println!("Job {} already ran with key {}", job_id, key);
    }
    Err(Error::Scheduler(SchedulerError::MaxRetriesExceeded { job_id, attempts })) => {
        // Job moved to dead letter queue
        eprintln!("Job {} failed after {} attempts", job_id, attempts);
    }
    Err(Error::Scheduler(SchedulerError::InvalidTrigger(config))) => {
        eprintln!("Invalid trigger config: {}", config);
    }
    _ => {}
}
```

---

## Dead Letter Queue

Jobs that exceed max retries are moved to the dead letter state.

```rust
// List dead letter jobs
let dlq_jobs = repo.list_dead_letter().await?;

// Retry a dead letter job
for mut job in dlq_jobs {
    if should_retry_manually(&job) {
        job.state = JobState::Active;
        job.consecutive_failures = 0;
        job.next_run_at = Some(Utc::now());
        repo.update(&job).await?;
    }
}
```

---

## Best Practices

1. **Set appropriate lease duration**: Longer than expected job execution
2. **Use idempotency keys**: Prevent duplicate executions
3. **Monitor DLQ**: Regularly review and retry failed jobs
4. **Set reasonable retry limits**: Balance reliability and noise
5. **Use UTC internally**: Convert to local time only for display
6. **Log execution details**: Include trace IDs for debugging

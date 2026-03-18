//! Job scheduling commands.

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

// Job state is handled by the database

async fn open_schedule_pool() -> Result<sqlx::SqlitePool> {
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;
    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;
    Ok(pool)
}

/// List scheduled jobs.
pub async fn list() -> Result<()> {
    let pool = open_schedule_pool().await?;

    // Query scheduled jobs
    let rows = sqlx::query(
        r#"
        SELECT 
            id,
            name,
            description,
            workflow_id,
            state,
            trigger_type,
            next_run_at,
            last_run_at,
            run_count,
            consecutive_failures,
            created_at
        FROM scheduled_jobs 
        ORDER BY 
            CASE state 
                WHEN 'active' THEN 1 
                WHEN 'paused' THEN 2 
                ELSE 3 
            END,
            next_run_at
        "#,
    )
    .fetch_all(&pool)
    .await
    .context("Failed to query scheduled jobs")?;

    if rows.is_empty() {
        println!("No scheduled jobs found.");
        println!();
        println!("To create a scheduled job:");
        println!("  openrustclaw schedule create --name <name> --workflow <workflow>");
        return Ok(());
    }

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║                Scheduled Jobs                            ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    for row in rows {
        let id: String = row.get("id");
        let name: String = row.get("name");
        let description: Option<String> = row.get("description");
        let workflow_id: String = row.get("workflow_id");
        let state: String = row.get("state");
        let trigger_type: String = row.get("trigger_type");
        let next_run_at: Option<String> = row.get("next_run_at");
        let last_run_at: Option<String> = row.get("last_run_at");
        let run_count: i64 = row.get("run_count");
        let consecutive_failures: i64 = row.get("consecutive_failures");

        // State emoji
        let state_icon = match state.as_str() {
            "active" => "\x1b[32m●\x1b[0m",
            "paused" => "\x1b[33m⏸\x1b[0m",
            "completed" => "\x1b[90m✓\x1b[0m",
            "failed" => "\x1b[31m✗\x1b[0m",
            "dead_letter" => "\x1b[31m☠\x1b[0m",
            _ => "\x1b[90m?\x1b[0m",
        };

        println!("{} {} ({})", state_icon, name, &id[..8]);

        if let Some(desc) = description {
            println!("  {}", desc);
        }

        println!("  Workflow: {}", workflow_id);
        println!("  Trigger: {}", trigger_type);
        println!("  Runs: {} ({} failures)", run_count, consecutive_failures);

        if let Some(next) = next_run_at {
            println!("  Next run: {}", next);
        } else {
            println!("  Next run: Not scheduled");
        }

        if let Some(last) = last_run_at {
            println!("  Last run: {}", last);
        }

        println!();
    }

    Ok(())
}

/// Create a new scheduled job.
pub async fn create(
    name: &str,
    workflow: &str,
    description: Option<&str>,
    every_seconds: Option<u64>,
    at: Option<&str>,
    payload: Option<&str>,
) -> Result<()> {
    println!("Creating scheduled job: {}", name);
    println!("Workflow: {}", workflow);

    if every_seconds.is_some() && at.is_some() {
        anyhow::bail!("Use either --every-seconds or --at, not both");
    }

    let pool = open_schedule_pool().await?;

    // Check if job with same name exists
    let existing: Option<String> =
        sqlx::query_scalar("SELECT id FROM scheduled_jobs WHERE name = ?")
            .bind(name)
            .fetch_optional(&pool)
            .await?;

    if existing.is_some() {
        anyhow::bail!("A job with name '{}' already exists", name);
    }

    // Create job
    let id = Uuid::new_v4().to_string();
    let idempotency_key = format!("{}:{}", id, Uuid::new_v4());

    let payload_json = match payload {
        Some(raw) => serde_json::from_str::<serde_json::Value>(raw)
            .with_context(|| format!("Invalid --payload JSON: {}", raw))?,
        None => serde_json::json!({}),
    };
    let metadata = serde_json::json!({ "input": payload_json });

    let (trigger_type, trigger_config, next_run, trigger_description) = if let Some(run_at) = at {
        let run_at = chrono::DateTime::parse_from_rfc3339(run_at)
            .with_context(|| format!("Invalid RFC3339 timestamp for --at: {}", run_at))?
            .with_timezone(&Utc);
        (
            "absolute",
            serde_json::json!({
                "type": "absolute",
                "run_at": run_at.to_rfc3339(),
            }),
            run_at,
            format!("Once at {}", run_at.to_rfc3339()),
        )
    } else {
        let interval_secs = every_seconds.unwrap_or(3600);
        (
            "interval",
            serde_json::json!({
                "type": "interval",
                "interval_secs": interval_secs,
            }),
            Utc::now() + chrono::Duration::seconds(interval_secs as i64),
            format!("Every {} seconds", interval_secs),
        )
    };

    sqlx::query(
        r#"
        INSERT INTO scheduled_jobs (
            id, name, description, workflow_id, trigger_type, trigger_config,
            idempotency_key, state, timezone, max_retries, next_run_at, run_count, metadata, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
        "#,
    )
    .bind(&id)
    .bind(name)
    .bind(
        description
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("Auto-created job for workflow {}", workflow)),
    )
    .bind(workflow)
    .bind(trigger_type)
    .bind(trigger_config.to_string())
    .bind(&idempotency_key)
    .bind("active")
    .bind("UTC")
    .bind(3i64)
    .bind(next_run.to_rfc3339())
    .bind(0i64)
    .bind(metadata.to_string())
    .execute(&pool)
    .await
    .context("Failed to create scheduled job")?;

    println!("✓ Job created successfully");
    println!("  ID: {}", id);
    println!("  Name: {}", name);
    println!("  Workflow: {}", workflow);
    println!("  Trigger: {}", trigger_description);
    println!("  Next run: {}", next_run.to_rfc3339());
    println!();
    println!("To pause this job:");
    println!("  openrustclaw schedule pause {}", id);

    Ok(())
}

/// Pause a scheduled job.
pub async fn pause(id: &str) -> Result<()> {
    let pool = open_schedule_pool().await?;

    // Check if job exists
    let existing: Option<String> =
        sqlx::query_scalar("SELECT id FROM scheduled_jobs WHERE id = ? OR name = ?")
            .bind(id)
            .bind(id)
            .fetch_optional(&pool)
            .await?;

    let job_id = match existing {
        Some(id) => id,
        None => anyhow::bail!("Job '{}' not found", id),
    };

    // Update state to paused
    let result =
        sqlx::query("UPDATE scheduled_jobs SET state = 'paused' WHERE id = ? AND state = 'active'")
            .bind(&job_id)
            .execute(&pool)
            .await?;

    if result.rows_affected() == 0 {
        println!("Job '{}' is already paused or not in active state", id);
    } else {
        println!("✓ Job '{}' paused successfully", id);
    }

    Ok(())
}

/// Resume a scheduled job.
pub async fn resume(id: &str) -> Result<()> {
    let pool = open_schedule_pool().await?;

    // Check if job exists
    let existing: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT id, next_run_at FROM scheduled_jobs WHERE id = ? OR name = ?")
            .bind(id)
            .bind(id)
            .fetch_optional(&pool)
            .await?;

    let (job_id, next_run) = match existing {
        Some(row) => row,
        None => anyhow::bail!("Job '{}' not found", id),
    };

    // Calculate next run time if not set
    let next_run_at = match next_run {
        Some(t) => t,
        None => {
            let next = Utc::now() + chrono::Duration::minutes(1);
            next.to_rfc3339()
        }
    };

    // Update state to active
    let result = sqlx::query(
        "UPDATE scheduled_jobs SET state = 'active', next_run_at = ? WHERE id = ? AND state = 'paused'"
    )
    .bind(&next_run_at)
    .bind(&job_id)
    .execute(&pool)
    .await?;

    if result.rows_affected() == 0 {
        println!("Job '{}' is already active or not in paused state", id);
    } else {
        println!("✓ Job '{}' resumed successfully", id);
        println!("  Next run: {}", next_run_at);
    }

    Ok(())
}

/// List recent job attempts.
pub async fn runs(job: Option<&str>, limit: usize) -> Result<()> {
    let pool = open_schedule_pool().await?;

    let rows = if let Some(job) = job {
        sqlx::query(
            r#"
            SELECT r.id, r.job_id, j.name, r.status, r.started_at, r.completed_at,
                   r.retry_count, r.langsmith_trace_id, r.result
            FROM job_runs r
            JOIN scheduled_jobs j ON j.id = r.job_id
            WHERE r.job_id = ? OR j.name = ?
            ORDER BY r.started_at DESC
            LIMIT ?
            "#,
        )
        .bind(job)
        .bind(job)
        .bind(limit as i64)
        .fetch_all(&pool)
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT r.id, r.job_id, j.name, r.status, r.started_at, r.completed_at,
                   r.retry_count, r.langsmith_trace_id, r.result
            FROM job_runs r
            JOIN scheduled_jobs j ON j.id = r.job_id
            ORDER BY r.started_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&pool)
        .await?
    };

    if rows.is_empty() {
        println!("No job runs found.");
        return Ok(());
    }

    for row in rows {
        println!(
            "{} {} [{}] retry={} started={} completed={}",
            row.get::<String, _>("job_id"),
            row.get::<String, _>("name"),
            row.get::<String, _>("status"),
            row.get::<i64, _>("retry_count"),
            row.get::<String, _>("started_at"),
            row.get::<Option<String>, _>("completed_at")
                .unwrap_or_else(|| "-".to_string())
        );
        if let Some(trace_id) = row.get::<Option<String>, _>("langsmith_trace_id") {
            println!("  trace: {}", trace_id);
        }
    }

    Ok(())
}

/// List current dead-letter entries.
pub async fn dead_letters(job: Option<&str>, limit: usize) -> Result<()> {
    let pool = open_schedule_pool().await?;
    let rows = if let Some(job) = job {
        sqlx::query(
            r#"
            SELECT d.id, d.job_id, j.name, d.last_error, d.failed_at, d.retry_count, d.resolved
            FROM dead_letter_queue d
            JOIN scheduled_jobs j ON j.id = d.job_id
            WHERE d.job_id = ? OR j.name = ?
            ORDER BY d.failed_at DESC
            LIMIT ?
            "#,
        )
        .bind(job)
        .bind(job)
        .bind(limit as i64)
        .fetch_all(&pool)
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT d.id, d.job_id, j.name, d.last_error, d.failed_at, d.retry_count, d.resolved
            FROM dead_letter_queue d
            JOIN scheduled_jobs j ON j.id = d.job_id
            ORDER BY d.failed_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&pool)
        .await?
    };

    if rows.is_empty() {
        println!("No dead-letter entries found.");
        return Ok(());
    }

    for row in rows {
        println!(
            "{} {} [{} retries] resolved={} failed_at={}",
            row.get::<String, _>("id"),
            row.get::<String, _>("name"),
            row.get::<i64, _>("retry_count"),
            row.get::<i64, _>("resolved") == 1,
            row.get::<String, _>("failed_at")
        );
        println!("  error: {}", row.get::<String, _>("last_error"));
    }

    Ok(())
}

/// Replay a dead-letter entry by resetting the target job.
pub async fn replay_dead_letter(id: &str) -> Result<()> {
    let pool = open_schedule_pool().await?;
    let row = sqlx::query(
        r#"
        SELECT id, job_id
        FROM dead_letter_queue
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(&pool)
    .await?;

    let Some(row) = row else {
        anyhow::bail!("Dead-letter entry '{}' not found", id);
    };

    let job_id: String = row.get("job_id");
    sqlx::query(
        r#"
        UPDATE scheduled_jobs
        SET state = 'active',
            consecutive_failures = 0,
            lease_owner = NULL,
            lease_expires_at = NULL,
            next_run_at = COALESCE(next_run_at, ?)
        WHERE id = ?
        "#,
    )
    .bind((Utc::now() + chrono::Duration::minutes(1)).to_rfc3339())
    .bind(&job_id)
    .execute(&pool)
    .await?;

    sqlx::query("UPDATE dead_letter_queue SET resolved = 1, resolved_at = ? WHERE id = ?")
        .bind(Utc::now().to_rfc3339())
        .bind(id)
        .execute(&pool)
        .await?;

    println!("✓ Replayed dead-letter entry '{}' for job '{}'", id, job_id);
    Ok(())
}

/// List recent runtime events.
pub async fn events(name: Option<&str>, limit: usize) -> Result<()> {
    let pool = open_schedule_pool().await?;
    let rows = if let Some(name) = name {
        sqlx::query(
            r#"
            SELECT id, event_name, event_type, session_id, status, created_at, processed_at
            FROM runtime_events
            WHERE event_name = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(name)
        .bind(limit as i64)
        .fetch_all(&pool)
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT id, event_name, event_type, session_id, status, created_at, processed_at
            FROM runtime_events
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&pool)
        .await?
    };

    if rows.is_empty() {
        println!("No runtime events found.");
        return Ok(());
    }

    for row in rows {
        println!(
            "{} {} [{}] status={} session={} created={}",
            row.get::<String, _>("id"),
            row.get::<String, _>("event_name"),
            row.get::<String, _>("event_type"),
            row.get::<String, _>("status"),
            row.get::<Option<String>, _>("session_id")
                .unwrap_or_else(|| "-".to_string()),
            row.get::<String, _>("created_at")
        );
    }

    Ok(())
}

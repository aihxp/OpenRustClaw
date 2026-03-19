use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, Utc};
use openrustclaw_core::config::AppConfig;
use openrustclaw_db::{SqlitePool, init_pool};
use serde::Serialize;
use sqlx::Row;

use super::{channels::ChannelRegistry, runtime};

#[derive(Debug, Clone, Serialize)]
pub struct SessionCounts {
    pub total: i64,
    pub active: i64,
    pub archived: i64,
    pub closed: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelRegistryCounts {
    pub total_accounts: usize,
    pub enabled_accounts: usize,
    pub approved_accounts: usize,
    pub blocked_accounts: usize,
    pub total_bindings: usize,
    pub enabled_bindings: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SidecarServiceStatus {
    pub role: String,
    pub auto_start: bool,
    pub running: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceStatusReport {
    pub generated_at: String,
    pub config_path: String,
    pub process_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<i64>,
    pub gateway_addr: String,
    pub default_provider: String,
    pub fallback_chain: Vec<String>,
    pub enabled_channels: Vec<String>,
    pub sessions: SessionCounts,
    pub registry: ChannelRegistryCounts,
    pub sidecar: SidecarServiceStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct SchedulerRunSummary {
    pub id: String,
    pub job_id: String,
    pub name: String,
    pub status: String,
    pub retry_count: i64,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub langsmith_trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SchedulerStatusReport {
    pub generated_at: String,
    pub poll_interval_ms: u64,
    pub lease_duration_secs: u64,
    pub total_jobs: i64,
    pub active_jobs: i64,
    pub paused_jobs: i64,
    pub completed_jobs: i64,
    pub due_jobs: i64,
    pub unresolved_dead_letters: i64,
    pub pending_runtime_events: i64,
    pub processed_runtime_events: i64,
    pub failed_runtime_events: i64,
    pub recent_runs: Vec<SchedulerRunSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeEventSummary {
    pub id: String,
    pub event_name: String,
    pub event_type: String,
    pub session_id: Option<String>,
    pub status: String,
    pub created_at: String,
    pub processed_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LiveRuntimeMetadata {
    pub config_path: String,
    pub gateway_addr: String,
    pub started_at: Option<DateTime<Utc>>,
    pub sidecar_running: bool,
}

pub async fn status(config_path: &str, workspace_root: &Path) -> Result<ServiceStatusReport> {
    let config = runtime::load_effective_config(config_path, workspace_root)?;
    let pool = init_pool(&config.database.url, config.database.max_connections).await?;
    let registry_root = super::channels::resolve_root(None)?;
    let registry = super::channels::load_registry(registry_root)?;
    status_with_pool(
        &config,
        &pool,
        &registry,
        &LiveRuntimeMetadata {
            config_path: config_path.to_string(),
            gateway_addr: format!("{}:{}", config.gateway.host, config.gateway.port),
            started_at: None,
            sidecar_running: false,
        },
    )
    .await
}

pub async fn status_with_pool(
    config: &AppConfig,
    pool: &SqlitePool,
    registry: &ChannelRegistry,
    live: &LiveRuntimeMetadata,
) -> Result<ServiceStatusReport> {
    let session_row = sqlx::query(
        r#"
        SELECT
            COUNT(*) AS total,
            SUM(CASE WHEN status = 'active' THEN 1 ELSE 0 END) AS active,
            SUM(CASE WHEN status = 'archived' THEN 1 ELSE 0 END) AS archived,
            SUM(CASE WHEN status = 'closed' THEN 1 ELSE 0 END) AS closed
        FROM sessions
        "#,
    )
    .fetch_one(pool)
    .await?;

    let uptime_seconds = live
        .started_at
        .map(|started_at| (Utc::now() - started_at).num_seconds().max(0));

    Ok(ServiceStatusReport {
        generated_at: Utc::now().to_rfc3339(),
        config_path: live.config_path.clone(),
        process_id: std::process::id(),
        started_at: live.started_at.map(|value| value.to_rfc3339()),
        uptime_seconds,
        gateway_addr: live.gateway_addr.clone(),
        default_provider: config.providers.default_provider.clone(),
        fallback_chain: config.providers.fallback_chain.clone(),
        enabled_channels: enabled_channels(config),
        sessions: SessionCounts {
            total: session_row.get::<i64, _>("total"),
            active: session_row.get::<Option<i64>, _>("active").unwrap_or(0),
            archived: session_row.get::<Option<i64>, _>("archived").unwrap_or(0),
            closed: session_row.get::<Option<i64>, _>("closed").unwrap_or(0),
        },
        registry: ChannelRegistryCounts {
            total_accounts: registry.accounts.len(),
            enabled_accounts: registry
                .accounts
                .values()
                .filter(|account| account.enabled)
                .count(),
            approved_accounts: registry
                .accounts
                .values()
                .filter(|account| account.approved)
                .count(),
            blocked_accounts: registry
                .accounts
                .values()
                .filter(|account| account.blocked)
                .count(),
            total_bindings: registry.bindings.len(),
            enabled_bindings: registry
                .bindings
                .iter()
                .filter(|binding| binding.enabled)
                .count(),
        },
        sidecar: SidecarServiceStatus {
            role: format!("{:?}", config.sidecar.role).to_lowercase(),
            auto_start: config.sidecar.auto_start,
            running: live.sidecar_running,
        },
    })
}

pub async fn scheduler(config_path: &str, workspace_root: &Path) -> Result<SchedulerStatusReport> {
    let config = runtime::load_effective_config(config_path, workspace_root)?;
    let pool = init_pool(&config.database.url, config.database.max_connections).await?;
    scheduler_with_pool(&config, &pool, 10).await
}

pub async fn scheduler_with_pool(
    config: &AppConfig,
    pool: &SqlitePool,
    recent_run_limit: usize,
) -> Result<SchedulerStatusReport> {
    let jobs_row = sqlx::query(
        r#"
        SELECT
            COUNT(*) AS total_jobs,
            SUM(CASE WHEN state = 'active' THEN 1 ELSE 0 END) AS active_jobs,
            SUM(CASE WHEN state = 'paused' THEN 1 ELSE 0 END) AS paused_jobs,
            SUM(CASE WHEN state = 'completed' THEN 1 ELSE 0 END) AS completed_jobs,
            SUM(
                CASE
                    WHEN state = 'active'
                     AND next_run_at IS NOT NULL
                     AND datetime(next_run_at) <= datetime('now')
                     AND (disabled_until IS NULL OR datetime(disabled_until) <= datetime('now'))
                    THEN 1 ELSE 0
                END
            ) AS due_jobs
        FROM scheduled_jobs
        "#,
    )
    .fetch_one(pool)
    .await?;

    let dead_letters: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM dead_letter_queue WHERE resolved = 0")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let events_row = sqlx::query(
        r#"
        SELECT
            SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) AS pending_runtime_events,
            SUM(CASE WHEN status = 'processed' THEN 1 ELSE 0 END) AS processed_runtime_events,
            SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) AS failed_runtime_events
        FROM runtime_events
        "#,
    )
    .fetch_one(pool)
    .await?;

    let recent_runs = sqlx::query(
        r#"
        SELECT r.id, r.job_id, j.name, r.status, r.retry_count, r.started_at, r.completed_at, r.langsmith_trace_id
        FROM job_runs r
        JOIN scheduled_jobs j ON j.id = r.job_id
        ORDER BY r.started_at DESC
        LIMIT ?
        "#,
    )
    .bind(recent_run_limit.max(1) as i64)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| SchedulerRunSummary {
        id: row.get("id"),
        job_id: row.get("job_id"),
        name: row.get("name"),
        status: row.get("status"),
        retry_count: row.get("retry_count"),
        started_at: row.get("started_at"),
        completed_at: row.get("completed_at"),
        langsmith_trace_id: row.get("langsmith_trace_id"),
    })
    .collect();

    Ok(SchedulerStatusReport {
        generated_at: Utc::now().to_rfc3339(),
        poll_interval_ms: config.scheduler.poll_interval_ms,
        lease_duration_secs: config.scheduler.lease_duration_secs,
        total_jobs: jobs_row.get::<i64, _>("total_jobs"),
        active_jobs: jobs_row.get::<Option<i64>, _>("active_jobs").unwrap_or(0),
        paused_jobs: jobs_row.get::<Option<i64>, _>("paused_jobs").unwrap_or(0),
        completed_jobs: jobs_row
            .get::<Option<i64>, _>("completed_jobs")
            .unwrap_or(0),
        due_jobs: jobs_row.get::<Option<i64>, _>("due_jobs").unwrap_or(0),
        unresolved_dead_letters: dead_letters,
        pending_runtime_events: events_row
            .get::<Option<i64>, _>("pending_runtime_events")
            .unwrap_or(0),
        processed_runtime_events: events_row
            .get::<Option<i64>, _>("processed_runtime_events")
            .unwrap_or(0),
        failed_runtime_events: events_row
            .get::<Option<i64>, _>("failed_runtime_events")
            .unwrap_or(0),
        recent_runs,
    })
}

pub async fn runtime_events(
    config_path: &str,
    workspace_root: &Path,
    event_name: Option<&str>,
    limit: usize,
) -> Result<Vec<RuntimeEventSummary>> {
    let config = runtime::load_effective_config(config_path, workspace_root)?;
    let pool = init_pool(&config.database.url, config.database.max_connections).await?;
    runtime_events_with_pool(&pool, event_name, limit).await
}

pub async fn runtime_events_with_pool(
    pool: &SqlitePool,
    event_name: Option<&str>,
    limit: usize,
) -> Result<Vec<RuntimeEventSummary>> {
    let rows = if let Some(event_name) = event_name {
        sqlx::query(
            r#"
            SELECT id, event_name, event_type, session_id, status, created_at, processed_at
            FROM runtime_events
            WHERE event_name = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(event_name)
        .bind(limit.max(1) as i64)
        .fetch_all(pool)
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
        .bind(limit.max(1) as i64)
        .fetch_all(pool)
        .await?
    };

    rows.into_iter()
        .map(|row| {
            Ok(RuntimeEventSummary {
                id: row.get("id"),
                event_name: row.get("event_name"),
                event_type: row.get("event_type"),
                session_id: row.get("session_id"),
                status: row.get("status"),
                created_at: row.get("created_at"),
                processed_at: row.get("processed_at"),
            })
        })
        .collect()
}

fn enabled_channels(config: &AppConfig) -> Vec<String> {
    let mut channels = Vec::new();
    if config.channels.telegram.enabled {
        channels.push("telegram".to_string());
    }
    if config.channels.discord.enabled {
        channels.push("discord".to_string());
    }
    if config.channels.slack.enabled {
        channels.push("slack".to_string());
    }
    if config.channels.whatsapp.enabled {
        channels.push("whatsapp".to_string());
    }
    if config.channels.teams.enabled {
        channels.push("teams".to_string());
    }
    if config.channels.mattermost.enabled {
        channels.push("mattermost".to_string());
    }
    if config.channels.google_chat.enabled {
        channels.push("google_chat".to_string());
    }
    if config.channels.google_meet.enabled {
        channels.push("google_meet".to_string());
    }
    if config.channels.gmail_pubsub.enabled {
        channels.push("gmail_pubsub".to_string());
    }
    if config.channels.signal.enabled {
        channels.push("signal".to_string());
    }
    if config.channels.matrix.enabled {
        channels.push("matrix".to_string());
    }
    if config.channels.imessage.enabled {
        channels.push("imessage".to_string());
    }
    channels
}

#[cfg(test)]
mod tests {
    use super::enabled_channels;
    use openrustclaw_core::config::AppConfig;

    #[test]
    fn enabled_channels_only_returns_enabled_entries() {
        let mut config = AppConfig::load().unwrap_or_default();
        config.channels.telegram.enabled = true;
        config.channels.discord.enabled = false;
        config.channels.matrix.enabled = true;
        let channels = enabled_channels(&config);
        assert!(channels.contains(&"telegram".to_string()));
        assert!(channels.contains(&"matrix".to_string()));
        assert!(!channels.contains(&"discord".to_string()));
    }
}

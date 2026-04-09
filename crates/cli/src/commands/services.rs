use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc};
use openrustclaw_app::channel_health_monitor as app_channel_health_monitor;
use openrustclaw_core::config::{AppConfig, IMessageBridgeMode};
use openrustclaw_db::{SqlitePool, init_pool};
use serde::Serialize;
use serde_json::Value;
use sqlx::Row;

use super::{channels::ChannelRegistry, runtime};

pub const DEFAULT_CHANNEL_PROBES_PATH: &str = ".claw/control/channel-probes.json";
pub const DEFAULT_CHANNEL_HEALTH_MONITOR_PATH: &str = ".claw/control/channel-health-monitor.json";

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
    pub listener_addr: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub advertised_addrs: Vec<String>,
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
    pub payload: Value,
    pub status: String,
    pub created_at: String,
    pub processed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelProbeStatus {
    Ready,
    Warning,
    Failed,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ChannelProbeEntry {
    pub platform: String,
    pub enabled: bool,
    pub status: ChannelProbeStatus,
    pub probe_kind: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ChannelProbeReport {
    pub generated_at: String,
    pub entries: Vec<ChannelProbeEntry>,
    pub monitor: ChannelHealthMonitorStatus,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ChannelHealthMonitorStatus {
    pub generated_at: String,
    pub health_monitor_enabled: bool,
    pub auto_restart_on_failure: bool,
    pub auto_restart_ready: bool,
    pub consecutive_failure_threshold: usize,
    pub current_consecutive_failures: usize,
    pub degraded: bool,
    pub failing_platforms: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_failure_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_healthy_at: Option<String>,
    pub restart_requested: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LiveRuntimeMetadata {
    pub config_path: String,
    pub gateway_addr: String,
    pub listener_addr: String,
    pub advertised_addrs: Vec<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub sidecar_running: bool,
}

fn app_channel_probe_entry(
    entry: &ChannelProbeEntry,
) -> app_channel_health_monitor::ChannelProbeEntry {
    app_channel_health_monitor::ChannelProbeEntry {
        platform: entry.platform.clone(),
        enabled: entry.enabled,
        status: match entry.status {
            ChannelProbeStatus::Ready => app_channel_health_monitor::ChannelProbeStatus::Ready,
            ChannelProbeStatus::Warning => app_channel_health_monitor::ChannelProbeStatus::Warning,
            ChannelProbeStatus::Failed => app_channel_health_monitor::ChannelProbeStatus::Failed,
        },
        probe_kind: entry.probe_kind.clone(),
        detail: entry.detail.clone(),
    }
}

fn app_channel_health_monitor_status(
    status: &ChannelHealthMonitorStatus,
) -> app_channel_health_monitor::ChannelHealthMonitorStatus {
    app_channel_health_monitor::ChannelHealthMonitorStatus {
        generated_at: status.generated_at.clone(),
        health_monitor_enabled: status.health_monitor_enabled,
        auto_restart_on_failure: status.auto_restart_on_failure,
        auto_restart_ready: status.auto_restart_ready,
        consecutive_failure_threshold: status.consecutive_failure_threshold,
        current_consecutive_failures: status.current_consecutive_failures,
        degraded: status.degraded,
        failing_platforms: status.failing_platforms.clone(),
        last_failure_at: status.last_failure_at.clone(),
        last_healthy_at: status.last_healthy_at.clone(),
        restart_requested: status.restart_requested,
        restart_reason: status.restart_reason.clone(),
    }
}

fn channel_health_monitor_status_from_app(
    status: app_channel_health_monitor::ChannelHealthMonitorStatus,
) -> ChannelHealthMonitorStatus {
    ChannelHealthMonitorStatus {
        generated_at: status.generated_at,
        health_monitor_enabled: status.health_monitor_enabled,
        auto_restart_on_failure: status.auto_restart_on_failure,
        auto_restart_ready: status.auto_restart_ready,
        consecutive_failure_threshold: status.consecutive_failure_threshold,
        current_consecutive_failures: status.current_consecutive_failures,
        degraded: status.degraded,
        failing_platforms: status.failing_platforms,
        last_failure_at: status.last_failure_at,
        last_healthy_at: status.last_healthy_at,
        restart_requested: status.restart_requested,
        restart_reason: status.restart_reason,
    }
}

pub async fn status(config_path: &str, workspace_root: &Path) -> Result<ServiceStatusReport> {
    let config = runtime::load_effective_config(config_path, workspace_root)?;
    let advertising = runtime::resolve_gateway_advertising(&config);
    let pool = init_pool(&config.database.url, config.database.max_connections).await?;
    let registry_root = super::channels::resolve_root(None)?;
    let registry = super::channels::load_registry(registry_root)?;
    status_with_pool(
        &config,
        &pool,
        &registry,
        &LiveRuntimeMetadata {
            config_path: config_path.to_string(),
            gateway_addr: advertising.primary_addr,
            listener_addr: advertising.listener_addr,
            advertised_addrs: advertising.advertised_addrs,
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
        listener_addr: live.listener_addr.clone(),
        advertised_addrs: live.advertised_addrs.clone(),
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

pub async fn channel_probes(
    config_path: &str,
    workspace_root: &Path,
) -> Result<ChannelProbeReport> {
    channel_probes_status(config_path, workspace_root, false).await
}

pub async fn channel_probes_status(
    config_path: &str,
    workspace_root: &Path,
    refresh: bool,
) -> Result<ChannelProbeReport> {
    if !refresh && let Some(report) = load_cached_channel_probes(workspace_root)? {
        return Ok(report);
    }
    let config = runtime::load_effective_config(config_path, workspace_root)?;
    let report = channel_probes_with_config(config_path, workspace_root, &config).await?;
    save_channel_probes(workspace_root, &report)?;
    save_channel_health_monitor_status(workspace_root, &report.monitor)?;
    Ok(report)
}

pub async fn channel_probes_with_config(
    config_path: &str,
    workspace_root: &Path,
    config: &AppConfig,
) -> Result<ChannelProbeReport> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .build()?;
    let mut entries = Vec::new();

    if config.channels.telegram.enabled {
        entries.push(probe_telegram(&client, &config.channels.telegram).await);
    }
    if config.channels.discord.enabled {
        entries.push(probe_discord(&client, &config.channels.discord).await);
    }
    if config.channels.slack.enabled {
        entries.push(probe_slack(&client, &config.channels.slack).await);
    }
    if config.channels.mattermost.enabled {
        entries.push(probe_mattermost(&client, &config.channels.mattermost).await);
    }
    if config.channels.matrix.enabled {
        entries.push(probe_matrix(&client, &config.channels.matrix).await);
    }
    if config.channels.whatsapp.enabled {
        entries.push(probe_whatsapp(&config.channels.whatsapp));
    }
    if config.channels.teams.enabled {
        entries.push(probe_teams(&config.channels.teams));
    }
    if config.channels.google_chat.enabled {
        entries.push(probe_google_chat(&config.channels.google_chat));
    }
    if config.channels.google_meet.enabled {
        entries.push(probe_google_meet(&config.channels.google_meet));
    }
    if config.channels.gmail_pubsub.enabled {
        entries.push(probe_gmail(&config.channels.gmail_pubsub));
    }
    if config.channels.signal.enabled {
        entries.push(probe_signal(&config.channels.signal));
    }
    if config.channels.imessage.enabled {
        entries.push(probe_imessage(&config.channels.imessage));
    }

    let monitor =
        build_channel_health_monitor_status(config_path, workspace_root, config, &entries)?;

    Ok(ChannelProbeReport {
        generated_at: Utc::now().to_rfc3339(),
        entries,
        monitor,
    })
}

pub async fn runtime_events_with_pool(
    pool: &SqlitePool,
    event_name: Option<&str>,
    limit: usize,
) -> Result<Vec<RuntimeEventSummary>> {
    let rows = if let Some(event_name) = event_name {
        sqlx::query(
            r#"
            SELECT id, event_name, event_type, session_id, payload, status, created_at, processed_at
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
            SELECT id, event_name, event_type, session_id, payload, status, created_at, processed_at
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
                payload: serde_json::from_str(&row.get::<String, _>("payload"))
                    .unwrap_or_else(|_| serde_json::json!({})),
                status: row.get("status"),
                created_at: row.get("created_at"),
                processed_at: row.get("processed_at"),
            })
        })
        .collect()
}

async fn probe_telegram(
    client: &reqwest::Client,
    config: &openrustclaw_core::config::TelegramConfig,
) -> ChannelProbeEntry {
    if config.token.trim().is_empty() {
        return failed_entry("telegram", "remote_auth", "telegram token is missing");
    }
    let base = config
        .api_base_url
        .as_deref()
        .unwrap_or("https://api.telegram.org")
        .trim_end_matches('/');
    let url = format!("{base}/bot{}/getMe", config.token);
    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => {
            ready_entry("telegram", "remote_auth", "Bot API auth probe succeeded")
        }
        Ok(resp) => failed_entry(
            "telegram",
            "remote_auth",
            format!("Bot API probe failed with status {}", resp.status()),
        ),
        Err(error) => failed_entry("telegram", "remote_auth", error.to_string()),
    }
}

async fn probe_discord(
    client: &reqwest::Client,
    config: &openrustclaw_core::config::DiscordConfig,
) -> ChannelProbeEntry {
    if config.token.trim().is_empty() {
        return failed_entry("discord", "remote_auth", "discord bot token is missing");
    }
    let base = config
        .api_base_url
        .as_deref()
        .unwrap_or("https://discord.com/api/v10")
        .trim_end_matches('/');
    let url = format!("{base}/users/@me");
    match client
        .get(url)
        .header("authorization", format!("Bot {}", config.token))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            ready_entry("discord", "remote_auth", "Discord auth probe succeeded")
        }
        Ok(resp) => failed_entry(
            "discord",
            "remote_auth",
            format!("Discord auth probe failed with status {}", resp.status()),
        ),
        Err(error) => failed_entry("discord", "remote_auth", error.to_string()),
    }
}

async fn probe_slack(
    client: &reqwest::Client,
    config: &openrustclaw_core::config::SlackConfig,
) -> ChannelProbeEntry {
    if config.token.trim().is_empty() {
        return failed_entry("slack", "remote_auth", "slack bot token is missing");
    }
    let base = config
        .api_base_url
        .as_deref()
        .unwrap_or("https://slack.com/api")
        .trim_end_matches('/');
    let url = format!("{base}/auth.test");
    match client.post(url).bearer_auth(&config.token).send().await {
        Ok(resp) if resp.status().is_success() => {
            ready_entry("slack", "remote_auth", "Slack auth probe succeeded")
        }
        Ok(resp) => failed_entry(
            "slack",
            "remote_auth",
            format!("Slack auth probe failed with status {}", resp.status()),
        ),
        Err(error) => failed_entry("slack", "remote_auth", error.to_string()),
    }
}

async fn probe_mattermost(
    client: &reqwest::Client,
    config: &openrustclaw_core::config::MattermostConfig,
) -> ChannelProbeEntry {
    if config.bot_token.trim().is_empty() {
        return failed_entry(
            "mattermost",
            "remote_auth",
            "mattermost bot token is missing",
        );
    }
    let url = format!(
        "{}/api/v4/users/me",
        config.server_url.trim_end_matches('/')
    );
    match client.get(url).bearer_auth(&config.bot_token).send().await {
        Ok(resp) if resp.status().is_success() => ready_entry(
            "mattermost",
            "remote_auth",
            "Mattermost auth probe succeeded",
        ),
        Ok(resp) => failed_entry(
            "mattermost",
            "remote_auth",
            format!("Mattermost auth probe failed with status {}", resp.status()),
        ),
        Err(error) => failed_entry("mattermost", "remote_auth", error.to_string()),
    }
}

async fn probe_matrix(
    client: &reqwest::Client,
    config: &openrustclaw_core::config::MatrixConfig,
) -> ChannelProbeEntry {
    let Some(token) = config.access_token.as_deref() else {
        return warning_entry(
            "matrix",
            "config_readiness",
            "Matrix is enabled but no access token is configured; password-based live probe is skipped",
        );
    };
    let url = format!(
        "{}/_matrix/client/v3/account/whoami",
        config.homeserver.trim_end_matches('/')
    );
    match client.get(url).bearer_auth(token).send().await {
        Ok(resp) if resp.status().is_success() => {
            ready_entry("matrix", "remote_auth", "Matrix whoami probe succeeded")
        }
        Ok(resp) => failed_entry(
            "matrix",
            "remote_auth",
            format!("Matrix whoami probe failed with status {}", resp.status()),
        ),
        Err(error) => failed_entry("matrix", "remote_auth", error.to_string()),
    }
}

fn probe_whatsapp(config: &openrustclaw_core::config::WhatsAppConfig) -> ChannelProbeEntry {
    if !Path::new(&config.bridge_path).exists() {
        return failed_entry(
            "whatsapp",
            "local_runtime",
            format!("bridge path '{}' does not exist", config.bridge_path),
        );
    }
    let session_dir = Path::new(&config.session_path);
    let creds_path = session_dir.join("creds.json");
    if creds_path.exists() {
        ready_entry(
            "whatsapp",
            "local_runtime",
            "Baileys bridge path exists and WhatsApp session credentials are present",
        )
    } else if session_dir.exists() {
        warning_entry(
            "whatsapp",
            "local_runtime",
            "Baileys bridge path exists and session directory is present, but WhatsApp pairing is not complete yet",
        )
    } else {
        warning_entry(
            "whatsapp",
            "local_runtime",
            "Baileys bridge path exists but session directory is not initialized yet",
        )
    }
}

fn probe_teams(config: &openrustclaw_core::config::TeamsConfig) -> ChannelProbeEntry {
    if config.app_id.trim().is_empty() || config.app_password.trim().is_empty() {
        failed_entry(
            "teams",
            "config_readiness",
            "teams app_id/app_password are required",
        )
    } else {
        ready_entry(
            "teams",
            "config_readiness",
            "Teams app credentials and webhook path are configured",
        )
    }
}

fn probe_google_chat(config: &openrustclaw_core::config::GoogleChatConfig) -> ChannelProbeEntry {
    if config.service_account_key.trim().is_empty() || config.project_id.trim().is_empty() {
        return failed_entry(
            "google_chat",
            "config_readiness",
            "service account key and project id are required",
        );
    }
    if !probe_key_path_like(&config.service_account_key) {
        return warning_entry(
            "google_chat",
            "config_readiness",
            "service account key path does not exist yet",
        );
    }
    ready_entry(
        "google_chat",
        "config_readiness",
        "Google Chat credentials are configured",
    )
}

fn probe_google_meet(config: &openrustclaw_core::config::GoogleMeetConfig) -> ChannelProbeEntry {
    if config.service_account_key_path.trim().is_empty()
        || config.delegated_user_email.trim().is_empty()
    {
        return failed_entry(
            "google_meet",
            "config_readiness",
            "service account key path and delegated user email are required",
        );
    }
    if !probe_key_path_like(&config.service_account_key_path) {
        return warning_entry(
            "google_meet",
            "config_readiness",
            "service account key path does not exist yet",
        );
    }
    ready_entry(
        "google_meet",
        "config_readiness",
        "Google Meet credentials are configured",
    )
}

fn probe_gmail(config: &openrustclaw_core::config::GmailPubSubConfig) -> ChannelProbeEntry {
    if config.service_account_key_path.trim().is_empty()
        || config.project_id.trim().is_empty()
        || config.user_email.trim().is_empty()
    {
        return failed_entry(
            "gmail_pubsub",
            "config_readiness",
            "service account key path, project id, and user email are required",
        );
    }
    if !probe_key_path_like(&config.service_account_key_path) {
        return warning_entry(
            "gmail_pubsub",
            "config_readiness",
            "service account key path does not exist yet",
        );
    }
    ready_entry(
        "gmail_pubsub",
        "config_readiness",
        "Gmail Pub/Sub credentials are configured",
    )
}

fn probe_signal(config: &openrustclaw_core::config::SignalConfig) -> ChannelProbeEntry {
    let path = config
        .signal_cli_path
        .clone()
        .unwrap_or_else(|| PathBuf::from("signal-cli"));
    if path.exists() || path == Path::new("signal-cli") {
        ready_entry("signal", "local_runtime", "Signal CLI path is configured")
    } else {
        failed_entry(
            "signal",
            "local_runtime",
            format!("signal-cli path '{}' does not exist", path.display()),
        )
    }
}

fn probe_imessage(config: &openrustclaw_core::config::IMessageConfig) -> ChannelProbeEntry {
    match &config.bridge_mode {
        IMessageBridgeMode::BlueBubbles {
            server_url,
            password,
        } => {
            if server_url.trim().is_empty() || password.trim().is_empty() {
                failed_entry(
                    "imessage",
                    "config_readiness",
                    "BlueBubbles server_url and password are required",
                )
            } else {
                ready_entry(
                    "imessage",
                    "config_readiness",
                    "BlueBubbles server credentials are configured",
                )
            }
        }
        IMessageBridgeMode::MacOSDirect => {
            ready_entry("imessage", "local_runtime", "macOS direct mode selected")
        }
        IMessageBridgeMode::PrivateApi => warning_entry(
            "imessage",
            "local_runtime",
            "private API mode requires local macOS validation",
        ),
    }
}

fn probe_key_path_like(value: &str) -> bool {
    value.starts_with("token:") || value.starts_with("env:") || Path::new(value).exists()
}

pub fn channel_probe_path_for(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root.as_ref().join(DEFAULT_CHANNEL_PROBES_PATH)
}

pub fn channel_health_monitor_path_for(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root
        .as_ref()
        .join(DEFAULT_CHANNEL_HEALTH_MONITOR_PATH)
}

pub fn load_cached_channel_probes(workspace_root: &Path) -> Result<Option<ChannelProbeReport>> {
    let path = channel_probe_path_for(workspace_root);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)?;
    let report = serde_json::from_str(&raw)?;
    Ok(Some(report))
}

pub fn save_channel_probes(workspace_root: &Path, report: &ChannelProbeReport) -> Result<()> {
    let path = channel_probe_path_for(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let rendered = serde_json::to_string_pretty(report)?;
    fs::write(path, rendered.as_bytes())?;
    Ok(())
}

pub fn load_channel_health_monitor_status(
    workspace_root: &Path,
) -> Result<Option<ChannelHealthMonitorStatus>> {
    let path = channel_health_monitor_path_for(workspace_root);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)?;
    let report = serde_json::from_str(&raw)?;
    Ok(Some(report))
}

pub fn save_channel_health_monitor_status(
    workspace_root: &Path,
    monitor: &ChannelHealthMonitorStatus,
) -> Result<()> {
    let path = channel_health_monitor_path_for(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let rendered = serde_json::to_string_pretty(monitor)?;
    fs::write(path, rendered.as_bytes())?;
    Ok(())
}

fn build_channel_health_monitor_status(
    config_path: &str,
    workspace_root: &Path,
    config: &AppConfig,
    entries: &[ChannelProbeEntry],
) -> Result<ChannelHealthMonitorStatus> {
    let previous = load_channel_health_monitor_status(workspace_root)?;
    let service_install_status =
        runtime::runtime_service_install_status(config_path, workspace_root)?;
    Ok(compose_channel_health_monitor_status(
        config,
        service_install_status.supported && service_install_status.installed,
        previous.as_ref(),
        entries,
    ))
}

fn compose_channel_health_monitor_status(
    config: &AppConfig,
    auto_restart_ready: bool,
    previous: Option<&ChannelHealthMonitorStatus>,
    entries: &[ChannelProbeEntry],
) -> ChannelHealthMonitorStatus {
    let app_previous = previous.map(app_channel_health_monitor_status);
    let app_entries = entries
        .iter()
        .map(app_channel_probe_entry)
        .collect::<Vec<_>>();
    channel_health_monitor_status_from_app(
        app_channel_health_monitor::ChannelHealthMonitorService::new().compose_status(
            config,
            auto_restart_ready,
            app_previous.as_ref(),
            &app_entries,
        ),
    )
}

fn ready_entry(platform: &str, probe_kind: &str, detail: impl Into<String>) -> ChannelProbeEntry {
    ChannelProbeEntry {
        platform: platform.to_string(),
        enabled: true,
        status: ChannelProbeStatus::Ready,
        probe_kind: probe_kind.to_string(),
        detail: detail.into(),
    }
}

fn warning_entry(platform: &str, probe_kind: &str, detail: impl Into<String>) -> ChannelProbeEntry {
    ChannelProbeEntry {
        platform: platform.to_string(),
        enabled: true,
        status: ChannelProbeStatus::Warning,
        probe_kind: probe_kind.to_string(),
        detail: detail.into(),
    }
}

fn failed_entry(platform: &str, probe_kind: &str, detail: impl Into<String>) -> ChannelProbeEntry {
    ChannelProbeEntry {
        platform: platform.to_string(),
        enabled: true,
        status: ChannelProbeStatus::Failed,
        probe_kind: probe_kind.to_string(),
        detail: detail.into(),
    }
}

fn enabled_channels(config: &AppConfig) -> Vec<String> {
    app_channel_health_monitor::ChannelHealthMonitorService::new().enabled_channels(config)
}

#[cfg(test)]
mod tests {
    use super::{
        ChannelHealthMonitorStatus, ChannelProbeEntry, ChannelProbeStatus,
        compose_channel_health_monitor_status, enabled_channels, probe_whatsapp,
        runtime_events_with_pool,
    };
    use chrono::Utc;
    use openrustclaw_core::config::AppConfig;
    use tempfile::tempdir;
    use uuid::Uuid;

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

    #[test]
    fn channel_health_monitor_requests_restart_after_threshold() {
        let mut config = AppConfig::default();
        config.channels.runtime.auto_restart_on_failure = true;
        config.channels.runtime.failure_threshold = 2;

        let previous = ChannelHealthMonitorStatus {
            generated_at: Utc::now().to_rfc3339(),
            health_monitor_enabled: true,
            auto_restart_on_failure: true,
            auto_restart_ready: true,
            consecutive_failure_threshold: 2,
            current_consecutive_failures: 1,
            degraded: true,
            failing_platforms: vec!["slack".to_string()],
            last_failure_at: Some(Utc::now().to_rfc3339()),
            last_healthy_at: None,
            restart_requested: false,
            restart_reason: None,
        };
        let entries = vec![ChannelProbeEntry {
            platform: "slack".to_string(),
            enabled: true,
            status: ChannelProbeStatus::Failed,
            probe_kind: "remote_auth".to_string(),
            detail: "token rejected".to_string(),
        }];

        let monitor =
            compose_channel_health_monitor_status(&config, true, Some(&previous), &entries);
        assert!(monitor.restart_requested);
        assert_eq!(monitor.current_consecutive_failures, 2);
    }

    #[test]
    fn channel_health_monitor_resets_after_healthy_scan() {
        let mut config = AppConfig::default();
        config.channels.runtime.auto_restart_on_failure = true;

        let previous = ChannelHealthMonitorStatus {
            generated_at: Utc::now().to_rfc3339(),
            health_monitor_enabled: true,
            auto_restart_on_failure: true,
            auto_restart_ready: true,
            consecutive_failure_threshold: 3,
            current_consecutive_failures: 2,
            degraded: true,
            failing_platforms: vec!["telegram".to_string()],
            last_failure_at: Some(Utc::now().to_rfc3339()),
            last_healthy_at: None,
            restart_requested: false,
            restart_reason: None,
        };
        let entries = vec![ChannelProbeEntry {
            platform: "telegram".to_string(),
            enabled: true,
            status: ChannelProbeStatus::Ready,
            probe_kind: "remote_auth".to_string(),
            detail: "ok".to_string(),
        }];

        let monitor =
            compose_channel_health_monitor_status(&config, true, Some(&previous), &entries);
        assert!(!monitor.degraded);
        assert_eq!(monitor.current_consecutive_failures, 0);
        assert!(!monitor.restart_requested);
        assert!(monitor.last_healthy_at.is_some());
    }

    #[test]
    fn probe_whatsapp_requires_credentials_for_ready_status() {
        let temp = tempdir().unwrap();
        let session_dir = temp.path().join("whatsapp-session");
        std::fs::create_dir_all(&session_dir).unwrap();
        std::fs::write(temp.path().join("bridge.js"), "// bridge").unwrap();

        let mut config = AppConfig::default();
        config.channels.whatsapp.enabled = true;
        config.channels.whatsapp.bridge_path = temp.path().join("bridge.js").display().to_string();
        config.channels.whatsapp.session_path = session_dir.display().to_string();

        let warning = probe_whatsapp(&config.channels.whatsapp);
        assert_eq!(warning.status, ChannelProbeStatus::Warning);
        assert!(warning.detail.contains("pairing is not complete"));

        std::fs::write(session_dir.join("creds.json"), "{}").unwrap();

        let ready = probe_whatsapp(&config.channels.whatsapp);
        assert_eq!(ready.status, ChannelProbeStatus::Ready);
        assert!(ready.detail.contains("credentials are present"));
    }

    #[tokio::test]
    async fn runtime_events_with_pool_returns_payload_for_memory_searches() {
        let workspace = tempdir().expect("tempdir");
        let db_path = workspace.path().join("runtime-events.db");
        let db_url = format!("sqlite://{}", db_path.display());
        let pool = openrustclaw_db::init_pool(&db_url, 1).await.unwrap();
        openrustclaw_db::run_migrations(&pool).await.unwrap();

        let payload = serde_json::json!({
            "query": "ownership",
            "namespace": "user-1",
            "result_count": 1,
            "recall_pack": {
                "degraded": true,
                "items": [{
                    "id": Uuid::new_v4().to_string(),
                    "memory_type": "semantic",
                    "namespace": "user-1",
                    "content": "Rust ownership memory",
                    "score": 0.91,
                    "importance": 0.8,
                    "confidence": 0.9,
                    "explanation": {
                        "primary_artifact": {
                            "artifact_id": "artifact-1",
                            "artifact_kind": "conversation_memory",
                            "namespace": "user-1"
                        },
                        "contributing_artifacts": [{
                            "artifact_id": "artifact-1",
                            "artifact_kind": "conversation_memory",
                            "namespace": "user-1"
                        }],
                        "factors": {
                            "lexical_score": 0.8,
                            "vector_score": null,
                            "recency_score": 0.7,
                            "confidence_score": 0.9,
                            "importance_score": 0.8,
                            "fused_score": 0.91,
                            "vector_lane": "unavailable"
                        },
                        "freshness": null,
                        "degraded_state": {
                            "code": "vector_unavailable",
                            "message": "query embeddings unavailable"
                        }
                    }
                }]
            }
        });

        sqlx::query(
            r#"
            INSERT INTO runtime_events (id, event_name, event_type, session_id, payload, status, created_at)
            VALUES (?, 'memory.searched', 'memory_searched', NULL, ?, 'processed', datetime('now'))
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(payload.to_string())
        .execute(&pool)
        .await
        .unwrap();

        let events = runtime_events_with_pool(&pool, Some("memory.searched"), 10)
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_name, "memory.searched");
        assert_eq!(events[0].payload["query"], "ownership");
        assert_eq!(events[0].payload["recall_pack"]["degraded"], true);
        assert_eq!(
            events[0].payload["recall_pack"]["items"][0]["namespace"],
            "user-1"
        );
    }
}

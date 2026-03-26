use anyhow::Result;
use openrustclaw_core::types::{MemoryEntry, Message};
use openrustclaw_db::models::MemoryArchiveRow;
use openrustclaw_db::{
    PersistedSession, SessionStatus, SqliteMemoryStore, SqlitePool, SqliteSessionStore,
};
use serde::Serialize;
use sqlx::Row;

#[derive(Debug, Clone, Serialize)]
pub struct AssistantContinuitySummary {
    pub assistant_managed: bool,
    pub assistant_identity: Option<String>,
    pub assistant_surface: Option<String>,
    pub assistant_session_model: Option<String>,
    pub route_key: Option<String>,
    pub workspace_root: Option<String>,
    pub route_bound: bool,
    pub history_messages: usize,
    pub likely_resumed: bool,
    pub status_label: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionListEntry {
    pub session: PersistedSession,
    pub continuity: AssistantContinuitySummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionDetailReport {
    pub session: Option<PersistedSession>,
    pub history: Vec<Message>,
    pub continuity: Option<AssistantContinuitySummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryTimelineReport {
    pub namespace: Option<String>,
    pub entries: Vec<MemoryEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryArchiveReport {
    pub namespace: Option<String>,
    pub entries: Vec<MemoryArchiveRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub workflow_id: String,
    pub trigger_type: String,
    pub trigger_config: serde_json::Value,
    pub state: String,
    pub timezone: Option<String>,
    pub max_retries: i64,
    pub priority: i64,
    pub source_kind: Option<String>,
    pub owner: Option<String>,
    pub tags: Vec<String>,
    pub disabled_until: Option<String>,
    pub manifest_path: Option<String>,
    pub task_notes_path: Option<String>,
    pub next_run_at: Option<String>,
    pub last_run_at: Option<String>,
    pub run_count: i64,
    pub consecutive_failures: i64,
    pub metadata: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobRunSummary {
    pub id: String,
    pub job_id: String,
    pub name: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub retry_count: i64,
    pub langsmith_trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeadLetterSummary {
    pub id: String,
    pub job_id: String,
    pub name: String,
    pub last_error: String,
    pub failed_at: String,
    pub retry_count: i64,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobDetailReport {
    pub job: Option<JobSummary>,
    pub recent_runs: Vec<JobRunSummary>,
    pub dead_letters: Vec<DeadLetterSummary>,
}

pub async fn list_sessions(
    pool: &SqlitePool,
    status: Option<&str>,
    limit: usize,
) -> Result<Vec<SessionListEntry>> {
    let store = SqliteSessionStore::new(pool.clone());
    let parsed = status.and_then(parse_session_status);
    let sessions = store.list_sessions(parsed, limit.max(1)).await?;
    let mut entries = Vec::with_capacity(sessions.len());
    for session in sessions {
        let history_messages = count_history_messages(pool, &session.session.id.to_string()).await?;
        entries.push(SessionListEntry {
            continuity: assistant_continuity_summary(&session, history_messages),
            session,
        });
    }
    Ok(entries)
}

pub async fn inspect_session(
    pool: &SqlitePool,
    id: &str,
    history_limit: usize,
) -> Result<SessionDetailReport> {
    let store = SqliteSessionStore::new(pool.clone());
    let session = store.get_session(id).await?;
    let history = if session.is_some() {
        store.list_history(id, history_limit.max(1)).await?
    } else {
        Vec::new()
    };
    let continuity = match session.as_ref() {
        Some(session) => Some(assistant_continuity_summary(
            session,
            count_history_messages(pool, id).await?,
        )),
        None => None,
    };
    Ok(SessionDetailReport {
        session,
        history,
        continuity,
    })
}

pub fn assistant_continuity_summary(
    session: &PersistedSession,
    history_messages: usize,
) -> AssistantContinuitySummary {
    let assistant_identity = metadata_string(&session.session.metadata, "assistant_identity");
    let assistant_surface = metadata_string(&session.session.metadata, "assistant_surface");
    let assistant_session_model =
        metadata_string(&session.session.metadata, "assistant_session_model");
    let route_key = session
        .route_key
        .clone()
        .or_else(|| metadata_string(&session.session.metadata, "route_key"));
    let workspace_root = metadata_string(&session.session.metadata, "workspace_root");
    let route_bound = route_key.is_some();
    let assistant_managed = assistant_identity.is_some()
        || assistant_surface.is_some()
        || assistant_session_model.is_some();
    let likely_resumed = assistant_managed && route_bound && history_messages > 0;
    let status_label = if !assistant_managed {
        "generic"
    } else if likely_resumed {
        "resumed"
    } else if route_bound {
        "route-bound"
    } else if assistant_session_model.as_deref() == Some("persisted") {
        "persisted"
    } else {
        "assistant"
    }
    .to_string();
    let detail = continuity_detail(
        assistant_managed,
        assistant_surface.as_deref(),
        route_bound,
        history_messages,
    );

    AssistantContinuitySummary {
        assistant_managed,
        assistant_identity,
        assistant_surface,
        assistant_session_model,
        route_key,
        workspace_root,
        route_bound,
        history_messages,
        likely_resumed,
        status_label,
        detail,
    }
}

async fn count_history_messages(pool: &SqlitePool, session_id: &str) -> Result<usize> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM conversations WHERE session_id = ?")
        .bind(session_id)
        .fetch_one(pool)
        .await?;
    Ok(count.max(0) as usize)
}

fn metadata_string(metadata: &serde_json::Value, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
}

fn continuity_detail(
    assistant_managed: bool,
    assistant_surface: Option<&str>,
    route_bound: bool,
    history_messages: usize,
) -> String {
    let history_label = if history_messages == 1 {
        "1 persisted message".to_string()
    } else {
        format!("{history_messages} persisted messages")
    };

    if !assistant_managed {
        return format!("Generic session with {history_label}.");
    }

    let surface = assistant_surface.unwrap_or("assistant");
    if route_bound && history_messages > 0 {
        return format!(
            "Primary {surface} assistant session matched by route key with {history_label} restored."
        );
    }
    if route_bound {
        return format!(
            "Primary {surface} assistant session is route-bound and ready to accumulate persisted history."
        );
    }
    format!("Primary {surface} assistant session with {history_label}.")
}

pub async fn memory_timeline(
    store: &SqliteMemoryStore,
    namespace: Option<&str>,
    limit: usize,
) -> Result<MemoryTimelineReport> {
    let namespace_value = namespace.map(|value| value.to_string());
    let entries = store.list_recent(namespace, limit.max(1)).await?;
    Ok(MemoryTimelineReport {
        namespace: namespace_value,
        entries,
    })
}

pub async fn memory_namespaces(store: &SqliteMemoryStore) -> Result<Vec<String>> {
    Ok(store.list_namespaces().await?)
}

pub async fn memory_archive(
    store: &SqliteMemoryStore,
    namespace: Option<&str>,
    limit: usize,
) -> Result<MemoryArchiveReport> {
    let namespace_value = namespace.map(|value| value.to_string());
    let entries = store.list_archive_entries(namespace, limit.max(1)).await?;
    Ok(MemoryArchiveReport {
        namespace: namespace_value,
        entries,
    })
}

pub async fn list_jobs(
    pool: &SqlitePool,
    state: Option<&str>,
    limit: usize,
) -> Result<Vec<JobSummary>> {
    let mut sql = String::from(
        r#"
        SELECT id, name, description, workflow_id, trigger_type, trigger_config,
               state, timezone, max_retries, priority, source_kind, owner,
               tags, disabled_until, manifest_path, task_notes_path,
               next_run_at, last_run_at, run_count, consecutive_failures,
               metadata, created_at
        FROM scheduled_jobs
        "#,
    );

    let state = state.unwrap_or_default().trim().to_string();
    if state.is_empty() {
        sql.push_str(" ORDER BY priority ASC, next_run_at ASC, created_at DESC LIMIT ?");
    } else {
        sql.push_str(
            " WHERE state = ? ORDER BY priority ASC, next_run_at ASC, created_at DESC LIMIT ?",
        );
    }

    let mut query = sqlx::query(&sql);
    if !state.is_empty() {
        query = query.bind(state);
    }
    query = query.bind(limit.max(1) as i64);

    let rows = query.fetch_all(pool).await?;
    Ok(rows.into_iter().map(row_to_job_summary).collect())
}

pub async fn inspect_job(
    pool: &SqlitePool,
    id_or_name: &str,
    run_limit: usize,
    dead_letter_limit: usize,
) -> Result<JobDetailReport> {
    let job = sqlx::query(
        r#"
        SELECT id, name, description, workflow_id, trigger_type, trigger_config,
               state, timezone, max_retries, priority, source_kind, owner,
               tags, disabled_until, manifest_path, task_notes_path,
               next_run_at, last_run_at, run_count, consecutive_failures,
               metadata, created_at
        FROM scheduled_jobs
        WHERE id = ? OR name = ?
        LIMIT 1
        "#,
    )
    .bind(id_or_name)
    .bind(id_or_name)
    .fetch_optional(pool)
    .await?
    .map(|row| row_to_job_summary(row));

    let Some(job) = job else {
        return Ok(JobDetailReport {
            job: None,
            recent_runs: Vec::new(),
            dead_letters: Vec::new(),
        });
    };

    let recent_runs = sqlx::query(
        r#"
        SELECT r.id, r.job_id, j.name, r.status, r.started_at, r.completed_at,
               r.retry_count, r.langsmith_trace_id
        FROM job_runs r
        JOIN scheduled_jobs j ON j.id = r.job_id
        WHERE r.job_id = ? OR j.name = ?
        ORDER BY r.started_at DESC
        LIMIT ?
        "#,
    )
    .bind(&job.id)
    .bind(&job.name)
    .bind(run_limit.max(1) as i64)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| JobRunSummary {
        id: row.get("id"),
        job_id: row.get("job_id"),
        name: row.get("name"),
        status: row.get("status"),
        started_at: row.get("started_at"),
        completed_at: row.get("completed_at"),
        retry_count: row.get::<Option<i64>, _>("retry_count").unwrap_or(0),
        langsmith_trace_id: row.get("langsmith_trace_id"),
    })
    .collect();

    let dead_letters = sqlx::query(
        r#"
        SELECT d.id, d.job_id, j.name, d.last_error, d.failed_at, d.retry_count, d.resolved
        FROM dead_letter_queue d
        JOIN scheduled_jobs j ON j.id = d.job_id
        WHERE d.job_id = ? OR j.name = ?
        ORDER BY d.failed_at DESC
        LIMIT ?
        "#,
    )
    .bind(&job.id)
    .bind(&job.name)
    .bind(dead_letter_limit.max(1) as i64)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| DeadLetterSummary {
        id: row.get("id"),
        job_id: row.get("job_id"),
        name: row.get("name"),
        last_error: row.get("last_error"),
        failed_at: row.get("failed_at"),
        retry_count: row.get("retry_count"),
        resolved: row.get::<Option<i64>, _>("resolved").unwrap_or(0) == 1,
    })
    .collect();

    Ok(JobDetailReport {
        job: Some(job),
        recent_runs,
        dead_letters,
    })
}

fn parse_session_status(value: &str) -> Option<SessionStatus> {
    match value.trim().to_ascii_lowercase().as_str() {
        "active" => Some(SessionStatus::Active),
        "archived" => Some(SessionStatus::Archived),
        "closed" => Some(SessionStatus::Closed),
        _ => None,
    }
}

fn row_to_job_summary(row: sqlx::sqlite::SqliteRow) -> JobSummary {
    JobSummary {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        workflow_id: row.get("workflow_id"),
        trigger_type: row.get("trigger_type"),
        trigger_config: parse_json_value(row.get::<String, _>("trigger_config")),
        state: row
            .get::<Option<String>, _>("state")
            .unwrap_or_else(|| "active".to_string()),
        timezone: row.get("timezone"),
        max_retries: row.get::<Option<i64>, _>("max_retries").unwrap_or(0),
        priority: row.get::<Option<i64>, _>("priority").unwrap_or(0),
        source_kind: row.get("source_kind"),
        owner: row.get("owner"),
        tags: parse_string_array(row.get::<Option<String>, _>("tags")),
        disabled_until: row.get("disabled_until"),
        manifest_path: row.get("manifest_path"),
        task_notes_path: row.get("task_notes_path"),
        next_run_at: row.get("next_run_at"),
        last_run_at: row.get("last_run_at"),
        run_count: row.get::<Option<i64>, _>("run_count").unwrap_or(0),
        consecutive_failures: row
            .get::<Option<i64>, _>("consecutive_failures")
            .unwrap_or(0),
        metadata: parse_optional_json_value(row.get::<Option<String>, _>("metadata")),
        created_at: row.get("created_at"),
    }
}

fn parse_string_array(value: Option<String>) -> Vec<String> {
    value
        .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
        .unwrap_or_default()
}

fn parse_json_value(value: String) -> serde_json::Value {
    serde_json::from_str(&value).unwrap_or_else(|_| serde_json::json!({ "raw": value }))
}

fn parse_optional_json_value(value: Option<String>) -> serde_json::Value {
    value
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

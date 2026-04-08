//! Embedded database migrations for OpenRustClaw.
//!
//! All SQL schema definitions are compiled into the binary as `const` strings
//! and executed in order on startup. This avoids the need for an external
//! migrations directory or the `sqlx::migrate!()` macro.

use sqlx::{Row, SqlitePool};
use tracing::info;

use openrustclaw_core::error::{DatabaseError, Error, Result};

/// Individual migration with a name and SQL body.
struct Migration {
    name: &'static str,
    sql: &'static str,
}

/// All migrations in execution order.
const MIGRATIONS: &[Migration] = &[
    Migration {
        name: "001_sessions",
        sql: r#"
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    session_type TEXT NOT NULL CHECK(session_type IN ('dm', 'group', 'isolated')),
    user_id TEXT NOT NULL,
    channel TEXT NOT NULL,
    workspace_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    metadata TEXT DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'active',
    route_key TEXT,
    archived_at TEXT,
    closed_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_sessions_status_updated ON sessions(status, updated_at);
CREATE INDEX IF NOT EXISTS idx_sessions_route_key ON sessions(route_key, status);
"#,
    },
    Migration {
        name: "002_conversations",
        sql: r#"
CREATE TABLE IF NOT EXISTS conversations (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system', 'tool')),
    content TEXT NOT NULL,
    tool_calls TEXT,
    tool_call_id TEXT,
    token_count INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_conv_session ON conversations(session_id, created_at);
"#,
    },
    Migration {
        name: "003_memory",
        sql: r#"
CREATE TABLE IF NOT EXISTS memory_entries (
    id TEXT PRIMARY KEY,
    memory_type TEXT NOT NULL CHECK(memory_type IN ('episodic', 'semantic', 'procedural')),
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    source TEXT,
    source_type TEXT,
    session_id TEXT,
    user_id TEXT,
    namespace TEXT DEFAULT 'global',
    importance REAL DEFAULT 0.5,
    confidence REAL DEFAULT 1.0,
    access_count INTEGER DEFAULT 0,
    last_accessed TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT,
    metadata TEXT DEFAULT '{}'
);
CREATE INDEX IF NOT EXISTS idx_memory_type ON memory_entries(memory_type);
CREATE INDEX IF NOT EXISTS idx_memory_user ON memory_entries(user_id);
CREATE INDEX IF NOT EXISTS idx_memory_hash ON memory_entries(content_hash);
CREATE INDEX IF NOT EXISTS idx_memory_expires ON memory_entries(expires_at);
"#,
    },
    Migration {
        name: "004_memory_fts",
        sql: r#"
CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
    content,
    tokenize='porter unicode61'
);
CREATE TABLE IF NOT EXISTS memory_fts_mapping (
    fts_rowid INTEGER PRIMARY KEY,
    memory_id TEXT NOT NULL UNIQUE REFERENCES memory_entries(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_fts_mapping_memory ON memory_fts_mapping(memory_id);
"#,
    },
    Migration {
        name: "005_skills",
        sql: r#"
CREATE TABLE IF NOT EXISTS skills (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    source TEXT NOT NULL CHECK(source IN ('workspace', 'managed', 'bundled', 'marketplace')),
    version TEXT,
    signature TEXT,
    verified INTEGER DEFAULT 0,
    enabled INTEGER DEFAULT 1,
    capabilities TEXT,
    schema TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#,
    },
    Migration {
        name: "006_audit_log",
        sql: r#"
CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    session_id TEXT,
    user_id TEXT,
    details TEXT,
    severity TEXT DEFAULT 'info' CHECK(severity IN ('info', 'warn', 'error', 'critical')),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_audit_severity ON audit_log(severity, created_at);
"#,
    },
    Migration {
        name: "007_scheduled_jobs",
        sql: r#"
CREATE TABLE IF NOT EXISTS scheduled_jobs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    workflow_id TEXT NOT NULL,
    trigger_type TEXT NOT NULL CHECK(trigger_type IN ('interval', 'event', 'webhook', 'dependency', 'absolute')),
    trigger_config TEXT NOT NULL,
    idempotency_key TEXT UNIQUE,
    state TEXT DEFAULT 'active' CHECK(state IN ('active', 'paused', 'completed', 'failed', 'dead_letter')),
    lease_owner TEXT,
    lease_expires_at TEXT,
    timezone TEXT DEFAULT 'UTC',
    max_retries INTEGER DEFAULT 3,
    last_run_at TEXT,
    next_run_at TEXT,
    run_count INTEGER DEFAULT 0,
    consecutive_failures INTEGER DEFAULT 0,
    metadata TEXT DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_jobs_next_run ON scheduled_jobs(next_run_at, state);
CREATE INDEX IF NOT EXISTS idx_jobs_lease ON scheduled_jobs(lease_owner, lease_expires_at);
"#,
    },
    Migration {
        name: "008_job_runs",
        sql: r#"
CREATE TABLE IF NOT EXISTS job_runs (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES scheduled_jobs(id),
    idempotency_key TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('running', 'success', 'failure', 'timeout')),
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    result TEXT,
    langsmith_trace_id TEXT,
    retry_count INTEGER DEFAULT 0,
    metadata TEXT DEFAULT '{}'
);
CREATE INDEX IF NOT EXISTS idx_runs_job ON job_runs(job_id, started_at);
"#,
    },
    Migration {
        name: "009_dead_letter_queue",
        sql: r#"
CREATE TABLE IF NOT EXISTS dead_letter_queue (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES scheduled_jobs(id),
    last_error TEXT NOT NULL,
    failed_at TEXT NOT NULL DEFAULT (datetime('now')),
    retry_count INTEGER NOT NULL,
    original_payload TEXT,
    resolved INTEGER DEFAULT 0,
    resolved_at TEXT
);
"#,
    },
    Migration {
        name: "010_workflow_checkpoints",
        sql: r#"
CREATE TABLE IF NOT EXISTS workflow_checkpoints (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    thread_id TEXT NOT NULL,
    state TEXT NOT NULL,
    step INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_checkpoint_workflow ON workflow_checkpoints(workflow_id, thread_id, step);
"#,
    },
    Migration {
        name: "011_core_memory",
        sql: r#"
CREATE TABLE IF NOT EXISTS core_memory (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    importance REAL NOT NULL DEFAULT 0.8,
    token_count INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, key)
);
"#,
    },
    Migration {
        name: "012_memory_archive",
        sql: r#"
CREATE TABLE IF NOT EXISTS memory_archive (
    id TEXT PRIMARY KEY,
    summary TEXT NOT NULL,
    source_memory_ids TEXT NOT NULL,
    source_type TEXT,
    namespace TEXT DEFAULT 'global',
    importance REAL DEFAULT 0.5,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_archive_namespace ON memory_archive(namespace);
"#,
    },
    Migration {
        name: "013_rag_chunks",
        sql: r#"
CREATE TABLE IF NOT EXISTS rag_chunks (
    collection_name TEXT NOT NULL,
    chunk_id TEXT NOT NULL,
    source_id TEXT NOT NULL,
    chunk_index INTEGER NOT NULL DEFAULT 0,
    content TEXT NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY(collection_name, chunk_id)
);
CREATE INDEX IF NOT EXISTS idx_rag_chunks_collection ON rag_chunks(collection_name, chunk_index);
CREATE INDEX IF NOT EXISTS idx_rag_chunks_source ON rag_chunks(source_id);
"#,
    },
    Migration {
        name: "014_optimization_framework",
        sql: r#"
CREATE TABLE IF NOT EXISTS optimization_targets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    target_kind TEXT NOT NULL,
    execution_tier TEXT NOT NULL,
    risk_class TEXT NOT NULL,
    ship_status TEXT NOT NULL DEFAULT 'experimental',
    workspace_root TEXT NOT NULL,
    mutation_policy TEXT NOT NULL,
    eval_suite TEXT NOT NULL,
    promotion_policy TEXT NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_optimization_targets_kind ON optimization_targets(target_kind, execution_tier);
CREATE TABLE IF NOT EXISTS optimization_candidates (
    id TEXT PRIMARY KEY,
    target_id TEXT NOT NULL REFERENCES optimization_targets(id) ON DELETE CASCADE,
    hypothesis TEXT NOT NULL,
    proposed_by TEXT NOT NULL,
    change_set TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN (
        'draft',
        'approved',
        'running',
        'passed',
        'failed',
        'rejected',
        'promoted_experimental',
        'promoted_compat',
        'queued_rust_merge'
    )),
    diff_summary TEXT,
    result_summary TEXT,
    artifact_manifest TEXT NOT NULL DEFAULT '{}',
    trace_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_optimization_candidates_target ON optimization_candidates(target_id, created_at);
CREATE INDEX IF NOT EXISTS idx_optimization_candidates_status ON optimization_candidates(status, created_at);
CREATE TABLE IF NOT EXISTS optimization_evaluations (
    id TEXT PRIMARY KEY,
    candidate_id TEXT NOT NULL REFERENCES optimization_candidates(id) ON DELETE CASCADE,
    eval_name TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('success', 'failure', 'timeout')),
    exit_code INTEGER,
    duration_ms INTEGER NOT NULL DEFAULT 0,
    stdout TEXT NOT NULL DEFAULT '',
    stderr TEXT NOT NULL DEFAULT '',
    metrics TEXT NOT NULL DEFAULT '{}',
    trace_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_optimization_evaluations_candidate ON optimization_evaluations(candidate_id, created_at);
CREATE TABLE IF NOT EXISTS optimization_promotions (
    id TEXT PRIMARY KEY,
    candidate_id TEXT NOT NULL REFERENCES optimization_candidates(id) ON DELETE CASCADE,
    decision TEXT NOT NULL CHECK(decision IN (
        'reject',
        'keep_candidate',
        'approve',
        'promote_experimental',
        'promote_compat',
        'queue_rust_merge'
    )),
    decided_by TEXT NOT NULL,
    notes TEXT,
    rollback_reference TEXT,
    trace_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_optimization_promotions_candidate ON optimization_promotions(candidate_id, created_at);
"#,
    },
    Migration {
        name: "015_runtime_events",
        sql: r#"
CREATE TABLE IF NOT EXISTS runtime_events (
    id TEXT PRIMARY KEY,
    event_name TEXT NOT NULL,
    event_type TEXT NOT NULL,
    session_id TEXT,
    payload TEXT NOT NULL,
    dedupe_key TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'processed', 'failed')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    processed_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_runtime_events_name ON runtime_events(event_name, created_at);
CREATE INDEX IF NOT EXISTS idx_runtime_events_status ON runtime_events(status, created_at);
"#,
    },
    Migration {
        name: "016_event_dispatch_queue",
        sql: r#"
CREATE TABLE IF NOT EXISTS event_dispatch_queue (
    id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL REFERENCES runtime_events(id) ON DELETE CASCADE,
    job_id TEXT NOT NULL REFERENCES scheduled_jobs(id) ON DELETE CASCADE,
    state TEXT NOT NULL DEFAULT 'pending' CHECK(state IN ('pending', 'leased', 'completed', 'failed', 'dead_letter')),
    lease_owner TEXT,
    lease_expires_at TEXT,
    attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_error TEXT,
    payload TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    UNIQUE(event_id, job_id)
);
CREATE INDEX IF NOT EXISTS idx_event_dispatch_queue_state ON event_dispatch_queue(state, next_attempt_at);
CREATE INDEX IF NOT EXISTS idx_event_dispatch_queue_job ON event_dispatch_queue(job_id, created_at);
"#,
    },
    Migration {
        name: "017_task_manifests",
        sql: r#"
CREATE TABLE IF NOT EXISTS task_manifests (
    job_id TEXT PRIMARY KEY REFERENCES scheduled_jobs(id) ON DELETE CASCADE,
    manifest_path TEXT NOT NULL UNIQUE,
    manifest_hash TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    origin TEXT NOT NULL DEFAULT 'filesystem',
    imported_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_task_manifests_path ON task_manifests(manifest_path);
"#,
    },
    Migration {
        name: "018_memory_model_artifacts",
        sql: r#"
CREATE TABLE IF NOT EXISTS memory_model_artifacts (
    id TEXT PRIMARY KEY,
    namespace TEXT NOT NULL DEFAULT 'global',
    kind TEXT NOT NULL CHECK(kind IN ('user_model', 'operator_model', 'project_memory', 'archive_summary')),
    summary TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'inactive', 'superseded', 'removed')),
    importance REAL NOT NULL DEFAULT 0.8,
    confidence REAL NOT NULL DEFAULT 1.0,
    source_lineage TEXT NOT NULL DEFAULT '[]',
    promoted_by TEXT,
    correction_note TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    deactivated_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_memory_model_artifacts_namespace
    ON memory_model_artifacts(namespace, kind, status, created_at);
"#,
    },
    Migration {
        name: "019_learning_candidates",
        sql: r#"
CREATE TABLE IF NOT EXISTS learning_candidates (
    id TEXT PRIMARY KEY,
    namespace TEXT NOT NULL DEFAULT 'global',
    kind TEXT NOT NULL,
    signal TEXT NOT NULL,
    recommendation TEXT NOT NULL,
    rationale TEXT,
    confidence REAL NOT NULL DEFAULT 0.7,
    impact TEXT NOT NULL DEFAULT 'standard' CHECK(impact IN ('standard', 'high')),
    status TEXT NOT NULL DEFAULT 'pending_review' CHECK(status IN ('pending_review', 'approved', 'rejected', 'superseded', 'promoted', 'rolled_back')),
    source_kind TEXT NOT NULL CHECK(source_kind IN ('reflection_candidate', 'audit_record', 'runtime_event', 'model_artifact', 'manual')),
    source_id TEXT NOT NULL,
    source_detail TEXT,
    review_note TEXT,
    reviewed_by TEXT,
    task_id TEXT,
    category TEXT,
    claw_id TEXT,
    model_profile_id TEXT,
    provider TEXT,
    autonomy_level TEXT,
    execution_mode TEXT,
    promoted_lesson_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    reviewed_at TEXT,
    rolled_back_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_learning_candidates_status
    ON learning_candidates(namespace, status, created_at);
CREATE INDEX IF NOT EXISTS idx_learning_candidates_source
    ON learning_candidates(source_kind, source_id);

CREATE TABLE IF NOT EXISTS learning_candidate_evidence (
    id TEXT PRIMARY KEY,
    candidate_id TEXT NOT NULL REFERENCES learning_candidates(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK(kind IN ('replay', 'evaluation', 'audit', 'runtime_event', 'operator_review')),
    summary TEXT NOT NULL,
    source_id TEXT,
    recorded_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_learning_candidate_evidence_candidate
    ON learning_candidate_evidence(candidate_id, created_at);

CREATE TABLE IF NOT EXISTS learning_candidate_history (
    id TEXT PRIMARY KEY,
    candidate_id TEXT NOT NULL REFERENCES learning_candidates(id) ON DELETE CASCADE,
    action TEXT NOT NULL,
    lesson_id TEXT,
    actor TEXT,
    note TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_learning_candidate_history_candidate
    ON learning_candidate_history(candidate_id, created_at);
"#,
    },
    Migration {
        name: "020_skill_proposals",
        sql: r#"
CREATE TABLE IF NOT EXISTS skill_proposals (
    id TEXT PRIMARY KEY,
    namespace TEXT NOT NULL DEFAULT 'global',
    skill_name TEXT NOT NULL,
    summary TEXT NOT NULL,
    body TEXT NOT NULL,
    rationale TEXT,
    status TEXT NOT NULL DEFAULT 'pending_review' CHECK(status IN ('pending_review', 'approved', 'rejected', 'superseded', 'installed', 'rolled_back')),
    verification_status TEXT NOT NULL DEFAULT 'pending' CHECK(verification_status IN ('pending', 'passed', 'failed', 'blocked')),
    source_kind TEXT NOT NULL CHECK(source_kind IN ('learning_candidate', 'lesson', 'manual')),
    source_id TEXT NOT NULL,
    source_detail TEXT,
    artifact_path TEXT NOT NULL,
    review_note TEXT,
    reviewed_by TEXT,
    verification_summary TEXT,
    verification_compiled_skill_name TEXT,
    verification_artifact_path TEXT,
    verification_blocked INTEGER NOT NULL DEFAULT 0,
    verified_by TEXT,
    installed_skill_name TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    reviewed_at TEXT,
    verified_at TEXT,
    installed_at TEXT,
    rolled_back_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_skill_proposals_status
    ON skill_proposals(namespace, status, created_at);
CREATE INDEX IF NOT EXISTS idx_skill_proposals_source
    ON skill_proposals(source_kind, source_id);
CREATE INDEX IF NOT EXISTS idx_skill_proposals_skill_name
    ON skill_proposals(skill_name, installed_skill_name);

CREATE TABLE IF NOT EXISTS skill_proposal_history (
    id TEXT PRIMARY KEY,
    proposal_id TEXT NOT NULL REFERENCES skill_proposals(id) ON DELETE CASCADE,
    action TEXT NOT NULL,
    actor TEXT,
    note TEXT,
    verification_status TEXT,
    installed_skill_name TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_skill_proposal_history_proposal
    ON skill_proposal_history(proposal_id, created_at);
"#,
    },
];

/// Run all embedded database migrations in order.
///
/// Each migration may contain multiple SQL statements separated by semicolons.
/// Statements are executed individually within a single transaction so that
/// either all migrations succeed or none are applied.
pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    for migration in MIGRATIONS {
        // Split on semicolons to handle multi-statement migrations
        // (e.g. CREATE TABLE + CREATE INDEX in the same migration).
        for statement in migration.sql.split(';') {
            let trimmed = statement.trim();
            if trimmed.is_empty() {
                continue;
            }
            sqlx::query(trimmed).execute(pool).await.map_err(|e| {
                Error::Database(DatabaseError::Migration(format!(
                    "Migration '{}' failed: {}",
                    migration.name, e
                )))
            })?;
        }
        info!("Applied migration: {}", migration.name);
    }

    ensure_scheduler_task_registry_columns(pool).await?;
    ensure_phase4_session_columns(pool).await?;

    info!("All {} database migrations completed", MIGRATIONS.len());
    Ok(())
}

async fn column_exists(pool: &SqlitePool, table: &str, column: &str) -> Result<bool> {
    let pragma = format!("PRAGMA table_info({table})");
    let rows = sqlx::query(&pragma).fetch_all(pool).await.map_err(|e| {
        Error::Database(DatabaseError::Migration(format!(
            "failed to inspect table '{table}': {e}"
        )))
    })?;

    Ok(rows.iter().any(|row| {
        row.try_get::<String, _>("name")
            .map(|value| value == column)
            .unwrap_or(false)
    }))
}

async fn add_column_if_missing(
    pool: &SqlitePool,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    if column_exists(pool, table, column).await? {
        return Ok(());
    }

    let statement = format!("ALTER TABLE {table} ADD COLUMN {column} {definition}");
    sqlx::query(&statement).execute(pool).await.map_err(|e| {
        Error::Database(DatabaseError::Migration(format!(
            "failed to add column '{table}.{column}': {e}"
        )))
    })?;

    Ok(())
}

async fn ensure_scheduler_task_registry_columns(pool: &SqlitePool) -> Result<()> {
    add_column_if_missing(
        pool,
        "scheduled_jobs",
        "priority",
        "INTEGER NOT NULL DEFAULT 100",
    )
    .await?;
    add_column_if_missing(
        pool,
        "scheduled_jobs",
        "source_kind",
        "TEXT NOT NULL DEFAULT 'cli'",
    )
    .await?;
    add_column_if_missing(pool, "scheduled_jobs", "owner", "TEXT").await?;
    add_column_if_missing(pool, "scheduled_jobs", "tags", "TEXT NOT NULL DEFAULT '[]'").await?;
    add_column_if_missing(pool, "scheduled_jobs", "disabled_until", "TEXT").await?;
    add_column_if_missing(pool, "scheduled_jobs", "manifest_path", "TEXT").await?;
    add_column_if_missing(pool, "scheduled_jobs", "task_notes_path", "TEXT").await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_jobs_priority_due ON scheduled_jobs(state, priority, next_run_at)",
    )
    .execute(pool)
    .await
    .map_err(|e| {
        Error::Database(DatabaseError::Migration(format!(
            "failed to create scheduler priority index: {e}"
        )))
    })?;

    Ok(())
}

async fn ensure_phase4_session_columns(pool: &SqlitePool) -> Result<()> {
    add_column_if_missing(pool, "sessions", "status", "TEXT NOT NULL DEFAULT 'active'").await?;
    add_column_if_missing(pool, "sessions", "route_key", "TEXT").await?;
    add_column_if_missing(pool, "sessions", "archived_at", "TEXT").await?;
    add_column_if_missing(pool, "sessions", "closed_at", "TEXT").await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_sessions_status_updated ON sessions(status, updated_at)",
    )
    .execute(pool)
    .await
    .map_err(|e| {
        Error::Database(DatabaseError::Migration(format!(
            "failed to create session status index: {e}"
        )))
    })?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_route_key ON sessions(route_key, status)")
        .execute(pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Migration(format!(
                "failed to create session route-key index: {e}"
            )))
        })?;

    Ok(())
}

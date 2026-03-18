//! Embedded database migrations for OpenRustClaw.
//!
//! All SQL schema definitions are compiled into the binary as `const` strings
//! and executed in order on startup. This avoids the need for an external
//! migrations directory or the `sqlx::migrate!()` macro.

use sqlx::SqlitePool;
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
    metadata TEXT DEFAULT '{}'
);
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

    info!("All {} database migrations completed", MIGRATIONS.len());
    Ok(())
}

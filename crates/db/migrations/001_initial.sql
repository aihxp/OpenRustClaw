-- OpenRustClaw Initial Database Schema
-- This migration creates all tables needed for the OpenRustClaw memory system

-- Sessions table: tracks conversation sessions
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

CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at);

-- Conversations table: stores messages within sessions
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

-- Memory entries table: the main recall memory storage
CREATE TABLE IF NOT EXISTS memory_entries (
    id TEXT PRIMARY KEY,
    memory_type TEXT NOT NULL CHECK(memory_type IN ('episodic', 'semantic', 'procedural')),
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL UNIQUE,
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
CREATE INDEX IF NOT EXISTS idx_memory_namespace ON memory_entries(namespace);
CREATE INDEX IF NOT EXISTS idx_memory_expires ON memory_entries(expires_at) WHERE expires_at IS NOT NULL;

-- Memory FTS5 virtual table: full-text search index
-- Tokenize with porter (stemming) + unicode61 (Unicode word characters)
-- Stores both memory_id and content for full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
    content,
    tokenize='porter unicode61'
);

-- Create a mapping table to link FTS5 rowid to memory_entries.id
CREATE TABLE IF NOT EXISTS memory_fts_mapping (
    fts_rowid INTEGER PRIMARY KEY,
    memory_id TEXT NOT NULL UNIQUE REFERENCES memory_entries(id) ON DELETE CASCADE
);

-- Memory vectors table: stores vector embeddings separately for libSQL compatibility
-- Using BLOB storage for compact vector representation
CREATE TABLE IF NOT EXISTS memory_vectors (
    memory_id TEXT PRIMARY KEY REFERENCES memory_entries(id) ON DELETE CASCADE,
    vector BLOB NOT NULL,  -- Binary representation of f32 array
    dimensions INTEGER NOT NULL,
    model_id TEXT,         -- Which embedding model generated this vector
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_vectors_model ON memory_vectors(model_id);

-- Core memory table: tiny key-value memory that stays in prompt (~500 tokens budget)
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

CREATE INDEX IF NOT EXISTS idx_core_memory_user ON core_memory(user_id);

-- Memory archive table: stores summarized/condensed memories
CREATE TABLE IF NOT EXISTS memory_archive (
    id TEXT PRIMARY KEY,
    summary TEXT NOT NULL,
    source_memory_ids TEXT NOT NULL,  -- JSON array of original memory IDs
    source_type TEXT,
    namespace TEXT DEFAULT 'global',
    importance REAL DEFAULT 0.5,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_archive_namespace ON memory_archive(namespace);

-- Skills table: registered tools/skills
CREATE TABLE IF NOT EXISTS skills (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    source TEXT NOT NULL CHECK(source IN ('workspace', 'managed', 'bundled', 'marketplace')),
    version TEXT,
    signature TEXT,
    verified INTEGER DEFAULT 0,
    enabled INTEGER DEFAULT 1,
    capabilities TEXT,  -- JSON array of required capabilities
    schema TEXT,        -- JSON schema for the skill
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_skills_enabled ON skills(enabled);

-- Audit log table: security and operational events
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
CREATE INDEX IF NOT EXISTS idx_audit_session ON audit_log(session_id);

-- Scheduled jobs table: cron-like job scheduling
CREATE TABLE IF NOT EXISTS scheduled_jobs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    workflow_id TEXT NOT NULL,
    trigger_type TEXT NOT NULL CHECK(trigger_type IN ('interval', 'event', 'webhook', 'dependency', 'absolute')),
    trigger_config TEXT NOT NULL,  -- JSON configuration for the trigger
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
CREATE INDEX IF NOT EXISTS idx_jobs_state ON scheduled_jobs(state);

-- Job runs table: execution history of scheduled jobs
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
CREATE INDEX IF NOT EXISTS idx_runs_status ON job_runs(status);

-- Dead letter queue: failed jobs that exceeded retry limits
CREATE TABLE IF NOT EXISTS dead_letter_queue (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES scheduled_jobs(id),
    run_id TEXT REFERENCES job_runs(id),
    last_error TEXT NOT NULL,
    failed_at TEXT NOT NULL DEFAULT (datetime('now')),
    retry_count INTEGER NOT NULL,
    original_payload TEXT,
    resolved INTEGER DEFAULT 0,
    resolved_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_dlq_unresolved ON dead_letter_queue(resolved, failed_at);
CREATE INDEX IF NOT EXISTS idx_dlq_job ON dead_letter_queue(job_id);

-- Workflow checkpoints: LangGraph-style state persistence
CREATE TABLE IF NOT EXISTS workflow_checkpoints (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    thread_id TEXT NOT NULL,
    state TEXT NOT NULL,  -- JSON serialized workflow state
    step INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_checkpoint_workflow ON workflow_checkpoints(workflow_id, thread_id, step);
CREATE INDEX IF NOT EXISTS idx_checkpoint_thread ON workflow_checkpoints(thread_id, step DESC);

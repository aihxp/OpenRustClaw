//! Database row structs for OpenRustClaw.
//!
//! Each struct maps directly to a database table and derives `sqlx::FromRow`
//! for automatic deserialization from query results. These are "row" types
//! that mirror the SQL schema exactly — all TEXT columns are `String` (or
//! `Option<String>` when nullable), INTEGER columns are `i64`, and REAL
//! columns are `f64`.
//!
//! Higher-level domain types live in `openrustclaw_core::types`; conversion
//! between row types and domain types is handled by repository modules.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ──────────────────────────────────────────────
// Sessions
// ──────────────────────────────────────────────

/// A row from the `sessions` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SessionRow {
    pub id: String,
    pub session_type: String,
    pub user_id: String,
    pub channel: String,
    pub workspace_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata: Option<String>,
    pub status: Option<String>,
    pub route_key: Option<String>,
    pub archived_at: Option<String>,
    pub closed_at: Option<String>,
}

// ──────────────────────────────────────────────
// Conversations
// ──────────────────────────────────────────────

/// A row from the `conversations` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConversationRow {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<String>,
    pub tool_call_id: Option<String>,
    pub token_count: Option<i64>,
    pub created_at: String,
}

// ──────────────────────────────────────────────
// Memory entries
// ──────────────────────────────────────────────

/// A row from the `memory_entries` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MemoryEntryRow {
    pub id: String,
    pub memory_type: String,
    pub content: String,
    pub content_hash: String,
    pub source: Option<String>,
    pub source_type: Option<String>,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub namespace: Option<String>,
    pub importance: Option<f64>,
    pub confidence: Option<f64>,
    pub access_count: Option<i64>,
    pub last_accessed: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub metadata: Option<String>,
}

/// A row from the `memory_vectors` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MemoryVectorRow {
    pub memory_id: String,
    pub vector: Vec<u8>,
    pub dimensions: i64,
    pub model_id: Option<String>,
    pub created_at: String,
}

// ──────────────────────────────────────────────
// Skills
// ──────────────────────────────────────────────

/// A row from the `skills` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub source: String,
    pub version: Option<String>,
    pub signature: Option<String>,
    pub verified: Option<i64>,
    pub enabled: Option<i64>,
    pub capabilities: Option<String>,
    pub schema: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ──────────────────────────────────────────────
// Audit log
// ──────────────────────────────────────────────

/// A row from the `audit_log` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLogRow {
    pub id: i64,
    pub event_type: String,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub details: Option<String>,
    pub severity: Option<String>,
    pub created_at: String,
}

// ──────────────────────────────────────────────
// Scheduled jobs
// ──────────────────────────────────────────────

/// A row from the `scheduled_jobs` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScheduledJobRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub workflow_id: String,
    pub trigger_type: String,
    pub trigger_config: String,
    pub idempotency_key: Option<String>,
    pub state: Option<String>,
    pub lease_owner: Option<String>,
    pub lease_expires_at: Option<String>,
    pub timezone: Option<String>,
    pub max_retries: Option<i64>,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub run_count: Option<i64>,
    pub consecutive_failures: Option<i64>,
    pub metadata: Option<String>,
    pub created_at: String,
}

// ──────────────────────────────────────────────
// Job runs
// ──────────────────────────────────────────────

/// A row from the `job_runs` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct JobRunRow {
    pub id: String,
    pub job_id: String,
    pub idempotency_key: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub result: Option<String>,
    pub langsmith_trace_id: Option<String>,
    pub retry_count: Option<i64>,
    pub metadata: Option<String>,
}

// ──────────────────────────────────────────────
// Dead letter queue
// ──────────────────────────────────────────────

/// A row from the `dead_letter_queue` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeadLetterRow {
    pub id: String,
    pub job_id: String,
    pub last_error: String,
    pub failed_at: String,
    pub retry_count: i64,
    pub original_payload: Option<String>,
    pub resolved: Option<i64>,
    pub resolved_at: Option<String>,
}

// ──────────────────────────────────────────────
// Workflow checkpoints
// ──────────────────────────────────────────────

/// A row from the `workflow_checkpoints` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WorkflowCheckpointRow {
    pub id: String,
    pub workflow_id: String,
    pub thread_id: String,
    pub state: String,
    pub step: i64,
    pub created_at: String,
}

// ──────────────────────────────────────────────
// Core memory
// ──────────────────────────────────────────────

/// A row from the `core_memory` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CoreMemoryRow {
    pub id: String,
    pub user_id: String,
    pub key: String,
    pub value: String,
    pub importance: f64,
    pub token_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

// ──────────────────────────────────────────────
// Memory archive
// ──────────────────────────────────────────────

/// A row from the `memory_archive` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MemoryArchiveRow {
    pub id: String,
    pub summary: String,
    pub source_memory_ids: String,
    pub source_type: Option<String>,
    pub namespace: Option<String>,
    pub importance: Option<f64>,
    pub created_at: String,
}

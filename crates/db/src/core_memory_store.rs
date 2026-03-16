//! Core Memory Store implementation for OpenRustClaw.
//!
//! Provides SQLite-based storage for core memory - a small, high-priority
//! key-value store that stays in the system prompt (~500 tokens budget).

use async_trait::async_trait;
use chrono::Utc;
use sqlx::SqlitePool;
use tracing::{debug, instrument, warn};
use uuid::Uuid;

use openrustclaw_core::error::{DatabaseError, Error, MemoryError, Result};
use openrustclaw_core::traits::CoreMemoryStore as CoreMemoryStoreTrait;
use openrustclaw_core::types::CoreEntry;

use crate::models::CoreMemoryRow;

/// Default maximum tokens for core memory.
pub const DEFAULT_CORE_MEMORY_BUDGET: usize = 500;

/// SQLite-based implementation of the CoreMemoryStore trait.
///
/// Core memory is designed to be:
/// - Small and focused (~500 tokens)
/// - Always loaded in the system prompt
/// - Key-value based for structured access
/// - Priority-weighted for intelligent trimming
#[derive(Debug, Clone)]
pub struct SqliteCoreMemoryStore {
    pool: SqlitePool,
    /// Maximum token budget for core memory
    budget: usize,
}

impl SqliteCoreMemoryStore {
    /// Create a new core memory store with the given database pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            budget: DEFAULT_CORE_MEMORY_BUDGET,
        }
    }

    /// Set a custom token budget.
    pub fn with_budget(mut self, budget: usize) -> Self {
        self.budget = budget;
        self
    }

    /// Convert a CoreMemoryRow to a CoreEntry domain type.
    fn row_to_entry(row: &CoreMemoryRow) -> Result<CoreEntry> {
        let updated_at = chrono::DateTime::parse_from_rfc3339(&row.updated_at)
            .map_err(|e| {
                Error::Database(DatabaseError::Query(format!("Invalid updated_at date: {}", e)))
            })?
            .with_timezone(&Utc);

        Ok(CoreEntry {
            key: row.key.clone(),
            value: row.value.clone(),
            importance: row.importance as f32,
            token_count: row.token_count as usize,
            updated_at,
        })
    }

    /// Estimate token count for a string.
    ///
    /// Uses a simple heuristic: ~4 characters per token on average.
    /// For production use, consider using tiktoken-rs or similar.
    fn estimate_tokens(text: &str) -> usize {
        // Rough approximation: 1 token ≈ 4 characters
        (text.len() / 4).max(1)
    }

    /// Format core entries as a string for system prompt injection.
    fn format_entries(entries: &[CoreEntry]) -> String {
        if entries.is_empty() {
            return "No core memory entries.".to_string();
        }

        let mut parts = vec!["## Core Memory".to_string()];

        for entry in entries {
            parts.push(format!("- **{}**: {}", entry.key, entry.value));
        }

        parts.join("\n")
    }

    /// Trim entries to fit within budget by removing lowest importance entries.
    fn trim_to_budget(entries: &mut Vec<CoreEntry>, budget: usize) {
        // Sort by importance descending, then by updated_at descending
        entries.sort_by(|a, b| {
            b.importance
                .partial_cmp(&a.importance)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.updated_at.cmp(&a.updated_at))
        });

        let mut total_tokens: usize = entries.iter().map(|e| e.token_count).sum();

        // Remove lowest importance entries until we fit the budget
        while total_tokens > budget && entries.len() > 1 {
            if let Some(removed) = entries.pop() {
                total_tokens -= removed.token_count;
                warn!(
                    "Trimmed core memory entry '{}' ({} tokens) to fit budget",
                    removed.key, removed.token_count
                );
            }
        }
    }
}

#[async_trait]
impl CoreMemoryStoreTrait for SqliteCoreMemoryStore {
    #[instrument(skip(self))]
    async fn get_all(&self, user_id: &str) -> Result<Vec<CoreEntry>> {
        let rows: Vec<CoreMemoryRow> = sqlx::query_as(
            r#"
            SELECT * FROM core_memory
            WHERE user_id = ?
            ORDER BY importance DESC, updated_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to get core memory: {}",
                e
            )))
        })?;

        rows.iter().map(Self::row_to_entry).collect()
    }

    #[instrument(skip(self, entry))]
    async fn set(&self, user_id: &str, entry: CoreEntry) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        // Check current token usage
        let current_tokens: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(token_count), 0) FROM core_memory
            WHERE user_id = ? AND key != ?
            "#,
        )
        .bind(user_id)
        .bind(&entry.key)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to check token budget: {}",
                e
            )))
        })?;

        let new_total = current_tokens as usize + entry.token_count;

        if new_total > self.budget {
            return Err(Error::Memory(MemoryError::CoreMemoryBudgetExceeded {
                used: new_total,
                max: self.budget,
            }));
        }

        sqlx::query(
            r#"
            INSERT INTO core_memory (id, user_id, key, value, importance, token_count, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(user_id, key) DO UPDATE SET
                value = excluded.value,
                importance = excluded.importance,
                token_count = excluded.token_count,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&entry.key)
        .bind(&entry.value)
        .bind(entry.importance as f64)
        .bind(entry.token_count as i64)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to set core memory: {}",
                e
            )))
        })?;

        debug!(
            "Set core memory entry '{}' for user {}",
            entry.key, user_id
        );
        Ok(())
    }

    #[instrument(skip(self))]
    async fn remove(&self, user_id: &str, key: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM core_memory
            WHERE user_id = ? AND key = ?
            "#,
        )
        .bind(user_id)
        .bind(key)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to remove core memory: {}",
                e
            )))
        })?;

        if result.rows_affected() == 0 {
            return Err(Error::Database(DatabaseError::NotFound {
                entity: "CoreMemory".to_string(),
                id: format!("{}/{}", user_id, key),
            }));
        }

        debug!("Removed core memory entry '{}' for user {}", key, user_id);
        Ok(())
    }

    #[instrument(skip(self))]
    async fn render(&self, user_id: &str) -> Result<String> {
        let entries = self.get_all(user_id).await?;

        // If we're over budget, trim (this shouldn't happen with proper set validation,
        // but serves as a safety net)
        let mut entries = entries;
        let total_tokens: usize = entries.iter().map(|e| e.token_count).sum();

        if total_tokens > self.budget {
            warn!(
                "Core memory over budget ({} > {}), trimming",
                total_tokens, self.budget
            );
            Self::trim_to_budget(&mut entries, self.budget);
        }

        Ok(Self::format_entries(&entries))
    }

    #[instrument(skip(self))]
    async fn total_tokens(&self, user_id: &str) -> Result<usize> {
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(token_count), 0) FROM core_memory
            WHERE user_id = ?
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to count tokens: {}",
                e
            )))
        })?;

        Ok(total as usize)
    }
}

/// Builder for constructing CoreEntry instances with sensible defaults.
pub struct CoreEntryBuilder {
    key: String,
    value: String,
    importance: f32,
}

impl CoreEntryBuilder {
    /// Create a new builder with the given key and value.
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            importance: 0.8,
        }
    }

    /// Set the importance level (0.0 - 1.0).
    pub fn importance(mut self, importance: f32) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    /// Build the CoreEntry.
    pub fn build(self) -> CoreEntry {
        let token_count = SqliteCoreMemoryStore::estimate_tokens(&self.value);
        CoreEntry {
            key: self.key,
            value: self.value,
            importance: self.importance,
            token_count,
            updated_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        // Roughly 4 chars per token (using integer division)
        assert_eq!(SqliteCoreMemoryStore::estimate_tokens("hello"), 1);
        assert_eq!(SqliteCoreMemoryStore::estimate_tokens("this is a longer sentence"), 6);
        // Single character is still 1 token due to max(1, ...)
        assert_eq!(SqliteCoreMemoryStore::estimate_tokens("x"), 1);
    }

    #[test]
    fn test_format_entries() {
        let entries = vec![
            CoreEntry {
                key: "name".to_string(),
                value: "Alice".to_string(),
                importance: 0.9,
                token_count: 2,
                updated_at: Utc::now(),
            },
            CoreEntry {
                key: "preference".to_string(),
                value: "likes rust".to_string(),
                importance: 0.7,
                token_count: 3,
                updated_at: Utc::now(),
            },
        ];

        let formatted = SqliteCoreMemoryStore::format_entries(&entries);
        assert!(formatted.contains("## Core Memory"));
        assert!(formatted.contains("**name**: Alice"));
        assert!(formatted.contains("**preference**: likes rust"));
    }

    #[test]
    fn test_trim_to_budget() {
        let mut entries = vec![
            CoreEntry {
                key: "high_priority".to_string(),
                value: "important".to_string(),
                importance: 0.9,
                token_count: 10,
                updated_at: Utc::now(),
            },
            CoreEntry {
                key: "medium_priority".to_string(),
                value: "somewhat important".to_string(),
                importance: 0.6,
                token_count: 10,
                updated_at: Utc::now(),
            },
            CoreEntry {
                key: "low_priority".to_string(),
                value: "not important".to_string(),
                importance: 0.3,
                token_count: 10,
                updated_at: Utc::now(),
            },
        ];

        SqliteCoreMemoryStore::trim_to_budget(&mut entries, 25);

        // Should keep high and medium, remove low
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "high_priority");
        assert_eq!(entries[1].key, "medium_priority");
    }

    #[test]
    fn test_core_entry_builder() {
        let entry = CoreEntryBuilder::new("test_key", "test value")
            .importance(0.95)
            .build();

        assert_eq!(entry.key, "test_key");
        assert_eq!(entry.value, "test value");
        assert_eq!(entry.importance, 0.95);
        assert!(entry.token_count > 0);
    }
}

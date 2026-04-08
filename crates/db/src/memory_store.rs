//! Memory Store implementation for OpenRustClaw.
//!
//! Provides SQLite-based storage for the recall memory system with:
//! - Full-text search via FTS5
//! - Vector storage for similarity search
//! - Hybrid ranking: BM25 + vector similarity + MMR + temporal decay

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, Row, SqlitePool};
use std::collections::HashMap;
use tracing::{debug, instrument};
use uuid::Uuid;

use openrustclaw_core::error::{DatabaseError, Error, MemoryError, Result};
use openrustclaw_core::traits::MemoryStore as MemoryStoreTrait;
use openrustclaw_core::types::{
    MemoryEntry, MemoryQuery, MemoryType, ModelArtifact, ModelArtifactKind,
    ModelArtifactPromotionRequest, ModelArtifactStatus, ModelArtifactUpdateRequest,
    RetrievalArtifactKind, RetrievalArtifactRef, RetrievalDegradedCode, RetrievalDegradedState,
    RetrievalExplanation, RetrievalFreshness, RetrievalScoreFactors, RetrievalVectorLane,
    ScoredMemory, SourceType,
};

use crate::models::MemoryArchiveRow;
use crate::models::MemoryEntryRow;
use crate::models::MemoryModelArtifactRow;

#[derive(Debug, FromRow)]
struct MemorySearchRow {
    #[sqlx(flatten)]
    entry: MemoryEntryRow,
    #[sqlx(rename = "rank")]
    _rank: f64,
}

/// Trait for embedding providers to generate vector representations.
///
/// This is a simplified interface that can be implemented by any
/// embedding service (OpenAI, local models, etc.).
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Embed a single text into a vector.
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Embed multiple texts into vectors.
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// The dimensionality of the embedding vectors.
    fn dimensions(&self) -> usize;

    /// The embedding model identifier.
    fn model_id(&self) -> &str;
}

/// SQLite-based implementation of the MemoryStore trait.
///
/// Uses a hybrid search approach combining:
/// - BM25 full-text search via FTS5
/// - Vector similarity (cosine) via stored embeddings
/// - MMR (Maximal Marginal Relevance) for diversity
/// - Temporal decay for recency bias
#[derive(Clone)]
pub struct SqliteMemoryStore {
    pool: SqlitePool,
}

impl std::fmt::Debug for SqliteMemoryStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteMemoryStore")
            .field("pool", &"<SqlitePool>")
            .finish()
    }
}

impl SqliteMemoryStore {
    /// Create a new memory store with the given database pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Convert a MemoryEntryRow to a MemoryEntry domain type.
    fn row_to_entry(row: &MemoryEntryRow) -> Result<MemoryEntry> {
        let memory_type = match row.memory_type.as_str() {
            "episodic" => MemoryType::Episodic,
            "semantic" => MemoryType::Semantic,
            "procedural" => MemoryType::Procedural,
            _ => MemoryType::Semantic, // Default fallback
        };

        let source_type = row.source_type.as_deref().and_then(|s| match s {
            "document" => Some(SourceType::Document),
            "code" => Some(SourceType::Code),
            "config" => Some(SourceType::Config),
            "conversation" => Some(SourceType::Conversation),
            "runbook" => Some(SourceType::Runbook),
            "tool_schema" => Some(SourceType::ToolSchema),
            _ => None,
        });

        let session_id = row
            .session_id
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok());

        let last_accessed = row
            .last_accessed
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let expires_at = row
            .expires_at
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let created_at = DateTime::parse_from_rfc3339(&row.created_at)
            .map_err(|e| Error::Database(DatabaseError::Query(format!("Invalid date: {}", e))))?
            .with_timezone(&Utc);

        let metadata = row
            .metadata
            .as_deref()
            .map(|s| serde_json::from_str(s).unwrap_or_else(|e| {
                tracing::warn!(error = %e, raw = %s, "Failed to parse memory metadata, using empty object");
                serde_json::json!({})
            }))
            .unwrap_or_else(|| serde_json::json!({}));

        Ok(MemoryEntry {
            id: Uuid::parse_str(&row.id).map_err(|e| {
                Error::Database(DatabaseError::Query(format!("Invalid UUID: {}", e)))
            })?,
            memory_type,
            content: row.content.clone(),
            content_hash: row.content_hash.clone(),
            source: row.source.clone(),
            source_type,
            session_id,
            user_id: row.user_id.clone(),
            namespace: row
                .namespace
                .clone()
                .unwrap_or_else(|| "global".to_string()),
            importance: row.importance.map(|f| f as f32).unwrap_or(0.5),
            confidence: row.confidence.map(|f| f as f32).unwrap_or(1.0),
            access_count: row.access_count.map(|i| i as u32).unwrap_or(0),
            last_accessed,
            created_at,
            expires_at,
            metadata,
        })
    }

    /// Convert MemoryType to string for database storage.
    fn memory_type_to_string(memory_type: MemoryType) -> &'static str {
        match memory_type {
            MemoryType::Episodic => "episodic",
            MemoryType::Semantic => "semantic",
            MemoryType::Procedural => "procedural",
        }
    }

    /// Convert SourceType to string for database storage.
    fn source_type_to_string(source_type: &SourceType) -> &'static str {
        match source_type {
            SourceType::Document => "document",
            SourceType::Code => "code",
            SourceType::Config => "config",
            SourceType::Conversation => "conversation",
            SourceType::Runbook => "runbook",
            SourceType::ToolSchema => "tool_schema",
        }
    }

    /// Persist a memory archive summary and return the archive id.
    #[instrument(skip(self, source_memory_ids))]
    pub async fn store_archive_entry(
        &self,
        id: &str,
        summary: &str,
        source_memory_ids: &[String],
        namespace: Option<&str>,
        importance: Option<f32>,
        source_type: Option<&str>,
    ) -> Result<String> {
        let serialized_ids = serde_json::to_string(source_memory_ids).map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to serialize archive source ids: {}",
                e
            )))
        })?;

        sqlx::query(
            r#"
            INSERT INTO memory_archive (id, summary, source_memory_ids, source_type, namespace, importance)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                summary = excluded.summary,
                source_memory_ids = excluded.source_memory_ids,
                source_type = excluded.source_type,
                namespace = excluded.namespace,
                importance = excluded.importance
            "#,
        )
        .bind(id)
        .bind(summary)
        .bind(serialized_ids)
        .bind(source_type)
        .bind(namespace.unwrap_or("global"))
        .bind(importance.unwrap_or(0.5) as f64)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to store memory archive entry: {}",
                e
            )))
        })?;

        Ok(id.to_string())
    }

    /// Return episodic memories older than the supplied cutoff.
    #[instrument(skip(self))]
    pub async fn list_old_episodic_memories(
        &self,
        cutoff: DateTime<Utc>,
        namespace: Option<&str>,
        user_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>> {
        let mut sql = String::from(
            r#"
            SELECT * FROM memory_entries
            WHERE memory_type = 'episodic'
              AND created_at <= ?
              AND (expires_at IS NULL OR expires_at > ?)
            "#,
        );

        if namespace.is_some() {
            sql.push_str(" AND namespace = ?");
        }
        if user_id.is_some() {
            sql.push_str(" AND user_id = ?");
        }
        sql.push_str(" ORDER BY created_at ASC LIMIT ?");

        let now = Utc::now().to_rfc3339();
        let mut query = sqlx::query_as::<_, MemoryEntryRow>(&sql)
            .bind(cutoff.to_rfc3339())
            .bind(now);
        if let Some(namespace) = namespace {
            query = query.bind(namespace);
        }
        if let Some(user_id) = user_id {
            query = query.bind(user_id);
        }
        query = query.bind(limit as i64);

        let rows = query.fetch_all(&self.pool).await.map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to list old episodic memories: {}",
                e
            )))
        })?;

        rows.iter().map(Self::row_to_entry).collect()
    }

    /// Return recent memories for a namespace ordered newest-first.
    #[instrument(skip(self))]
    pub async fn list_recent(
        &self,
        namespace: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>> {
        let rows = if let Some(namespace) = namespace {
            sqlx::query_as::<_, MemoryEntryRow>(
                r#"
                SELECT * FROM memory_entries
                WHERE namespace = ?
                  AND (expires_at IS NULL OR expires_at > ?)
                ORDER BY created_at DESC
                LIMIT ?
                "#,
            )
            .bind(namespace)
            .bind(Utc::now().to_rfc3339())
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, MemoryEntryRow>(
                r#"
                SELECT * FROM memory_entries
                WHERE (expires_at IS NULL OR expires_at > ?)
                ORDER BY created_at DESC
                LIMIT ?
                "#,
            )
            .bind(Utc::now().to_rfc3339())
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to list recent memories: {}",
                e
            )))
        })?;

        rows.iter().map(Self::row_to_entry).collect()
    }

    /// List distinct namespaces for operator inspection.
    #[instrument(skip(self))]
    pub async fn list_namespaces(&self) -> Result<Vec<String>> {
        let rows = sqlx::query_scalar::<_, String>(
            r#"
            SELECT DISTINCT namespace
            FROM memory_entries
            WHERE namespace IS NOT NULL
            ORDER BY namespace ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to list memory namespaces: {}",
                e
            )))
        })?;

        Ok(rows)
    }

    /// Inspect stored archive summaries for one namespace.
    #[instrument(skip(self))]
    pub async fn list_archive_entries(
        &self,
        namespace: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryArchiveRow>> {
        let rows = if let Some(namespace) = namespace {
            sqlx::query_as::<_, MemoryArchiveRow>(
                r#"
                SELECT * FROM memory_archive
                WHERE namespace = ?
                ORDER BY created_at DESC
                LIMIT ?
                "#,
            )
            .bind(namespace)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, MemoryArchiveRow>(
                r#"
                SELECT * FROM memory_archive
                ORDER BY created_at DESC
                LIMIT ?
                "#,
            )
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to list memory archive entries: {}",
                e
            )))
        })?;

        Ok(rows)
    }

    /// List stored structured model artifacts.
    #[instrument(skip(self))]
    pub async fn list_model_artifacts(
        &self,
        namespace: Option<&str>,
        include_inactive: bool,
        limit: usize,
    ) -> Result<Vec<ModelArtifact>> {
        let mut sql = String::from(
            r#"
            SELECT * FROM memory_model_artifacts
            WHERE 1 = 1
            "#,
        );
        if namespace.is_some() {
            sql.push_str(" AND namespace = ?");
        }
        if !include_inactive {
            sql.push_str(" AND status = 'active'");
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT ?");

        let mut query = sqlx::query_as::<_, MemoryModelArtifactRow>(&sql);
        if let Some(namespace) = namespace {
            query = query.bind(namespace);
        }
        query = query.bind(limit.max(1) as i64);

        let rows = query.fetch_all(&self.pool).await.map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to list model artifacts: {}",
                e
            )))
        })?;

        rows.iter().map(Self::row_to_model_artifact).collect()
    }

    /// Fetch one structured model artifact by id.
    #[instrument(skip(self))]
    pub async fn get_model_artifact(&self, id: &str) -> Result<Option<ModelArtifact>> {
        let row = sqlx::query_as::<_, MemoryModelArtifactRow>(
            r#"
            SELECT * FROM memory_model_artifacts
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to fetch model artifact: {}",
                e
            )))
        })?;

        row.map(|row| Self::row_to_model_artifact(&row)).transpose()
    }

    /// Promote a new structured model artifact, superseding any active sibling.
    #[instrument(skip(self, request))]
    pub async fn promote_model_artifact(
        &self,
        request: &ModelArtifactPromotionRequest,
    ) -> Result<ModelArtifact> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let kind = Self::model_artifact_kind_to_string(request.kind);
        let source_lineage = serde_json::to_string(&request.source_lineage).map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to serialize model artifact lineage: {}",
                e
            )))
        })?;

        sqlx::query(
            r#"
            UPDATE memory_model_artifacts
            SET status = 'superseded',
                updated_at = ?,
                deactivated_at = ?
            WHERE namespace = ?
              AND kind = ?
              AND status = 'active'
            "#,
        )
        .bind(&now)
        .bind(&now)
        .bind(&request.namespace)
        .bind(kind)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to supersede active model artifacts: {}",
                e
            )))
        })?;

        sqlx::query(
            r#"
            INSERT INTO memory_model_artifacts (
                id, namespace, kind, summary, status, importance, confidence, source_lineage,
                promoted_by, correction_note, created_at, updated_at, deactivated_at
            )
            VALUES (?, ?, ?, ?, 'active', ?, ?, ?, ?, ?, ?, ?, NULL)
            "#,
        )
        .bind(&id)
        .bind(&request.namespace)
        .bind(kind)
        .bind(request.summary.trim())
        .bind(request.importance as f64)
        .bind(request.confidence as f64)
        .bind(source_lineage)
        .bind(request.promoted_by.as_deref())
        .bind(request.correction_note.as_deref())
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to insert model artifact: {}",
                e
            )))
        })?;

        self.get_model_artifact(&id).await?.ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "ModelArtifact".to_string(),
                id,
            })
        })
    }

    /// Update a structured model artifact in place.
    #[instrument(skip(self, request))]
    pub async fn update_model_artifact(
        &self,
        id: &str,
        request: &ModelArtifactUpdateRequest,
    ) -> Result<ModelArtifact> {
        let Some(existing) = self.get_model_artifact(id).await? else {
            return Err(Error::Database(DatabaseError::NotFound {
                entity: "ModelArtifact".to_string(),
                id: id.to_string(),
            }));
        };

        let now = Utc::now().to_rfc3339();
        let summary = request
            .summary
            .as_deref()
            .unwrap_or(existing.summary.as_str())
            .trim()
            .to_string();
        let status = request.status.unwrap_or(existing.status);

        if status.is_active() {
            sqlx::query(
                r#"
                UPDATE memory_model_artifacts
                SET status = 'superseded',
                    updated_at = ?,
                    deactivated_at = ?
                WHERE namespace = ?
                  AND kind = ?
                  AND status = 'active'
                  AND id != ?
                "#,
            )
            .bind(&now)
            .bind(&now)
            .bind(&existing.namespace)
            .bind(Self::model_artifact_kind_to_string(existing.kind))
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                Error::Database(DatabaseError::Query(format!(
                    "Failed to supersede sibling model artifacts: {}",
                    e
                )))
            })?;
        }

        let deactivated_at = if status.is_active() {
            None
        } else {
            Some(now.as_str())
        };

        sqlx::query(
            r#"
            UPDATE memory_model_artifacts
            SET summary = ?,
                status = ?,
                correction_note = ?,
                promoted_by = ?,
                updated_at = ?,
                deactivated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(summary)
        .bind(Self::model_artifact_status_to_string(status))
        .bind(
            request
                .correction_note
                .as_deref()
                .or(existing.correction_note.as_deref()),
        )
        .bind(
            request
                .updated_by
                .as_deref()
                .or(existing.promoted_by.as_deref()),
        )
        .bind(&now)
        .bind(deactivated_at)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to update model artifact: {}",
                e
            )))
        })?;

        self.get_model_artifact(id).await?.ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "ModelArtifact".to_string(),
                id: id.to_string(),
            })
        })
    }

    /// Delete a set of memories by id and return the number removed.
    #[instrument(skip(self, ids))]
    pub async fn delete_many(&self, ids: &[String]) -> Result<u64> {
        let mut removed = 0_u64;
        for id in ids {
            let result = sqlx::query(
                r#"
                DELETE FROM memory_entries
                WHERE id = ?
                "#,
            )
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                Error::Database(DatabaseError::Query(format!(
                    "Failed to delete archived memory entry {}: {}",
                    id, e
                )))
            })?;
            removed += result.rows_affected();
        }

        Ok(removed)
    }

    /// Serialize a vector of f32 to bytes for BLOB storage.
    fn vector_to_blob(vector: &[f32]) -> Vec<u8> {
        vector.iter().flat_map(|f| f.to_le_bytes()).collect()
    }

    /// Deserialize bytes to a vector of f32.
    fn blob_to_vector(blob: &[u8], dimensions: usize) -> Result<Vec<f32>> {
        if blob.len() != dimensions * 4 {
            return Err(Error::Memory(MemoryError::Embedding(
                "Vector blob size mismatch".to_string(),
            )));
        }

        let mut vector = Vec::with_capacity(dimensions);
        for i in 0..dimensions {
            let bytes: [u8; 4] = blob[i * 4..(i + 1) * 4]
                .try_into()
                .map_err(|_| Error::Memory(MemoryError::Embedding("Invalid blob".to_string())))?;
            vector.push(f32::from_le_bytes(bytes));
        }
        Ok(vector)
    }

    /// Calculate cosine similarity between two vectors.
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        (dot_product / (norm_a * norm_b)).clamp(-1.0, 1.0)
    }

    /// Calculate temporal decay score based on age of memory.
    fn temporal_decay(created_at: DateTime<Utc>, now: DateTime<Utc>) -> f32 {
        let age_days = (now - created_at).num_days() as f32;
        // Exponential decay with 30-day half-life
        (-age_days / 30.0).exp()
    }

    fn normalize_fts_distance(rank: f64) -> f32 {
        if !rank.is_finite() {
            return 0.0;
        }
        let distance = rank.abs() as f32;
        1.0 / (1.0 + distance)
    }

    fn row_to_model_artifact(row: &MemoryModelArtifactRow) -> Result<ModelArtifact> {
        let created_at = DateTime::parse_from_rfc3339(&row.created_at)
            .map_err(|e| {
                Error::Database(DatabaseError::Query(format!(
                    "Invalid model artifact created_at: {}",
                    e
                )))
            })?
            .with_timezone(&Utc);
        let updated_at = DateTime::parse_from_rfc3339(&row.updated_at)
            .map_err(|e| {
                Error::Database(DatabaseError::Query(format!(
                    "Invalid model artifact updated_at: {}",
                    e
                )))
            })?
            .with_timezone(&Utc);
        let deactivated_at = row
            .deactivated_at
            .as_deref()
            .map(DateTime::parse_from_rfc3339)
            .transpose()
            .map_err(|e| {
                Error::Database(DatabaseError::Query(format!(
                    "Invalid model artifact deactivated_at: {}",
                    e
                )))
            })?
            .map(|value| value.with_timezone(&Utc));
        let source_lineage = serde_json::from_str(&row.source_lineage).map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Invalid model artifact lineage payload: {}",
                e
            )))
        })?;

        Ok(ModelArtifact {
            id: row.id.clone(),
            namespace: row.namespace.clone(),
            kind: Self::parse_model_artifact_kind(&row.kind)?,
            summary: row.summary.clone(),
            status: Self::parse_model_artifact_status(&row.status)?,
            importance: row.importance as f32,
            confidence: row.confidence as f32,
            source_lineage,
            promoted_by: row.promoted_by.clone(),
            correction_note: row.correction_note.clone(),
            created_at,
            updated_at,
            deactivated_at,
        })
    }

    fn parse_model_artifact_kind(value: &str) -> Result<ModelArtifactKind> {
        match value {
            "user_model" => Ok(ModelArtifactKind::UserModel),
            "operator_model" => Ok(ModelArtifactKind::OperatorModel),
            "project_memory" => Ok(ModelArtifactKind::ProjectMemory),
            "archive_summary" => Ok(ModelArtifactKind::ArchiveSummary),
            other => Err(Error::Database(DatabaseError::Query(format!(
                "Unknown model artifact kind '{}'",
                other
            )))),
        }
    }

    fn parse_model_artifact_status(value: &str) -> Result<ModelArtifactStatus> {
        match value {
            "active" => Ok(ModelArtifactStatus::Active),
            "inactive" => Ok(ModelArtifactStatus::Inactive),
            "superseded" => Ok(ModelArtifactStatus::Superseded),
            "removed" => Ok(ModelArtifactStatus::Removed),
            other => Err(Error::Database(DatabaseError::Query(format!(
                "Unknown model artifact status '{}'",
                other
            )))),
        }
    }

    fn model_artifact_kind_to_string(kind: ModelArtifactKind) -> &'static str {
        match kind {
            ModelArtifactKind::UserModel => "user_model",
            ModelArtifactKind::OperatorModel => "operator_model",
            ModelArtifactKind::ProjectMemory => "project_memory",
            ModelArtifactKind::ArchiveSummary => "archive_summary",
        }
    }

    fn model_artifact_status_to_string(status: ModelArtifactStatus) -> &'static str {
        match status {
            ModelArtifactStatus::Active => "active",
            ModelArtifactStatus::Inactive => "inactive",
            ModelArtifactStatus::Superseded => "superseded",
            ModelArtifactStatus::Removed => "removed",
        }
    }

    fn lexical_scores(rows: &[MemorySearchRow]) -> HashMap<String, f32> {
        let mut base_scores = HashMap::new();
        let mut min_score = f32::INFINITY;
        let mut max_score = f32::NEG_INFINITY;

        for row in rows {
            let score = Self::normalize_fts_distance(row._rank);
            min_score = min_score.min(score);
            max_score = max_score.max(score);
            base_scores.insert(row.entry.id.clone(), score);
        }

        if base_scores.is_empty() {
            return base_scores;
        }

        if (max_score - min_score).abs() < f32::EPSILON {
            return base_scores
                .into_iter()
                .map(|(id, score)| (id, score.clamp(0.0, 1.0)))
                .collect();
        }

        base_scores
            .into_iter()
            .map(|(id, score)| {
                let relative = ((score - min_score) / (max_score - min_score)).clamp(0.0, 1.0);
                let normalized = ((relative * 0.8) + (score * 0.2)).clamp(0.0, 1.0);
                (id, normalized)
            })
            .collect()
    }

    fn fuse_score(
        lexical_score: f32,
        vector_score: Option<f32>,
        recency_score: f32,
        confidence_score: f32,
        importance_score: f32,
        recency_weight: f32,
    ) -> f32 {
        let mut weighted_sum = 0.0;
        let mut weight_total = 0.0;

        let lexical_weight = 0.35;
        weighted_sum += lexical_score * lexical_weight;
        weight_total += lexical_weight;

        let confidence_weight = 0.20;
        weighted_sum += confidence_score * confidence_weight;
        weight_total += confidence_weight;

        let importance_weight = 0.15;
        weighted_sum += importance_score * importance_weight;
        weight_total += importance_weight;

        let recency_weight = recency_weight.clamp(0.0, 1.0) * 0.15;
        if recency_weight > 0.0 {
            weighted_sum += recency_score * recency_weight;
            weight_total += recency_weight;
        }

        if let Some(vector_score) = vector_score {
            let vector_weight = 0.15;
            weighted_sum += vector_score * vector_weight;
            weight_total += vector_weight;
        }

        if weight_total == 0.0 {
            0.0
        } else {
            (weighted_sum / weight_total).clamp(0.0, 1.0)
        }
    }

    /// Build a permissive FTS query that matches any meaningful term.
    fn build_fts_query(text: &str) -> String {
        let terms: Vec<String> = text
            .split_whitespace()
            .map(|term| term.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|term| term.len() >= 2)
            .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
            .collect();

        if terms.is_empty() {
            "\"\"".to_string()
        } else {
            terms.join(" OR ")
        }
    }

    /// Fetch vectors for a batch of memory entries.
    async fn fetch_vectors(&self, memory_ids: &[String]) -> Result<HashMap<String, Vec<f32>>> {
        if memory_ids.is_empty() {
            return Ok(HashMap::new());
        }

        // Build parameterized query
        let placeholders: Vec<String> = memory_ids.iter().map(|_| "?".to_string()).collect();
        let query = format!(
            "SELECT memory_id, vector, dimensions FROM memory_vectors WHERE memory_id IN ({})",
            placeholders.join(", ")
        );

        let mut sql_query = sqlx::query(&query);
        for id in memory_ids {
            sql_query = sql_query.bind(id);
        }

        let rows = sql_query.fetch_all(&self.pool).await.map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to fetch vectors: {}",
                e
            )))
        })?;

        let mut vectors = HashMap::new();
        for row in rows {
            let memory_id: String = row.try_get("memory_id").map_err(|e| {
                Error::Database(DatabaseError::Query(format!(
                    "Failed to get memory_id: {}",
                    e
                )))
            })?;
            let blob: Vec<u8> = row.try_get("vector").map_err(|e| {
                Error::Database(DatabaseError::Query(format!(
                    "Failed to get vector blob: {}",
                    e
                )))
            })?;
            let dimensions: i64 = row.try_get("dimensions").map_err(|e| {
                Error::Database(DatabaseError::Query(format!(
                    "Failed to get dimensions: {}",
                    e
                )))
            })?;

            let vector = Self::blob_to_vector(&blob, dimensions as usize)?;
            vectors.insert(memory_id, vector);
        }

        Ok(vectors)
    }

    fn retrieval_artifact(entry: &MemoryEntry) -> RetrievalArtifactRef {
        RetrievalArtifactRef {
            artifact_id: entry.id.to_string(),
            artifact_kind: RetrievalArtifactKind::from_memory_parts(
                entry.memory_type,
                entry.source_type,
            ),
            namespace: entry.namespace.clone(),
            source_type: entry.source_type,
            source_label: entry.source.clone(),
        }
    }

    fn retrieval_explanation(
        entry: &MemoryEntry,
        now: DateTime<Utc>,
        factors: RetrievalScoreFactors,
        degraded_state: Option<RetrievalDegradedState>,
    ) -> RetrievalExplanation {
        let primary_artifact = Self::retrieval_artifact(entry);
        RetrievalExplanation {
            contributing_artifacts: vec![primary_artifact.clone()],
            primary_artifact,
            factors,
            freshness: Some(RetrievalFreshness {
                created_at: entry.created_at,
                last_accessed: entry.last_accessed,
                age_seconds: (now - entry.created_at).num_seconds().max(0),
            }),
            degraded_state,
        }
    }

    /// Store a vector embedding for a memory entry.
    pub async fn store_vector(
        &self,
        memory_id: &str,
        vector: Vec<f32>,
        model_id: &str,
    ) -> Result<()> {
        let dimensions = vector.len();
        let blob = Self::vector_to_blob(&vector);

        sqlx::query(
            r#"
            INSERT INTO memory_vectors (memory_id, vector, dimensions, model_id)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(memory_id) DO UPDATE SET
                vector = excluded.vector,
                dimensions = excluded.dimensions,
                model_id = excluded.model_id
            "#,
        )
        .bind(memory_id)
        .bind(&blob)
        .bind(dimensions as i64)
        .bind(model_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to store vector: {}",
                e
            )))
        })?;

        Ok(())
    }

    /// Search memories using a pre-computed query embedding.
    /// This allows the caller to use any embedding provider.
    pub async fn search_with_embedding(
        &self,
        query: &MemoryQuery,
        query_embedding: &[f32],
    ) -> Result<Vec<ScoredMemory>> {
        let now = Utc::now();

        // Build the FTS5 query
        let fts_query = Self::build_fts_query(&query.text);

        // Build filter conditions
        let mut filters = Vec::new();
        let mut params: Vec<String> = Vec::new();

        if !query.memory_types.is_empty() {
            let types: Vec<String> = query
                .memory_types
                .iter()
                .map(|t| format!("'{}'", Self::memory_type_to_string(*t)))
                .collect();
            filters.push(format!("e.memory_type IN ({})", types.join(", ")));
        }

        if !query.source_types.is_empty() {
            let types: Vec<String> = query
                .source_types
                .iter()
                .map(|t| format!("'{}'", Self::source_type_to_string(t)))
                .collect();
            filters.push(format!("e.source_type IN ({})", types.join(", ")));
        }

        if let Some(ref namespace) = query.namespace {
            filters.push("e.namespace = ?".to_string());
            params.push(namespace.clone());
        }

        if query.min_confidence > 0.0 {
            filters.push("e.confidence >= ?".to_string());
            params.push(query.min_confidence.to_string());
        }

        filters.push("(e.expires_at IS NULL OR e.expires_at > ?)".to_string());
        params.push(now.to_rfc3339());

        let filter_clause = if filters.is_empty() {
            String::new()
        } else {
            format!(" AND {}", filters.join(" AND "))
        };

        // Query using FTS5 for text search
        // Fetch more than limit to allow for reranking
        let fetch_limit = query.limit * 3;

        let sql = format!(
            r#"
            SELECT e.*, rank FROM memory_entries e
            JOIN memory_fts_mapping m ON e.id = m.memory_id
            JOIN memory_fts fts ON m.fts_rowid = fts.rowid
            WHERE memory_fts MATCH ? {}
            ORDER BY rank
            LIMIT ?
            "#,
            filter_clause
        );

        let mut sql_query = sqlx::query_as::<_, MemorySearchRow>(&sql);
        sql_query = sql_query.bind(&fts_query);

        for param in &params {
            sql_query = sql_query.bind(param);
        }

        sql_query = sql_query.bind(fetch_limit as i64);

        let rows: Vec<MemorySearchRow> = sql_query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::Database(DatabaseError::Query(format!("Search failed: {}", e))))?;
        let lexical_scores = Self::lexical_scores(&rows);

        // Convert to MemoryEntry and apply scoring
        let mut candidates: Vec<(MemoryEntry, RetrievalScoreFactors)> = Vec::new();

        for row in rows {
            let entry = Self::row_to_entry(&row.entry)?;
            let recency_score = Self::temporal_decay(entry.created_at, now);
            let confidence_score = entry.confidence.clamp(0.0, 1.0);
            let importance_score = entry.importance.clamp(0.0, 1.0);

            let lexical_score = lexical_scores
                .get(&entry.id.to_string())
                .copied()
                .unwrap_or_else(|| Self::normalize_fts_distance(row._rank));

            candidates.push((
                entry,
                RetrievalScoreFactors {
                    lexical_score,
                    vector_score: None,
                    recency_score,
                    confidence_score,
                    importance_score,
                    fused_score: Self::fuse_score(
                        lexical_score,
                        None,
                        recency_score,
                        confidence_score,
                        importance_score,
                        query.recency_weight,
                    ),
                    vector_lane: RetrievalVectorLane::RustRescored,
                },
            ));
        }

        // Do vector similarity scoring
        let memory_ids: Vec<String> = candidates.iter().map(|(e, _)| e.id.to_string()).collect();

        let vectors = self.fetch_vectors(&memory_ids).await?;

        // Re-score with vector similarity
        for (entry, factors) in &mut candidates {
            if let Some(vector) = vectors.get(&entry.id.to_string()) {
                let vector_sim = Self::cosine_similarity(query_embedding, vector);
                let vector_score = ((vector_sim + 1.0) / 2.0).clamp(0.0, 1.0);
                factors.vector_score = Some(vector_score);
                factors.fused_score = Self::fuse_score(
                    factors.lexical_score,
                    factors.vector_score,
                    factors.recency_score,
                    factors.confidence_score,
                    factors.importance_score,
                    query.recency_weight,
                );
            }
        }

        // Sort by score descending
        candidates.sort_by(|a, b| {
            b.1.fused_score
                .partial_cmp(&a.1.fused_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take top N
        candidates.truncate(query.limit);
        let results: Vec<ScoredMemory> = candidates
            .into_iter()
            .map(|(entry, factors)| {
                let score = factors.fused_score;
                let explanation = Self::retrieval_explanation(&entry, now, factors, None);
                ScoredMemory {
                    entry,
                    score,
                    explanation,
                }
            })
            .collect();

        // Update access counts for retrieved memories
        for scored in &results {
            let _ = sqlx::query(
                r#"
                UPDATE memory_entries
                SET access_count = access_count + 1,
                    last_accessed = datetime('now')
                WHERE id = ?
                "#,
            )
            .bind(scored.entry.id.to_string())
            .execute(&self.pool)
            .await;
        }

        Ok(results)
    }
}

#[async_trait]
impl MemoryStoreTrait for SqliteMemoryStore {
    #[instrument(skip(self, entry), fields(entry_id = %entry.id))]
    async fn store(&self, entry: MemoryEntry) -> Result<()> {
        let memory_type_str = Self::memory_type_to_string(entry.memory_type);
        let source_type_str = entry.source_type.as_ref().map(Self::source_type_to_string);
        let session_id_str = entry.session_id.map(|id| id.to_string());
        let last_accessed_str = entry.last_accessed.map(|dt| dt.to_rfc3339());
        let expires_at_str = entry.expires_at.map(|dt| dt.to_rfc3339());
        let created_at_str = entry.created_at.to_rfc3339();
        let metadata_str = serde_json::to_string(&entry.metadata).map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to serialize metadata: {}",
                e
            )))
        })?;

        // Insert into memory_entries
        sqlx::query(
            r#"
            INSERT INTO memory_entries (
                id, memory_type, content, content_hash, source, source_type,
                session_id, user_id, namespace, importance, confidence,
                access_count, last_accessed, created_at, expires_at, metadata
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                content = excluded.content,
                content_hash = excluded.content_hash,
                source = excluded.source,
                source_type = excluded.source_type,
                importance = excluded.importance,
                confidence = excluded.confidence,
                expires_at = excluded.expires_at,
                metadata = excluded.metadata
            "#,
        )
        .bind(entry.id.to_string())
        .bind(memory_type_str)
        .bind(&entry.content)
        .bind(&entry.content_hash)
        .bind(entry.source.as_ref())
        .bind(source_type_str)
        .bind(session_id_str.as_ref())
        .bind(entry.user_id.as_ref())
        .bind(&entry.namespace)
        .bind(entry.importance as f64)
        .bind(entry.confidence as f64)
        .bind(entry.access_count as i64)
        .bind(last_accessed_str.as_ref())
        .bind(&created_at_str)
        .bind(expires_at_str.as_ref())
        .bind(&metadata_str)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to store memory entry: {}",
                e
            )))
        })?;

        // Insert into FTS5 index
        // First delete any existing mapping (cascade will clean up FTS)
        sqlx::query("DELETE FROM memory_fts_mapping WHERE memory_id = ?")
            .bind(entry.id.to_string())
            .execute(&self.pool)
            .await
            .ok();

        // Insert into FTS5 (this creates the rowid automatically)
        let result = sqlx::query(
            r#"
            INSERT INTO memory_fts (content)
            VALUES (?)
            "#,
        )
        .bind(&entry.content)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to index memory entry: {}",
                e
            )))
        })?;

        // Store the mapping from FTS rowid to memory_id
        let fts_rowid = result.last_insert_rowid();
        sqlx::query(
            r#"
            INSERT INTO memory_fts_mapping (fts_rowid, memory_id)
            VALUES (?, ?)
            "#,
        )
        .bind(fts_rowid)
        .bind(entry.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to store FTS mapping: {}",
                e
            )))
        })?;

        debug!("Stored memory entry {}", entry.id);
        Ok(())
    }

    #[instrument(skip(self, query))]
    async fn search(&self, query: &MemoryQuery) -> Result<Vec<ScoredMemory>> {
        let now = Utc::now();

        // Build the FTS5 query
        let fts_query = Self::build_fts_query(&query.text);

        // Build filter conditions
        let mut filters = Vec::new();
        let mut params: Vec<String> = Vec::new();

        if !query.memory_types.is_empty() {
            let types: Vec<String> = query
                .memory_types
                .iter()
                .map(|t| format!("'{}'", Self::memory_type_to_string(*t)))
                .collect();
            filters.push(format!("e.memory_type IN ({})", types.join(", ")));
        }

        if !query.source_types.is_empty() {
            let types: Vec<String> = query
                .source_types
                .iter()
                .map(|t| format!("'{}'", Self::source_type_to_string(t)))
                .collect();
            filters.push(format!("e.source_type IN ({})", types.join(", ")));
        }

        if let Some(ref namespace) = query.namespace {
            filters.push("e.namespace = ?".to_string());
            params.push(namespace.clone());
        }

        if query.min_confidence > 0.0 {
            filters.push("e.confidence >= ?".to_string());
            params.push(query.min_confidence.to_string());
        }

        filters.push("(e.expires_at IS NULL OR e.expires_at > ?)".to_string());
        params.push(now.to_rfc3339());

        let filter_clause = if filters.is_empty() {
            String::new()
        } else {
            format!(" AND {}", filters.join(" AND "))
        };

        // Query using FTS5 for text search
        // Fetch more than limit to allow for reranking
        let fetch_limit = query.limit * 3;

        let sql = format!(
            r#"
            SELECT e.*, rank FROM memory_entries e
            JOIN memory_fts_mapping m ON e.id = m.memory_id
            JOIN memory_fts fts ON m.fts_rowid = fts.rowid
            WHERE memory_fts MATCH ? {}
            ORDER BY rank
            LIMIT ?
            "#,
            filter_clause
        );

        let mut sql_query = sqlx::query_as::<_, MemorySearchRow>(&sql);
        sql_query = sql_query.bind(&fts_query);

        for param in &params {
            sql_query = sql_query.bind(param);
        }

        sql_query = sql_query.bind(fetch_limit as i64);

        let rows: Vec<MemorySearchRow> = sql_query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::Database(DatabaseError::Query(format!("Search failed: {}", e))))?;
        let lexical_scores = Self::lexical_scores(&rows);

        // Convert to MemoryEntry and apply scoring
        let mut candidates: Vec<(MemoryEntry, RetrievalScoreFactors)> = Vec::new();

        for row in rows {
            let entry = Self::row_to_entry(&row.entry)?;
            let recency_score = Self::temporal_decay(entry.created_at, now);
            let confidence_score = entry.confidence.clamp(0.0, 1.0);
            let importance_score = entry.importance.clamp(0.0, 1.0);

            let lexical_score = lexical_scores
                .get(&entry.id.to_string())
                .copied()
                .unwrap_or_else(|| Self::normalize_fts_distance(row._rank));

            candidates.push((
                entry,
                RetrievalScoreFactors {
                    lexical_score,
                    vector_score: None,
                    recency_score,
                    confidence_score,
                    importance_score,
                    fused_score: Self::fuse_score(
                        lexical_score,
                        None,
                        recency_score,
                        confidence_score,
                        importance_score,
                        query.recency_weight,
                    ),
                    vector_lane: RetrievalVectorLane::Unavailable,
                },
            ));
        }

        // Sort by score descending
        candidates.sort_by(|a, b| {
            b.1.fused_score
                .partial_cmp(&a.1.fused_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take top N
        candidates.truncate(query.limit);
        let results: Vec<ScoredMemory> = candidates
            .into_iter()
            .map(|(entry, factors)| {
                let score = factors.fused_score;
                let explanation = Self::retrieval_explanation(
                    &entry,
                    now,
                    factors,
                    Some(RetrievalDegradedState {
                        code: RetrievalDegradedCode::VectorUnavailable,
                        message: "query embeddings are unavailable on this retrieval path; vector scoring was not applied".to_string(),
                    }),
                );
                ScoredMemory {
                    entry,
                    score,
                    explanation,
                }
            })
            .collect();

        // Update access counts for retrieved memories
        for scored in &results {
            let _ = sqlx::query(
                r#"
                UPDATE memory_entries
                SET access_count = access_count + 1,
                    last_accessed = datetime('now')
                WHERE id = ?
                "#,
            )
            .bind(scored.entry.id.to_string())
            .execute(&self.pool)
            .await;
        }

        Ok(results)
    }

    #[instrument(skip(self))]
    async fn get(&self, id: &str) -> Result<Option<MemoryEntry>> {
        let row: Option<MemoryEntryRow> = sqlx::query_as(
            r#"
            SELECT * FROM memory_entries
            WHERE id = ?
              AND (expires_at IS NULL OR expires_at > ?)
            "#,
        )
        .bind(id)
        .bind(Utc::now().to_rfc3339())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to get memory entry: {}",
                e
            )))
        })?;

        match row {
            Some(row) => {
                let entry = Self::row_to_entry(&row)?;

                // Update access count
                let _ = sqlx::query(
                    r#"
                    UPDATE memory_entries
                    SET access_count = access_count + 1,
                        last_accessed = datetime('now')
                    WHERE id = ?
                    "#,
                )
                .bind(id)
                .execute(&self.pool)
                .await;

                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn delete(&self, id: &str) -> Result<()> {
        // Delete from FTS5 mapping (cascade handles the FTS index)
        sqlx::query("DELETE FROM memory_fts_mapping WHERE memory_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                Error::Database(DatabaseError::Query(format!(
                    "Failed to delete from FTS index: {}",
                    e
                )))
            })?;

        // Delete from memory_entries (cascades to memory_vectors)
        let result = sqlx::query("DELETE FROM memory_entries WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                Error::Database(DatabaseError::Query(format!(
                    "Failed to delete memory entry: {}",
                    e
                )))
            })?;

        if result.rows_affected() == 0 {
            return Err(Error::Database(DatabaseError::NotFound {
                entity: "MemoryEntry".to_string(),
                id: id.to_string(),
            }));
        }

        debug!("Deleted memory entry {}", id);
        Ok(())
    }

    #[instrument(skip(self))]
    async fn dedupe_check(&self, content_hash: &str) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT id FROM memory_entries WHERE content_hash = ?
            "#,
        )
        .bind(content_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to check for duplicate: {}",
                e
            )))
        })?;

        Ok(row.map(|r| r.0))
    }

    #[instrument(skip(self))]
    async fn expire_stale(&self) -> Result<u64> {
        let now = Utc::now().to_rfc3339();

        // First, archive memories that are about to expire (optional - could be implemented)
        // For now, just delete expired entries

        let result = sqlx::query(
            r#"
            DELETE FROM memory_entries
            WHERE expires_at IS NOT NULL AND expires_at < ?
            "#,
        )
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to expire stale memories: {}",
                e
            )))
        })?;

        let count = result.rows_affected();
        debug!("Expired {} stale memories", count);
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((SqliteMemoryStore::cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((SqliteMemoryStore::cosine_similarity(&a, &b)).abs() < 0.001);
    }

    #[test]
    fn test_vector_blob_roundtrip() {
        let original = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        let blob = SqliteMemoryStore::vector_to_blob(&original);
        let recovered = SqliteMemoryStore::blob_to_vector(&blob, 5).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_temporal_decay() {
        let now = Utc::now();
        let recent = now - chrono::Duration::days(1);
        let old = now - chrono::Duration::days(30);
        let very_old = now - chrono::Duration::days(90);

        let recent_score = SqliteMemoryStore::temporal_decay(recent, now);
        let old_score = SqliteMemoryStore::temporal_decay(old, now);
        let very_old_score = SqliteMemoryStore::temporal_decay(very_old, now);

        assert!(recent_score > old_score);
        assert!(old_score > very_old_score);
    }

    // ── Integration tests with in-memory SQLite ──

    /// Create an in-memory SQLite database with the schema applied and return
    /// a SqliteMemoryStore backed by it.
    async fn setup_in_memory_store() -> SqliteMemoryStore {
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

        let options = SqliteConnectOptions::new()
            .filename(":memory:")
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("Failed to create in-memory SQLite pool");

        // Apply necessary schema
        let schema = r#"
            CREATE TABLE IF NOT EXISTS memory_entries (
                id TEXT PRIMARY KEY,
                memory_type TEXT NOT NULL,
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
            CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
                content,
                tokenize='porter unicode61'
            );
            CREATE TABLE IF NOT EXISTS memory_fts_mapping (
                fts_rowid INTEGER PRIMARY KEY,
                memory_id TEXT NOT NULL UNIQUE
            );
            CREATE TABLE IF NOT EXISTS memory_vectors (
                memory_id TEXT PRIMARY KEY,
                vector BLOB NOT NULL,
                dimensions INTEGER NOT NULL,
                model_id TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS memory_model_artifacts (
                id TEXT PRIMARY KEY,
                namespace TEXT NOT NULL DEFAULT 'global',
                kind TEXT NOT NULL,
                summary TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                importance REAL NOT NULL DEFAULT 0.8,
                confidence REAL NOT NULL DEFAULT 1.0,
                source_lineage TEXT NOT NULL DEFAULT '[]',
                promoted_by TEXT,
                correction_note TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                deactivated_at TEXT
            );
        "#;

        for statement in schema.split(';') {
            let trimmed = statement.trim();
            if trimmed.is_empty() {
                continue;
            }
            sqlx::query(trimmed)
                .execute(&pool)
                .await
                .expect("Schema creation failed");
        }

        SqliteMemoryStore::new(pool)
    }

    /// Helper to build a MemoryEntry for testing.
    fn make_entry(content: &str) -> MemoryEntry {
        MemoryEntry {
            id: Uuid::new_v4(),
            memory_type: MemoryType::Semantic,
            content: content.to_string(),
            content_hash: format!("{:x}", sha2::Sha256::digest(content.as_bytes())),
            source: Some("test".to_string()),
            source_type: Some(SourceType::Document),
            session_id: None,
            user_id: Some("test_user".to_string()),
            namespace: "global".to_string(),
            importance: 0.8,
            confidence: 1.0,
            access_count: 0,
            last_accessed: None,
            created_at: Utc::now(),
            expires_at: None,
            metadata: serde_json::json!({}),
        }
    }

    use sha2::Digest;

    #[tokio::test]
    async fn integration_store_and_get() {
        let store = setup_in_memory_store().await;
        let entry = make_entry("Rust is a systems programming language.");
        let entry_id = entry.id.to_string();

        // Store the entry
        store.store(entry.clone()).await.expect("store failed");

        // Retrieve it by ID
        let retrieved = store
            .get(&entry_id)
            .await
            .expect("get failed")
            .expect("entry should exist");

        assert_eq!(retrieved.id.to_string(), entry_id);
        assert_eq!(retrieved.content, "Rust is a systems programming language.");
        assert_eq!(retrieved.memory_type, MemoryType::Semantic);
        assert_eq!(retrieved.namespace, "global");
    }

    #[tokio::test]
    async fn integration_promote_model_artifact_supersedes_previous_active() {
        let store = setup_in_memory_store().await;

        let first = store
            .promote_model_artifact(&ModelArtifactPromotionRequest {
                namespace: "user-1".to_string(),
                kind: ModelArtifactKind::UserModel,
                summary: "User prefers concise technical explanations.".to_string(),
                importance: 0.9,
                confidence: 0.95,
                source_lineage: vec![],
                promoted_by: Some("test".to_string()),
                correction_note: None,
            })
            .await
            .expect("first promote failed");

        let second = store
            .promote_model_artifact(&ModelArtifactPromotionRequest {
                namespace: "user-1".to_string(),
                kind: ModelArtifactKind::UserModel,
                summary: "User prefers terse, high-signal answers.".to_string(),
                importance: 0.9,
                confidence: 0.95,
                source_lineage: vec![],
                promoted_by: Some("test".to_string()),
                correction_note: None,
            })
            .await
            .expect("second promote failed");

        let artifacts = store
            .list_model_artifacts(Some("user-1"), true, 10)
            .await
            .expect("list_model_artifacts failed");

        assert_eq!(artifacts.len(), 2);
        assert_eq!(second.status, ModelArtifactStatus::Active);
        assert!(artifacts
            .iter()
            .any(|artifact| artifact.id == first.id
                && artifact.status == ModelArtifactStatus::Superseded));
    }

    #[tokio::test]
    async fn integration_update_model_artifact_can_deactivate() {
        let store = setup_in_memory_store().await;
        let artifact = store
            .promote_model_artifact(&ModelArtifactPromotionRequest {
                namespace: "user-1".to_string(),
                kind: ModelArtifactKind::ProjectMemory,
                summary: "This repo is a Rust-first assistant runtime.".to_string(),
                importance: 0.8,
                confidence: 0.9,
                source_lineage: vec![],
                promoted_by: Some("test".to_string()),
                correction_note: None,
            })
            .await
            .expect("promote failed");

        let updated = store
            .update_model_artifact(
                &artifact.id,
                &ModelArtifactUpdateRequest {
                    status: Some(ModelArtifactStatus::Inactive),
                    ..Default::default()
                },
            )
            .await
            .expect("update_model_artifact failed");

        assert_eq!(updated.status, ModelArtifactStatus::Inactive);
        assert!(updated.deactivated_at.is_some());
    }

    #[tokio::test]
    async fn integration_get_nonexistent_returns_none() {
        let store = setup_in_memory_store().await;
        let result = store
            .get(&Uuid::new_v4().to_string())
            .await
            .expect("get failed");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn integration_store_and_delete() {
        let store = setup_in_memory_store().await;
        let entry = make_entry("Memory to be deleted");
        let entry_id = entry.id.to_string();

        store.store(entry).await.expect("store failed");

        // Verify it exists
        assert!(store.get(&entry_id).await.unwrap().is_some());

        // Delete it
        store.delete(&entry_id).await.expect("delete failed");

        // Verify it's gone
        assert!(store.get(&entry_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn integration_delete_nonexistent_returns_error() {
        let store = setup_in_memory_store().await;
        let result = store.delete(&Uuid::new_v4().to_string()).await;
        assert!(result.is_err(), "Deleting nonexistent entry should error");
    }

    #[tokio::test]
    async fn integration_fts5_search_by_keyword() {
        let store = setup_in_memory_store().await;

        let entry1 = make_entry("The quick brown fox jumps over the lazy dog");
        let entry2 = make_entry("Rust programming language offers memory safety");
        let entry3 = make_entry("The fox was cunning and clever");

        store.store(entry1).await.expect("store 1 failed");
        store.store(entry2).await.expect("store 2 failed");
        store.store(entry3).await.expect("store 3 failed");

        // Search for "fox" -- should match entries 1 and 3 but not 2
        let query = MemoryQuery {
            text: "fox".to_string(),
            limit: 10,
            ..Default::default()
        };
        let results = store.search(&query).await.expect("search failed");
        assert_eq!(results.len(), 2, "Should find 2 entries containing 'fox'");
        let contents: Vec<&str> = results.iter().map(|r| r.entry.content.as_str()).collect();
        assert!(
            contents
                .iter()
                .all(|c| c.contains("fox") || c.contains("Fox"))
        );

        // Search for "rust" -- should match only entry 2
        let query_rust = MemoryQuery {
            text: "rust".to_string(),
            limit: 10,
            ..Default::default()
        };
        let results_rust = store.search(&query_rust).await.expect("search failed");
        assert_eq!(results_rust.len(), 1);
        assert!(results_rust[0].entry.content.contains("Rust"));
    }

    #[tokio::test]
    async fn integration_fts5_search_no_matches() {
        let store = setup_in_memory_store().await;
        let entry = make_entry("The quick brown fox");
        store.store(entry).await.expect("store failed");

        let query = MemoryQuery {
            text: "elephant".to_string(),
            limit: 10,
            ..Default::default()
        };
        let results = store.search(&query).await.expect("search failed");
        assert!(results.is_empty(), "Should find no entries for 'elephant'");
    }

    #[tokio::test]
    async fn integration_vector_store_and_retrieve() {
        let store = setup_in_memory_store().await;
        let entry = make_entry("Vector test entry");
        let entry_id = entry.id.to_string();

        store.store(entry).await.expect("store failed");

        // Store a vector embedding
        let embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        store
            .store_vector(&entry_id, embedding.clone(), "test-model")
            .await
            .expect("store_vector failed");

        // Retrieve the vector via fetch_vectors
        let vectors = store
            .fetch_vectors(&[entry_id.clone()])
            .await
            .expect("fetch_vectors failed");

        assert!(vectors.contains_key(&entry_id));
        let retrieved = &vectors[&entry_id];
        assert_eq!(retrieved.len(), 5);
        for (a, b) in retrieved.iter().zip(embedding.iter()) {
            assert!((a - b).abs() < 1e-6, "Vector values should match");
        }
    }

    #[tokio::test]
    async fn integration_vector_upsert_overwrites() {
        let store = setup_in_memory_store().await;
        let entry = make_entry("Vector upsert test");
        let entry_id = entry.id.to_string();
        store.store(entry).await.expect("store failed");

        // Store initial vector
        store
            .store_vector(&entry_id, vec![1.0, 2.0, 3.0], "model-v1")
            .await
            .expect("store_vector failed");

        // Overwrite with new vector
        store
            .store_vector(&entry_id, vec![4.0, 5.0, 6.0], "model-v2")
            .await
            .expect("store_vector upsert failed");

        let vectors = store
            .fetch_vectors(&[entry_id.clone()])
            .await
            .expect("fetch_vectors failed");
        let v = &vectors[&entry_id];
        assert_eq!(v, &vec![4.0, 5.0, 6.0]);
    }

    #[tokio::test]
    async fn integration_access_count_increments_on_get() {
        let store = setup_in_memory_store().await;
        let entry = make_entry("Access count test");
        let entry_id = entry.id.to_string();

        store.store(entry).await.expect("store failed");

        // First get increments from 0 to 1
        let first = store.get(&entry_id).await.unwrap().unwrap();
        assert_eq!(first.access_count, 0, "First retrieval sees initial count");

        // Second get -- the UPDATE already ran during the first get, so this
        // should see access_count = 1
        let second = store.get(&entry_id).await.unwrap().unwrap();
        assert_eq!(
            second.access_count, 1,
            "Second retrieval should see count=1"
        );

        // Third get
        let third = store.get(&entry_id).await.unwrap().unwrap();
        assert_eq!(third.access_count, 2, "Third retrieval should see count=2");
    }

    #[tokio::test]
    async fn integration_dedupe_check() {
        let store = setup_in_memory_store().await;
        let entry = make_entry("Unique content for dedup check");
        let hash = entry.content_hash.clone();
        let entry_id = entry.id.to_string();

        store.store(entry).await.expect("store failed");

        // Should find the existing entry by content hash
        let result = store
            .dedupe_check(&hash)
            .await
            .expect("dedupe_check failed");
        assert_eq!(result, Some(entry_id));

        // Nonexistent hash
        let result_none = store
            .dedupe_check("nonexistent_hash")
            .await
            .expect("dedupe_check failed");
        assert!(result_none.is_none());
    }

    #[tokio::test]
    async fn integration_multiple_entries_management() {
        let store = setup_in_memory_store().await;

        // Store multiple entries
        let entries: Vec<MemoryEntry> = (0..5)
            .map(|i| make_entry(&format!("Entry number {} with unique content", i)))
            .collect();

        let ids: Vec<String> = entries.iter().map(|e| e.id.to_string()).collect();

        for entry in entries {
            store.store(entry).await.expect("store failed");
        }

        // All entries should be retrievable
        for id in &ids {
            let result = store.get(id).await.expect("get failed");
            assert!(result.is_some(), "Entry {} should exist", id);
        }

        // Delete the middle entry
        store.delete(&ids[2]).await.expect("delete failed");

        // Verify the middle entry is gone
        assert!(store.get(&ids[2]).await.unwrap().is_none());

        // Verify others still exist
        assert!(store.get(&ids[0]).await.unwrap().is_some());
        assert!(store.get(&ids[4]).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn integration_expire_stale_removes_expired_entries() {
        let store = setup_in_memory_store().await;

        // Create an entry that already expired
        let mut expired_entry = make_entry("This entry has expired");
        expired_entry.expires_at = Some(Utc::now() - chrono::Duration::hours(1));
        store
            .store(expired_entry.clone())
            .await
            .expect("store failed");

        // Create an entry that has not expired
        let mut fresh_entry = make_entry("This entry is still fresh");
        fresh_entry.expires_at = Some(Utc::now() + chrono::Duration::hours(24));
        store
            .store(fresh_entry.clone())
            .await
            .expect("store failed");

        // Create an entry with no expiration
        let permanent_entry = make_entry("This entry never expires");
        store
            .store(permanent_entry.clone())
            .await
            .expect("store failed");

        // Run expire_stale
        let expired_count = store.expire_stale().await.expect("expire_stale failed");
        assert_eq!(expired_count, 1, "Should expire exactly 1 entry");

        // Expired entry should be gone
        assert!(
            store
                .get(&expired_entry.id.to_string())
                .await
                .unwrap()
                .is_none()
        );

        // Fresh and permanent entries should remain
        assert!(
            store
                .get(&fresh_entry.id.to_string())
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            store
                .get(&permanent_entry.id.to_string())
                .await
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn integration_store_upsert_updates_content() {
        let store = setup_in_memory_store().await;
        let mut entry = make_entry("Original content");
        let entry_id = entry.id;

        store.store(entry.clone()).await.expect("store failed");

        // Update the content (same ID, different content)
        entry.content = "Updated content".to_string();
        entry.content_hash = format!("{:x}", sha2::Sha256::digest(b"Updated content"));
        store.store(entry).await.expect("upsert failed");

        let retrieved = store.get(&entry_id.to_string()).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Updated content");
    }

    #[tokio::test]
    async fn integration_search_respects_limit() {
        let store = setup_in_memory_store().await;

        // Store 5 entries all containing "database"
        for i in 0..5 {
            let entry = make_entry(&format!("Database concept number {}", i));
            store.store(entry).await.expect("store failed");
        }

        let query = MemoryQuery {
            text: "database".to_string(),
            limit: 2,
            ..Default::default()
        };
        let results = store.search(&query).await.expect("search failed");
        assert!(
            results.len() <= 2,
            "Should respect limit of 2, got {}",
            results.len()
        );
    }

    #[tokio::test]
    async fn integration_search_with_embedding_reports_rust_rescored_vector_lane() {
        let store = setup_in_memory_store().await;
        let mut entry = make_entry("Rust ownership model enables fearless refactoring");
        entry.source_type = Some(SourceType::Document);
        let entry_id = entry.id.to_string();

        store.store(entry).await.expect("store failed");
        store
            .store_vector(&entry_id, vec![1.0, 0.0, 0.0], "test-model")
            .await
            .expect("store_vector failed");

        let query = MemoryQuery {
            text: "ownership".to_string(),
            limit: 5,
            ..Default::default()
        };

        let results = store
            .search_with_embedding(&query, &[1.0, 0.0, 0.0])
            .await
            .expect("search_with_embedding failed");

        let explanation = &results[0].explanation;
        assert_eq!(
            explanation.factors.vector_lane,
            openrustclaw_core::types::RetrievalVectorLane::RustRescored
        );
        assert_eq!(
            explanation.primary_artifact.artifact_kind,
            openrustclaw_core::types::RetrievalArtifactKind::DocumentChunk
        );
        assert!(!explanation.contributing_artifacts.is_empty());
    }

    #[tokio::test]
    async fn integration_search_without_embedding_reports_unavailable_vector_lane() {
        let store = setup_in_memory_store().await;
        let entry = make_entry("Fallback lexical retrieval remains bounded");
        store.store(entry).await.expect("store failed");

        let query = MemoryQuery {
            text: "fallback".to_string(),
            limit: 5,
            ..Default::default()
        };

        let results = store.search(&query).await.expect("search failed");
        assert_eq!(
            results[0].explanation.factors.vector_lane,
            openrustclaw_core::types::RetrievalVectorLane::Unavailable
        );
        assert!(results[0].explanation.degraded_state.is_some());
    }

    #[tokio::test]
    async fn integration_search_reports_distinct_lexical_scores() {
        let store = setup_in_memory_store().await;

        let strong = make_entry("rust ownership borrowing rust ownership guarantees");
        let weak = make_entry("rust patterns");
        store.store(strong).await.expect("store strong failed");
        store.store(weak).await.expect("store weak failed");

        let query = MemoryQuery {
            text: "rust ownership".to_string(),
            limit: 5,
            ..Default::default()
        };

        let results = store.search(&query).await.expect("search failed");
        assert!(results.len() >= 2, "expected at least two matching results");
        assert!(
            results[0].explanation.factors.lexical_score
                > results[1].explanation.factors.lexical_score,
            "lexical factor should distinguish stronger textual matches"
        );
        assert_ne!(results[0].explanation.factors.lexical_score, 1.0);
        assert_ne!(results[1].explanation.factors.lexical_score, 1.0);
    }

    #[tokio::test]
    async fn integration_search_with_embedding_reports_complete_factor_payload() {
        let store = setup_in_memory_store().await;
        let mut entry = make_entry("Rust ownership notes from design docs");
        entry.source_type = Some(SourceType::Document);
        let entry_id = entry.id.to_string();
        store.store(entry).await.expect("store failed");
        store
            .store_vector(&entry_id, vec![0.8, 0.2, 0.0], "test-model")
            .await
            .expect("store_vector failed");

        let query = MemoryQuery {
            text: "ownership".to_string(),
            limit: 3,
            recency_weight: 0.4,
            ..Default::default()
        };

        let results = store
            .search_with_embedding(&query, &[1.0, 0.0, 0.0])
            .await
            .expect("search_with_embedding failed");
        let factors = &results[0].explanation.factors;
        assert!(factors.lexical_score >= 0.0 && factors.lexical_score <= 1.0);
        assert!(factors.vector_score.is_some());
        assert!(factors.recency_score >= 0.0 && factors.recency_score <= 1.0);
        assert!(factors.confidence_score >= 0.0 && factors.confidence_score <= 1.0);
        assert!(factors.importance_score >= 0.0 && factors.importance_score <= 1.0);
        assert!(results[0].explanation.freshness.is_some());
        assert_eq!(
            results[0].explanation.primary_artifact.artifact_kind,
            openrustclaw_core::types::RetrievalArtifactKind::DocumentChunk
        );
    }

    #[tokio::test]
    async fn integration_search_filters_low_confidence_and_updates_access_counts() {
        let store = setup_in_memory_store().await;
        let mut kept = make_entry("bounded retrieval stays trustworthy");
        kept.confidence = 0.9;
        let kept_id = kept.id.to_string();
        let mut filtered = make_entry("bounded retrieval discarded");
        filtered.confidence = 0.2;

        store.store(kept).await.expect("store kept failed");
        store.store(filtered).await.expect("store filtered failed");

        let query = MemoryQuery {
            text: "bounded retrieval".to_string(),
            limit: 5,
            min_confidence: 0.5,
            ..Default::default()
        };

        let results = store.search(&query).await.expect("search failed");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entry.id.to_string(), kept_id);

        let retrieved = store.get(&kept_id).await.expect("get failed").unwrap();
        assert_eq!(
            retrieved.access_count, 1,
            "search should increment access count before a follow-up read"
        );
    }
}

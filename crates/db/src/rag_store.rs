//! Durable RAG chunk storage backed by SQLite.

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use tracing::instrument;

use openrustclaw_core::error::{DatabaseError, Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RagChunkInput {
    pub chunk_id: String,
    pub source_id: String,
    pub chunk_index: i64,
    pub content: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RagChunkRecord {
    pub collection_name: String,
    pub chunk_id: String,
    pub source_id: String,
    pub chunk_index: i64,
    pub content: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct SqliteRagStore {
    pool: SqlitePool,
}

impl std::fmt::Debug for SqliteRagStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteRagStore")
            .field("pool", &"<SqlitePool>")
            .finish()
    }
}

impl SqliteRagStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    #[instrument(skip(self, chunks))]
    pub async fn replace_collection(
        &self,
        collection_name: &str,
        chunks: &[RagChunkInput],
    ) -> Result<usize> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to start rag transaction: {}",
                e
            )))
        })?;

        sqlx::query("DELETE FROM rag_chunks WHERE collection_name = ?")
            .bind(collection_name)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                Error::Database(DatabaseError::Query(format!(
                    "Failed to clear rag collection: {}",
                    e
                )))
            })?;

        for chunk in chunks {
            let metadata = serde_json::to_string(&chunk.metadata).map_err(|e| {
                Error::Database(DatabaseError::Query(format!(
                    "Failed to serialize rag chunk metadata: {}",
                    e
                )))
            })?;

            sqlx::query(
                r#"
                INSERT INTO rag_chunks (
                    collection_name,
                    chunk_id,
                    source_id,
                    chunk_index,
                    content,
                    metadata
                ) VALUES (?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(collection_name)
            .bind(&chunk.chunk_id)
            .bind(&chunk.source_id)
            .bind(chunk.chunk_index)
            .bind(&chunk.content)
            .bind(metadata)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                Error::Database(DatabaseError::Query(format!(
                    "Failed to store rag chunk: {}",
                    e
                )))
            })?;
        }

        tx.commit().await.map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to commit rag collection: {}",
                e
            )))
        })?;

        Ok(chunks.len())
    }

    #[instrument(skip(self))]
    pub async fn load_collection(
        &self,
        collection_name: &str,
        limit: Option<usize>,
    ) -> Result<Vec<RagChunkRecord>> {
        let effective_limit = limit.unwrap_or(1_000).max(1) as i64;
        let rows = sqlx::query(
            r#"
            SELECT collection_name, chunk_id, source_id, chunk_index, content, metadata, created_at
            FROM rag_chunks
            WHERE collection_name = ?
            ORDER BY chunk_index ASC, chunk_id ASC
            LIMIT ?
            "#,
        )
        .bind(collection_name)
        .bind(effective_limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to load rag collection: {}",
                e
            )))
        })?;

        rows.into_iter()
            .map(|row| {
                let metadata_raw: String = row.try_get("metadata").map_err(|e| {
                    Error::Database(DatabaseError::Query(format!(
                        "Failed to read rag metadata: {}",
                        e
                    )))
                })?;
                let created_at_raw: String = row.try_get("created_at").map_err(|e| {
                    Error::Database(DatabaseError::Query(format!(
                        "Failed to read rag created_at: {}",
                        e
                    )))
                })?;
                let metadata = serde_json::from_str(&metadata_raw).unwrap_or_else(|_| serde_json::json!({}));
                let created_at = if let Ok(parsed) = DateTime::parse_from_rfc3339(&created_at_raw)
                {
                    parsed.with_timezone(&Utc)
                } else {
                    let naive =
                        NaiveDateTime::parse_from_str(&created_at_raw, "%Y-%m-%d %H:%M:%S")
                            .map_err(|e| {
                                Error::Database(DatabaseError::Query(format!(
                                    "Invalid rag created_at value: {}",
                                    e
                                )))
                            })?;
                    DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
                };

                Ok(RagChunkRecord {
                    collection_name: row.try_get("collection_name").map_err(|e| {
                        Error::Database(DatabaseError::Query(format!(
                            "Failed to read rag collection_name: {}",
                            e
                        )))
                    })?,
                    chunk_id: row.try_get("chunk_id").map_err(|e| {
                        Error::Database(DatabaseError::Query(format!(
                            "Failed to read rag chunk_id: {}",
                            e
                        )))
                    })?,
                    source_id: row.try_get("source_id").map_err(|e| {
                        Error::Database(DatabaseError::Query(format!(
                            "Failed to read rag source_id: {}",
                            e
                        )))
                    })?,
                    chunk_index: row.try_get("chunk_index").map_err(|e| {
                        Error::Database(DatabaseError::Query(format!(
                            "Failed to read rag chunk_index: {}",
                            e
                        )))
                    })?,
                    content: row.try_get("content").map_err(|e| {
                        Error::Database(DatabaseError::Query(format!(
                            "Failed to read rag content: {}",
                            e
                        )))
                    })?,
                    metadata,
                    created_at,
                })
            })
            .collect()
    }

    #[instrument(skip(self))]
    pub async fn list_collections(&self, limit: Option<usize>) -> Result<Vec<(String, i64)>> {
        let effective_limit = limit.unwrap_or(100).max(1) as i64;
        let rows = sqlx::query(
            r#"
            SELECT collection_name, COUNT(*) AS chunk_count
            FROM rag_chunks
            GROUP BY collection_name
            ORDER BY collection_name ASC
            LIMIT ?
            "#,
        )
        .bind(effective_limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to list rag collections: {}",
                e
            )))
        })?;

        rows.into_iter()
            .map(|row| {
                let collection_name: String = row.try_get("collection_name").map_err(|e| {
                    Error::Database(DatabaseError::Query(format!(
                        "Failed to read rag collection_name: {}",
                        e
                    )))
                })?;
                let chunk_count: i64 = row.try_get("chunk_count").map_err(|e| {
                    Error::Database(DatabaseError::Query(format!(
                        "Failed to read rag chunk_count: {}",
                        e
                    )))
                })?;
                Ok((collection_name, chunk_count))
            })
            .collect()
    }

    #[instrument(skip(self))]
    pub async fn delete_collection(&self, collection_name: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM rag_chunks WHERE collection_name = ?")
            .bind(collection_name)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                Error::Database(DatabaseError::Query(format!(
                    "Failed to delete rag collection: {}",
                    e
                )))
            })?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{init_pool, run_migrations};
    use uuid::Uuid;

    #[tokio::test]
    async fn rag_store_replaces_and_loads_collection() {
        let db_path = std::env::temp_dir().join(format!("rag-store-{}.db", Uuid::new_v4()));
        let pool = init_pool(&format!("sqlite://{}", db_path.display()), 1)
            .await
            .unwrap();
        run_migrations(&pool).await.unwrap();
        let store = SqliteRagStore::new(pool);

        let stored = store
            .replace_collection(
                "docs",
                &[
                    RagChunkInput {
                        chunk_id: "chunk-a".to_string(),
                        source_id: "chunk-a".to_string(),
                        chunk_index: 0,
                        content: "Rust guarantees memory safety.".to_string(),
                        metadata: serde_json::json!({"type": "text"}),
                    },
                    RagChunkInput {
                        chunk_id: "chunk-b".to_string(),
                        source_id: "chunk-b".to_string(),
                        chunk_index: 1,
                        content: "Ownership and borrowing are core concepts.".to_string(),
                        metadata: serde_json::json!({"type": "text"}),
                    },
                ],
            )
            .await
            .unwrap();

        assert_eq!(stored, 2);

        let loaded = store.load_collection("docs", Some(10)).await.unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].chunk_id, "chunk-a");
        assert_eq!(loaded[1].chunk_id, "chunk-b");

        store
            .replace_collection(
                "docs",
                &[RagChunkInput {
                    chunk_id: "chunk-c".to_string(),
                    source_id: "chunk-c".to_string(),
                    chunk_index: 0,
                    content: "Replacement collection.".to_string(),
                    metadata: serde_json::json!({"type": "text"}),
                }],
            )
            .await
            .unwrap();

        let replaced = store.load_collection("docs", None).await.unwrap();
        assert_eq!(replaced.len(), 1);
        assert_eq!(replaced[0].chunk_id, "chunk-c");

        let collections = store.list_collections(None).await.unwrap();
        assert_eq!(collections, vec![("docs".to_string(), 1)]);

        let deleted = store.delete_collection("docs").await.unwrap();
        assert_eq!(deleted, 1);
        assert!(store.load_collection("docs", None).await.unwrap().is_empty());

        let _ = std::fs::remove_file(db_path);
    }
}

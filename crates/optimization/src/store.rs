use std::str::FromStr;

use chrono::{DateTime, Utc};
use openrustclaw_core::error::{DatabaseError, Error, Result};
use openrustclaw_db::SqlitePool;
use serde::de::DeserializeOwned;
use sqlx::Row;
use uuid::Uuid;

use crate::models::{
    CandidateChange, CandidateEvaluationRecord, CandidateStatus, ExecutionTier,
    OptimizationCandidate, OptimizationTarget, PromotionDecision, PromotionEvent, RiskClass,
    ShipStatus, TargetKind, TargetRegistration, TargetUpdate,
};

#[derive(Clone)]
pub struct OptimizationStore {
    pool: SqlitePool,
}

impl OptimizationStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn register_target(
        &self,
        registration: TargetRegistration,
    ) -> Result<OptimizationTarget> {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO optimization_targets (
                id, name, description, target_kind, execution_tier, risk_class, ship_status,
                workspace_root, mutation_policy, eval_suite, promotion_policy, metadata,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&registration.name)
        .bind(registration.description.clone())
        .bind(enum_str(&registration.target_kind))
        .bind(enum_str(&registration.execution_tier))
        .bind(enum_str(&registration.risk_class))
        .bind(enum_str(&registration.ship_status))
        .bind(&registration.workspace_root)
        .bind(to_json(&registration.mutation_policy)?)
        .bind(to_json(&registration.eval_suite)?)
        .bind(to_json(&registration.promotion_policy)?)
        .bind(registration.metadata.to_string())
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(db_query_error)?;

        self.get_target(&id).await
    }

    pub async fn update_target(
        &self,
        id: &str,
        update: TargetUpdate,
    ) -> Result<OptimizationTarget> {
        let existing = self.get_target(id).await?;
        let merged = TargetRegistration {
            name: existing.name,
            description: update.description.or(existing.description),
            target_kind: existing.target_kind,
            execution_tier: update.execution_tier.unwrap_or(existing.execution_tier),
            risk_class: update.risk_class.unwrap_or(existing.risk_class),
            ship_status: update.ship_status.unwrap_or(existing.ship_status),
            workspace_root: update.workspace_root.unwrap_or(existing.workspace_root),
            mutation_policy: update.mutation_policy.unwrap_or(existing.mutation_policy),
            eval_suite: update.eval_suite.unwrap_or(existing.eval_suite),
            promotion_policy: update.promotion_policy.unwrap_or(existing.promotion_policy),
            metadata: update.metadata.unwrap_or(existing.metadata),
        };

        sqlx::query(
            r#"
            UPDATE optimization_targets
            SET description = ?, execution_tier = ?, risk_class = ?, ship_status = ?,
                workspace_root = ?, mutation_policy = ?, eval_suite = ?, promotion_policy = ?,
                metadata = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(merged.description.clone())
        .bind(enum_str(&merged.execution_tier))
        .bind(enum_str(&merged.risk_class))
        .bind(enum_str(&merged.ship_status))
        .bind(&merged.workspace_root)
        .bind(to_json(&merged.mutation_policy)?)
        .bind(to_json(&merged.eval_suite)?)
        .bind(to_json(&merged.promotion_policy)?)
        .bind(merged.metadata.to_string())
        .bind(Utc::now().to_rfc3339())
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(db_query_error)?;

        self.get_target(id).await
    }

    pub async fn list_targets(&self) -> Result<Vec<OptimizationTarget>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, target_kind, execution_tier, risk_class,
                   ship_status, workspace_root, mutation_policy, eval_suite,
                   promotion_policy, metadata, created_at, updated_at
            FROM optimization_targets
            ORDER BY name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_query_error)?;

        rows.into_iter().map(target_from_row).collect()
    }

    pub async fn get_target(&self, id_or_name: &str) -> Result<OptimizationTarget> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, target_kind, execution_tier, risk_class,
                   ship_status, workspace_root, mutation_policy, eval_suite,
                   promotion_policy, metadata, created_at, updated_at
            FROM optimization_targets
            WHERE id = ? OR name = ?
            LIMIT 1
            "#,
        )
        .bind(id_or_name)
        .bind(id_or_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_query_error)?
        .ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "optimization_target".to_string(),
                id: id_or_name.to_string(),
            })
        })?;

        target_from_row(row)
    }

    pub async fn submit_candidate(
        &self,
        target_id: &str,
        hypothesis: &str,
        proposed_by: &str,
        changes: Vec<CandidateChange>,
        trace_id: Option<String>,
    ) -> Result<OptimizationCandidate> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO optimization_candidates (
                id, target_id, hypothesis, proposed_by, change_set, status, artifact_manifest,
                trace_id, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(target_id)
        .bind(hypothesis)
        .bind(proposed_by)
        .bind(to_json(&changes)?)
        .bind(enum_str(&CandidateStatus::Draft))
        .bind(serde_json::json!({}).to_string())
        .bind(trace_id)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(db_query_error)?;

        self.get_candidate(&id).await
    }

    pub async fn list_candidates(
        &self,
        target_id: Option<&str>,
        status: Option<CandidateStatus>,
        limit: Option<usize>,
    ) -> Result<Vec<OptimizationCandidate>> {
        let mut sql = String::from(
            r#"
            SELECT id, target_id, hypothesis, proposed_by, change_set, status, diff_summary,
                   result_summary, artifact_manifest, trace_id, created_at, updated_at
            FROM optimization_candidates
            WHERE 1 = 1
            "#,
        );
        if target_id.is_some() {
            sql.push_str(" AND target_id = ?");
        }
        if status.is_some() {
            sql.push_str(" AND status = ?");
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT ?");
        let mut query = sqlx::query(&sql);
        if let Some(target_id) = target_id {
            query = query.bind(target_id);
        }
        if let Some(status) = status {
            query = query.bind(enum_str(&status));
        }
        query = query.bind(limit.unwrap_or(50) as i64);

        let rows = query.fetch_all(&self.pool).await.map_err(db_query_error)?;
        rows.into_iter().map(candidate_from_row).collect()
    }

    pub async fn get_candidate(&self, id: &str) -> Result<OptimizationCandidate> {
        let row = sqlx::query(
            r#"
            SELECT id, target_id, hypothesis, proposed_by, change_set, status, diff_summary,
                   result_summary, artifact_manifest, trace_id, created_at, updated_at
            FROM optimization_candidates
            WHERE id = ?
            LIMIT 1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_query_error)?
        .ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "optimization_candidate".to_string(),
                id: id.to_string(),
            })
        })?;

        candidate_from_row(row)
    }

    pub async fn set_candidate_status(
        &self,
        candidate_id: &str,
        status: CandidateStatus,
        diff_summary: Option<serde_json::Value>,
        result_summary: Option<serde_json::Value>,
        artifact_manifest: Option<serde_json::Value>,
        trace_id: Option<String>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE optimization_candidates
            SET status = ?, diff_summary = COALESCE(?, diff_summary),
                result_summary = COALESCE(?, result_summary),
                artifact_manifest = COALESCE(?, artifact_manifest),
                trace_id = COALESCE(?, trace_id), updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(enum_str(&status))
        .bind(diff_summary.map(|value| value.to_string()))
        .bind(result_summary.map(|value| value.to_string()))
        .bind(artifact_manifest.map(|value| value.to_string()))
        .bind(trace_id)
        .bind(Utc::now().to_rfc3339())
        .bind(candidate_id)
        .execute(&self.pool)
        .await
        .map_err(db_query_error)?;
        Ok(())
    }

    pub async fn record_evaluation(
        &self,
        candidate_id: &str,
        eval_name: &str,
        status: &str,
        exit_code: Option<i64>,
        duration_ms: i64,
        stdout: &str,
        stderr: &str,
        metrics: serde_json::Value,
        trace_id: Option<String>,
    ) -> Result<CandidateEvaluationRecord> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO optimization_evaluations (
                id, candidate_id, eval_name, status, exit_code, duration_ms,
                stdout, stderr, metrics, trace_id, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(candidate_id)
        .bind(eval_name)
        .bind(status)
        .bind(exit_code)
        .bind(duration_ms)
        .bind(stdout)
        .bind(stderr)
        .bind(metrics.to_string())
        .bind(trace_id)
        .bind(created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(db_query_error)?;

        self.list_evaluations(candidate_id)
            .await?
            .into_iter()
            .find(|record| record.id == id)
            .ok_or_else(|| Error::Internal("stored evaluation missing".to_string()))
    }

    pub async fn list_evaluations(
        &self,
        candidate_id: &str,
    ) -> Result<Vec<CandidateEvaluationRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT id, candidate_id, eval_name, status, exit_code, duration_ms,
                   stdout, stderr, metrics, trace_id, created_at
            FROM optimization_evaluations
            WHERE candidate_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(candidate_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_query_error)?;

        rows.into_iter().map(evaluation_from_row).collect()
    }

    pub async fn record_promotion(
        &self,
        candidate_id: &str,
        decision: PromotionDecision,
        decided_by: &str,
        notes: Option<&str>,
        rollback_reference: Option<&str>,
        trace_id: Option<String>,
    ) -> Result<PromotionEvent> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO optimization_promotions (
                id, candidate_id, decision, decided_by, notes, rollback_reference, trace_id, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(candidate_id)
        .bind(enum_str(&decision))
        .bind(decided_by)
        .bind(notes)
        .bind(rollback_reference)
        .bind(trace_id.clone())
        .bind(created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(db_query_error)?;

        let mapped_status = match decision {
            PromotionDecision::Reject => Some(CandidateStatus::Rejected),
            PromotionDecision::Approve => Some(CandidateStatus::Approved),
            PromotionDecision::PromoteExperimental => Some(CandidateStatus::PromotedExperimental),
            PromotionDecision::PromoteCompat => Some(CandidateStatus::PromotedCompat),
            PromotionDecision::QueueRustMerge => Some(CandidateStatus::QueuedRustMerge),
            PromotionDecision::KeepCandidate => None,
        };
        if let Some(status) = mapped_status {
            self.set_candidate_status(candidate_id, status, None, None, None, trace_id.clone())
                .await?;
        }

        self.list_promotions(candidate_id)
            .await?
            .into_iter()
            .find(|record| record.id == id)
            .ok_or_else(|| Error::Internal("stored promotion missing".to_string()))
    }

    pub async fn list_promotions(&self, candidate_id: &str) -> Result<Vec<PromotionEvent>> {
        let rows = sqlx::query(
            r#"
            SELECT id, candidate_id, decision, decided_by, notes, rollback_reference, trace_id, created_at
            FROM optimization_promotions
            WHERE candidate_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(candidate_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_query_error)?;

        rows.into_iter().map(promotion_from_row).collect()
    }
}

fn db_query_error(error: sqlx::Error) -> Error {
    Error::Database(DatabaseError::Query(error.to_string()))
}

fn parse_time(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|error| Error::Internal(format!("invalid stored timestamp '{value}': {error}")))
}

fn parse_json<T: DeserializeOwned>(value: &str, column: &str) -> Result<T> {
    serde_json::from_str(value)
        .map_err(|error| Error::Internal(format!("invalid JSON in column {column}: {error}")))
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value)
        .map_err(|error| Error::Internal(format!("failed to serialize JSON: {error}")))
}

fn target_from_row(row: sqlx::sqlite::SqliteRow) -> Result<OptimizationTarget> {
    Ok(OptimizationTarget {
        id: row.get("id"),
        name: row.get("name"),
        description: row.try_get("description").ok(),
        target_kind: parse_enum(&row.get::<String, _>("target_kind"))?,
        execution_tier: parse_enum(&row.get::<String, _>("execution_tier"))?,
        risk_class: parse_enum(&row.get::<String, _>("risk_class"))?,
        ship_status: parse_enum(&row.get::<String, _>("ship_status"))?,
        workspace_root: row.get("workspace_root"),
        mutation_policy: parse_json(&row.get::<String, _>("mutation_policy"), "mutation_policy")?,
        eval_suite: parse_json(&row.get::<String, _>("eval_suite"), "eval_suite")?,
        promotion_policy: parse_json(
            &row.get::<String, _>("promotion_policy"),
            "promotion_policy",
        )?,
        metadata: parse_json(&row.get::<String, _>("metadata"), "metadata")?,
        created_at: parse_time(&row.get::<String, _>("created_at"))?,
        updated_at: parse_time(&row.get::<String, _>("updated_at"))?,
    })
}

fn candidate_from_row(row: sqlx::sqlite::SqliteRow) -> Result<OptimizationCandidate> {
    let artifact_manifest = row
        .try_get::<String, _>("artifact_manifest")
        .ok()
        .unwrap_or_else(|| "{}".to_string());
    Ok(OptimizationCandidate {
        id: row.get("id"),
        target_id: row.get("target_id"),
        hypothesis: row.get("hypothesis"),
        proposed_by: row.get("proposed_by"),
        changes: parse_json(&row.get::<String, _>("change_set"), "change_set")?,
        status: parse_enum(&row.get::<String, _>("status"))?,
        diff_summary: row
            .try_get::<Option<String>, _>("diff_summary")
            .ok()
            .flatten()
            .map(|raw| parse_json(&raw, "diff_summary"))
            .transpose()?,
        result_summary: row
            .try_get::<Option<String>, _>("result_summary")
            .ok()
            .flatten()
            .map(|raw| parse_json(&raw, "result_summary"))
            .transpose()?,
        artifact_manifest: parse_json(&artifact_manifest, "artifact_manifest")?,
        trace_id: row.try_get("trace_id").ok(),
        created_at: parse_time(&row.get::<String, _>("created_at"))?,
        updated_at: parse_time(&row.get::<String, _>("updated_at"))?,
    })
}

fn evaluation_from_row(row: sqlx::sqlite::SqliteRow) -> Result<CandidateEvaluationRecord> {
    Ok(CandidateEvaluationRecord {
        id: row.get("id"),
        candidate_id: row.get("candidate_id"),
        eval_name: row.get("eval_name"),
        status: row.get("status"),
        exit_code: row.try_get("exit_code").ok(),
        duration_ms: row.get("duration_ms"),
        stdout: row.get("stdout"),
        stderr: row.get("stderr"),
        metrics: parse_json(&row.get::<String, _>("metrics"), "metrics")?,
        trace_id: row.try_get("trace_id").ok(),
        created_at: parse_time(&row.get::<String, _>("created_at"))?,
    })
}

fn promotion_from_row(row: sqlx::sqlite::SqliteRow) -> Result<PromotionEvent> {
    Ok(PromotionEvent {
        id: row.get("id"),
        candidate_id: row.get("candidate_id"),
        decision: parse_enum(&row.get::<String, _>("decision"))?,
        decided_by: row.get("decided_by"),
        notes: row.try_get("notes").ok(),
        rollback_reference: row.try_get("rollback_reference").ok(),
        trace_id: row.try_get("trace_id").ok(),
        created_at: parse_time(&row.get::<String, _>("created_at"))?,
    })
}

fn parse_enum<T>(raw: &str) -> Result<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    raw.parse::<T>()
        .map_err(|error| Error::Internal(format!("invalid enum value '{raw}': {error}")))
}

fn enum_str<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "\"invalid\"".to_string())
        .trim_matches('"')
        .to_string()
}

macro_rules! impl_from_str_via_serde {
    ($ty:ty) => {
        impl FromStr for $ty {
            type Err = serde_json::Error;

            fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
                serde_json::from_value(serde_json::Value::String(value.to_string()))
            }
        }
    };
}

impl_from_str_via_serde!(TargetKind);
impl_from_str_via_serde!(ExecutionTier);
impl_from_str_via_serde!(RiskClass);
impl_from_str_via_serde!(ShipStatus);
impl_from_str_via_serde!(CandidateStatus);
impl_from_str_via_serde!(PromotionDecision);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EvaluationSpec, MutationPolicy, PromotionPolicy};
    use openrustclaw_db::{init_pool, run_migrations};

    #[tokio::test]
    async fn register_and_fetch_target() {
        let pool = init_pool("sqlite::memory:", 1).await.unwrap();
        run_migrations(&pool).await.unwrap();
        let store = OptimizationStore::new(pool);

        let target = store
            .register_target(TargetRegistration {
                name: "skills.instructions".to_string(),
                description: Some("Optimize skill instructions".to_string()),
                target_kind: TargetKind::Skill,
                execution_tier: ExecutionTier::RustNative,
                risk_class: RiskClass::SafeConfig,
                ship_status: ShipStatus::Experimental,
                workspace_root: ".".to_string(),
                mutation_policy: MutationPolicy::default(),
                eval_suite: vec![EvaluationSpec {
                    name: "echo".to_string(),
                    command: crate::models::EvaluationCommand {
                        program: "echo".to_string(),
                        args: vec!["ok".to_string()],
                    },
                    working_directory: None,
                    timeout_secs: Some(10),
                    success_metric: None,
                    metadata: serde_json::json!({}),
                }],
                promotion_policy: PromotionPolicy::default(),
                metadata: serde_json::json!({"lane":"skills"}),
            })
            .await
            .unwrap();

        let loaded = store.get_target(&target.id).await.unwrap();
        assert_eq!(loaded.name, "skills.instructions");
        assert_eq!(loaded.metadata["lane"], "skills");
    }

    #[tokio::test]
    async fn submit_candidate_and_record_promotion() {
        let pool = init_pool("sqlite::memory:", 1).await.unwrap();
        run_migrations(&pool).await.unwrap();
        let store = OptimizationStore::new(pool.clone());
        let target = store
            .register_target(TargetRegistration {
                name: "rag.controls".to_string(),
                description: None,
                target_kind: TargetKind::RagPolicy,
                execution_tier: ExecutionTier::RustNative,
                risk_class: RiskClass::SafeConfig,
                ship_status: ShipStatus::Experimental,
                workspace_root: ".".to_string(),
                mutation_policy: MutationPolicy::default(),
                eval_suite: vec![],
                promotion_policy: PromotionPolicy::default(),
                metadata: serde_json::json!({}),
            })
            .await
            .unwrap();

        let candidate = store
            .submit_candidate(
                &target.id,
                "Try a smaller top_k",
                "tester",
                vec![CandidateChange {
                    path: "config/rag.json".to_string(),
                    new_content: "{\"top_k\":3}".to_string(),
                    summary: None,
                    field_path: Some("top_k".to_string()),
                    metadata: serde_json::json!({}),
                }],
                None,
            )
            .await
            .unwrap();
        assert!(matches!(candidate.status, CandidateStatus::Draft));

        let event = store
            .record_promotion(
                &candidate.id,
                PromotionDecision::Approve,
                "operator",
                Some("good baseline"),
                None,
                None,
            )
            .await
            .unwrap();
        assert!(matches!(event.decision, PromotionDecision::Approve));

        let updated = store.get_candidate(&candidate.id).await.unwrap();
        assert!(matches!(updated.status, CandidateStatus::Approved));
    }
}

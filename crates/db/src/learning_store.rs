use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use openrustclaw_core::error::{DatabaseError, Error, Result};
use openrustclaw_core::types::{
    LearningCandidate, LearningCandidateCreateRequest, LearningCandidateEvidence,
    LearningCandidateEvidenceInput, LearningCandidateEvidenceKind, LearningCandidateImpact,
    LearningCandidateReviewAction, LearningCandidateReviewRequest, LearningCandidateSourceKind,
    LearningCandidateSourceRef, LearningCandidateStatus,
};

use crate::models::{LearningCandidateEvidenceRow, LearningCandidateRow};

#[derive(Clone)]
pub struct SqliteLearningStore {
    pool: SqlitePool,
}

impl std::fmt::Debug for SqliteLearningStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteLearningStore")
            .field("pool", &"<SqlitePool>")
            .finish()
    }
}

impl SqliteLearningStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_candidates(
        &self,
        namespace: Option<&str>,
        status: Option<LearningCandidateStatus>,
        limit: usize,
    ) -> Result<Vec<LearningCandidate>> {
        let mut sql = String::from(
            r#"
            SELECT * FROM learning_candidates
            WHERE 1 = 1
            "#,
        );
        if namespace.is_some() {
            sql.push_str(" AND namespace = ?");
        }
        if status.is_some() {
            sql.push_str(" AND status = ?");
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT ?");

        let mut query = sqlx::query_as::<_, LearningCandidateRow>(&sql);
        if let Some(namespace) = namespace {
            query = query.bind(namespace);
        }
        if let Some(status) = status {
            query = query.bind(Self::status_to_string(status));
        }
        query = query.bind(limit.max(1) as i64);

        let rows = query.fetch_all(&self.pool).await.map_err(|error| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to list learning candidates: {error}"
            )))
        })?;

        let mut candidates = Vec::with_capacity(rows.len());
        for row in rows {
            let evidence = self.list_candidate_evidence(&row.id).await?;
            candidates.push(Self::row_to_candidate(&row, evidence)?);
        }
        Ok(candidates)
    }

    pub async fn get_candidate(&self, id: &str) -> Result<Option<LearningCandidate>> {
        let row = sqlx::query_as::<_, LearningCandidateRow>(
            r#"
            SELECT * FROM learning_candidates
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to fetch learning candidate: {error}"
            )))
        })?;

        match row {
            Some(row) => {
                let evidence = self.list_candidate_evidence(&row.id).await?;
                Ok(Some(Self::row_to_candidate(&row, evidence)?))
            }
            None => Ok(None),
        }
    }

    pub async fn create_candidate(
        &self,
        request: &LearningCandidateCreateRequest,
    ) -> Result<LearningCandidate> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO learning_candidates (
                id, namespace, kind, signal, recommendation, rationale, confidence, impact, status,
                source_kind, source_id, source_detail, review_note, reviewed_by, god_mode_origin,
                task_id, category, claw_id, model_profile_id, provider, autonomy_level, execution_mode,
                promoted_lesson_id, created_at, updated_at, reviewed_at, quarantined_at, quarantined_by,
                quarantine_reason, rolled_back_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'pending_review', ?, ?, ?, NULL, NULL, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?, NULL, NULL, NULL, NULL, NULL)
            "#,
        )
        .bind(&id)
        .bind(&request.namespace)
        .bind(request.kind.trim())
        .bind(request.signal.trim())
        .bind(request.recommendation.trim())
        .bind(request.rationale.as_deref())
        .bind(request.confidence.clamp(0.0, 1.0) as f64)
        .bind(Self::impact_to_string(request.impact))
        .bind(Self::source_kind_to_string(request.source.kind))
        .bind(&request.source.source_id)
        .bind(request.source.detail.as_deref())
        .bind(i64::from(
            request.god_mode_origin || request.autonomy_level.as_deref() == Some("yolo"),
        ))
        .bind(request.task_id.as_deref())
        .bind(request.category.as_deref())
        .bind(request.claw_id.as_deref())
        .bind(request.model_profile_id.as_deref())
        .bind(request.provider.as_deref())
        .bind(request.autonomy_level.as_deref())
        .bind(request.execution_mode.as_deref())
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to insert learning candidate: {error}"
            )))
        })?;

        for evidence in &request.evidence {
            self.insert_evidence(&id, evidence).await?;
        }
        self.insert_history(&id, "queued", None, None, None).await?;

        self.get_candidate(&id).await?.ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "LearningCandidate".to_string(),
                id,
            })
        })
    }

    pub async fn review_candidate(
        &self,
        id: &str,
        request: &LearningCandidateReviewRequest,
    ) -> Result<LearningCandidate> {
        let current = self.get_candidate(id).await?.ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "LearningCandidate".to_string(),
                id: id.to_string(),
            })
        })?;
        let now = Utc::now().to_rfc3339();
        let status = match request.action {
            LearningCandidateReviewAction::Approve => LearningCandidateStatus::Approved,
            LearningCandidateReviewAction::Reject => LearningCandidateStatus::Rejected,
            LearningCandidateReviewAction::Supersede => LearningCandidateStatus::Superseded,
        };
        let review_note = request
            .review_note
            .as_deref()
            .or(current.review_note.as_deref());
        let reviewed_by = request
            .reviewed_by
            .as_deref()
            .or(current.reviewed_by.as_deref());

        sqlx::query(
            r#"
            UPDATE learning_candidates
            SET status = ?,
                review_note = ?,
                reviewed_by = ?,
                reviewed_at = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(Self::status_to_string(status))
        .bind(review_note)
        .bind(reviewed_by)
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to update learning candidate review: {error}"
            )))
        })?;

        self.insert_history(
            id,
            match request.action {
                LearningCandidateReviewAction::Approve => "approved",
                LearningCandidateReviewAction::Reject => "rejected",
                LearningCandidateReviewAction::Supersede => "superseded",
            },
            None,
            reviewed_by,
            review_note,
        )
        .await?;

        self.get_candidate(id).await?.ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "LearningCandidate".to_string(),
                id: id.to_string(),
            })
        })
    }

    pub async fn mark_promoted(
        &self,
        id: &str,
        lesson_id: &str,
        actor: Option<&str>,
        note: Option<&str>,
    ) -> Result<LearningCandidate> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE learning_candidates
            SET status = 'promoted',
                promoted_lesson_id = ?,
                reviewed_by = COALESCE(?, reviewed_by),
                reviewed_at = COALESCE(reviewed_at, ?),
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(lesson_id)
        .bind(actor)
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to mark learning candidate promoted: {error}"
            )))
        })?;

        self.insert_history(id, "promoted", Some(lesson_id), actor, note)
            .await?;

        self.get_candidate(id).await?.ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "LearningCandidate".to_string(),
                id: id.to_string(),
            })
        })
    }

    pub async fn mark_rolled_back(
        &self,
        id: &str,
        actor: Option<&str>,
        reason: Option<&str>,
    ) -> Result<LearningCandidate> {
        let current = self.get_candidate(id).await?.ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "LearningCandidate".to_string(),
                id: id.to_string(),
            })
        })?;
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE learning_candidates
            SET status = 'rolled_back',
                review_note = COALESCE(?, review_note),
                reviewed_by = COALESCE(?, reviewed_by),
                updated_at = ?,
                rolled_back_at = ?
            WHERE id = ?
            "#,
        )
        .bind(reason)
        .bind(actor)
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to roll back learning candidate: {error}"
            )))
        })?;

        self.insert_history(
            id,
            "rolled_back",
            current.promoted_lesson_id.as_deref(),
            actor,
            reason,
        )
        .await?;

        self.get_candidate(id).await?.ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "LearningCandidate".to_string(),
                id: id.to_string(),
            })
        })
    }

    pub async fn mark_quarantined(
        &self,
        id: &str,
        actor: Option<&str>,
        reason: Option<&str>,
    ) -> Result<LearningCandidate> {
        let current = self.get_candidate(id).await?.ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "LearningCandidate".to_string(),
                id: id.to_string(),
            })
        })?;
        let now = Utc::now().to_rfc3339();
        let status = if current.promoted_lesson_id.is_some() {
            "rolled_back"
        } else {
            Self::status_to_string(current.status)
        };
        sqlx::query(
            r#"
            UPDATE learning_candidates
            SET status = ?,
                review_note = COALESCE(?, review_note),
                reviewed_by = COALESCE(?, reviewed_by),
                updated_at = ?,
                quarantined_at = ?,
                quarantined_by = ?,
                quarantine_reason = ?,
                rolled_back_at = CASE
                    WHEN promoted_lesson_id IS NOT NULL THEN COALESCE(rolled_back_at, ?)
                    ELSE rolled_back_at
                END
            WHERE id = ?
            "#,
        )
        .bind(status)
        .bind(reason)
        .bind(actor)
        .bind(&now)
        .bind(&now)
        .bind(actor)
        .bind(reason)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to quarantine learning candidate: {error}"
            )))
        })?;

        self.insert_history(
            id,
            "quarantined",
            current.promoted_lesson_id.as_deref(),
            actor,
            reason,
        )
        .await?;

        self.get_candidate(id).await?.ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "LearningCandidate".to_string(),
                id: id.to_string(),
            })
        })
    }

    async fn list_candidate_evidence(
        &self,
        candidate_id: &str,
    ) -> Result<Vec<LearningCandidateEvidence>> {
        let rows = sqlx::query_as::<_, LearningCandidateEvidenceRow>(
            r#"
            SELECT * FROM learning_candidate_evidence
            WHERE candidate_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(candidate_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to list learning candidate evidence: {error}"
            )))
        })?;

        rows.iter().map(Self::row_to_evidence).collect()
    }

    async fn insert_evidence(
        &self,
        candidate_id: &str,
        evidence: &LearningCandidateEvidenceInput,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO learning_candidate_evidence (
                id, candidate_id, kind, summary, source_id, recorded_by, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(candidate_id)
        .bind(Self::evidence_kind_to_string(evidence.kind))
        .bind(evidence.summary.trim())
        .bind(evidence.source_id.as_deref())
        .bind(evidence.recorded_by.as_deref())
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to insert learning candidate evidence: {error}"
            )))
        })?;
        Ok(())
    }

    async fn insert_history(
        &self,
        candidate_id: &str,
        action: &str,
        lesson_id: Option<&str>,
        actor: Option<&str>,
        note: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO learning_candidate_history (
                id, candidate_id, action, lesson_id, actor, note, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(candidate_id)
        .bind(action)
        .bind(lesson_id)
        .bind(actor)
        .bind(note)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to insert learning candidate history: {error}"
            )))
        })?;
        Ok(())
    }

    fn row_to_candidate(
        row: &LearningCandidateRow,
        evidence: Vec<LearningCandidateEvidence>,
    ) -> Result<LearningCandidate> {
        Ok(LearningCandidate {
            id: row.id.clone(),
            namespace: row.namespace.clone(),
            kind: row.kind.clone(),
            signal: row.signal.clone(),
            recommendation: row.recommendation.clone(),
            rationale: row.rationale.clone(),
            confidence: row.confidence as f32,
            impact: Self::impact_from_str(&row.impact)?,
            status: Self::status_from_str(&row.status)?,
            source: LearningCandidateSourceRef {
                kind: Self::source_kind_from_str(&row.source_kind)?,
                source_id: row.source_id.clone(),
                detail: row.source_detail.clone(),
            },
            evidence,
            review_note: row.review_note.clone(),
            reviewed_by: row.reviewed_by.clone(),
            god_mode_origin: row.god_mode_origin.unwrap_or(0) == 1,
            task_id: row.task_id.clone(),
            category: row.category.clone(),
            claw_id: row.claw_id.clone(),
            model_profile_id: row.model_profile_id.clone(),
            provider: row.provider.clone(),
            autonomy_level: row.autonomy_level.clone(),
            execution_mode: row.execution_mode.clone(),
            promoted_lesson_id: row.promoted_lesson_id.clone(),
            created_at: parse_timestamp(&row.created_at)?,
            updated_at: parse_timestamp(&row.updated_at)?,
            reviewed_at: row
                .reviewed_at
                .as_deref()
                .map(parse_timestamp)
                .transpose()?,
            quarantined_at: row
                .quarantined_at
                .as_deref()
                .map(parse_timestamp)
                .transpose()?,
            quarantined_by: row.quarantined_by.clone(),
            quarantine_reason: row.quarantine_reason.clone(),
            rolled_back_at: row
                .rolled_back_at
                .as_deref()
                .map(parse_timestamp)
                .transpose()?,
        })
    }

    fn row_to_evidence(row: &LearningCandidateEvidenceRow) -> Result<LearningCandidateEvidence> {
        Ok(LearningCandidateEvidence {
            id: row.id.clone(),
            kind: Self::evidence_kind_from_str(&row.kind)?,
            summary: row.summary.clone(),
            source_id: row.source_id.clone(),
            recorded_by: row.recorded_by.clone(),
            created_at: parse_timestamp(&row.created_at)?,
        })
    }

    fn impact_to_string(value: LearningCandidateImpact) -> &'static str {
        match value {
            LearningCandidateImpact::Standard => "standard",
            LearningCandidateImpact::High => "high",
        }
    }

    fn status_to_string(value: LearningCandidateStatus) -> &'static str {
        match value {
            LearningCandidateStatus::PendingReview => "pending_review",
            LearningCandidateStatus::Approved => "approved",
            LearningCandidateStatus::Rejected => "rejected",
            LearningCandidateStatus::Superseded => "superseded",
            LearningCandidateStatus::Promoted => "promoted",
            LearningCandidateStatus::RolledBack => "rolled_back",
        }
    }

    fn source_kind_to_string(value: LearningCandidateSourceKind) -> &'static str {
        match value {
            LearningCandidateSourceKind::ReflectionCandidate => "reflection_candidate",
            LearningCandidateSourceKind::AuditRecord => "audit_record",
            LearningCandidateSourceKind::RuntimeEvent => "runtime_event",
            LearningCandidateSourceKind::ModelArtifact => "model_artifact",
            LearningCandidateSourceKind::Manual => "manual",
        }
    }

    fn evidence_kind_to_string(value: LearningCandidateEvidenceKind) -> &'static str {
        match value {
            LearningCandidateEvidenceKind::Replay => "replay",
            LearningCandidateEvidenceKind::Evaluation => "evaluation",
            LearningCandidateEvidenceKind::Audit => "audit",
            LearningCandidateEvidenceKind::RuntimeEvent => "runtime_event",
            LearningCandidateEvidenceKind::OperatorReview => "operator_review",
        }
    }

    fn impact_from_str(value: &str) -> Result<LearningCandidateImpact> {
        match value {
            "standard" => Ok(LearningCandidateImpact::Standard),
            "high" => Ok(LearningCandidateImpact::High),
            other => Err(Error::Database(DatabaseError::Query(format!(
                "Unknown learning candidate impact '{other}'"
            )))),
        }
    }

    fn status_from_str(value: &str) -> Result<LearningCandidateStatus> {
        match value {
            "pending_review" => Ok(LearningCandidateStatus::PendingReview),
            "approved" => Ok(LearningCandidateStatus::Approved),
            "rejected" => Ok(LearningCandidateStatus::Rejected),
            "superseded" => Ok(LearningCandidateStatus::Superseded),
            "promoted" => Ok(LearningCandidateStatus::Promoted),
            "rolled_back" => Ok(LearningCandidateStatus::RolledBack),
            other => Err(Error::Database(DatabaseError::Query(format!(
                "Unknown learning candidate status '{other}'"
            )))),
        }
    }

    fn source_kind_from_str(value: &str) -> Result<LearningCandidateSourceKind> {
        match value {
            "reflection_candidate" => Ok(LearningCandidateSourceKind::ReflectionCandidate),
            "audit_record" => Ok(LearningCandidateSourceKind::AuditRecord),
            "runtime_event" => Ok(LearningCandidateSourceKind::RuntimeEvent),
            "model_artifact" => Ok(LearningCandidateSourceKind::ModelArtifact),
            "manual" => Ok(LearningCandidateSourceKind::Manual),
            other => Err(Error::Database(DatabaseError::Query(format!(
                "Unknown learning candidate source kind '{other}'"
            )))),
        }
    }

    fn evidence_kind_from_str(value: &str) -> Result<LearningCandidateEvidenceKind> {
        match value {
            "replay" => Ok(LearningCandidateEvidenceKind::Replay),
            "evaluation" => Ok(LearningCandidateEvidenceKind::Evaluation),
            "audit" => Ok(LearningCandidateEvidenceKind::Audit),
            "runtime_event" => Ok(LearningCandidateEvidenceKind::RuntimeEvent),
            "operator_review" => Ok(LearningCandidateEvidenceKind::OperatorReview),
            other => Err(Error::Database(DatabaseError::Query(format!(
                "Unknown learning candidate evidence kind '{other}'"
            )))),
        }
    }
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|parsed| parsed.with_timezone(&Utc))
        .map_err(|error| {
            Error::Database(DatabaseError::Query(format!(
                "Invalid RFC3339 timestamp '{value}': {error}"
            )))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::LearningCandidateHistoryRow;
    use crate::{init_pool, run_migrations};

    fn candidate_request() -> LearningCandidateCreateRequest {
        LearningCandidateCreateRequest {
            namespace: "user-1".to_string(),
            kind: "worker_status".to_string(),
            signal: "worker returned failed".to_string(),
            recommendation: "route a safer model".to_string(),
            rationale: Some("failure observed".to_string()),
            confidence: 0.82,
            impact: LearningCandidateImpact::High,
            source: LearningCandidateSourceRef {
                kind: LearningCandidateSourceKind::ReflectionCandidate,
                source_id: "receipt-1:0".to_string(),
                detail: Some("run-1".to_string()),
            },
            evidence: vec![LearningCandidateEvidenceInput {
                kind: LearningCandidateEvidenceKind::Replay,
                summary: "Replay reproduced the failure safely".to_string(),
                source_id: Some("eval-1".to_string()),
                recorded_by: Some("test".to_string()),
            }],
            god_mode_origin: false,
            task_id: Some("task-1".to_string()),
            category: Some("code".to_string()),
            claw_id: Some("main".to_string()),
            model_profile_id: Some("core-groq".to_string()),
            provider: Some("groq".to_string()),
            autonomy_level: Some("managed".to_string()),
            execution_mode: Some("orchestrated".to_string()),
        }
    }

    #[tokio::test]
    async fn learning_store_round_trips_candidates_and_evidence() {
        let pool = init_pool("sqlite::memory:", 1).await.unwrap();
        run_migrations(&pool).await.unwrap();
        let store = SqliteLearningStore::new(pool.clone());

        let candidate = store.create_candidate(&candidate_request()).await.unwrap();
        assert_eq!(candidate.status, LearningCandidateStatus::PendingReview);
        assert_eq!(candidate.evidence.len(), 1);

        let listed = store
            .list_candidates(
                Some("user-1"),
                Some(LearningCandidateStatus::PendingReview),
                10,
            )
            .await
            .unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, candidate.id);
    }

    #[tokio::test]
    async fn learning_store_reviews_promotes_and_rolls_back_candidates() {
        let pool = init_pool("sqlite::memory:", 1).await.unwrap();
        run_migrations(&pool).await.unwrap();
        let store = SqliteLearningStore::new(pool.clone());

        let candidate = store.create_candidate(&candidate_request()).await.unwrap();
        let approved = store
            .review_candidate(
                &candidate.id,
                &LearningCandidateReviewRequest {
                    action: LearningCandidateReviewAction::Approve,
                    reviewed_by: Some("operator".to_string()),
                    review_note: Some("safe to promote".to_string()),
                },
            )
            .await
            .unwrap();
        assert_eq!(approved.status, LearningCandidateStatus::Approved);

        let promoted = store
            .mark_promoted(
                &candidate.id,
                "lesson-1",
                Some("operator"),
                Some("promoted"),
            )
            .await
            .unwrap();
        assert_eq!(promoted.status, LearningCandidateStatus::Promoted);
        assert_eq!(promoted.promoted_lesson_id.as_deref(), Some("lesson-1"));

        let rolled_back = store
            .mark_rolled_back(&candidate.id, Some("operator"), Some("rollback"))
            .await
            .unwrap();
        assert_eq!(rolled_back.status, LearningCandidateStatus::RolledBack);

        let history_rows = sqlx::query_as::<_, LearningCandidateHistoryRow>(
            "SELECT * FROM learning_candidate_history WHERE candidate_id = ? ORDER BY created_at ASC",
        )
        .bind(&candidate.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert!(history_rows.iter().any(|row| row.action == "approved"));
        assert!(history_rows.iter().any(|row| row.action == "promoted"));
        assert!(history_rows.iter().any(|row| row.action == "rolled_back"));
    }
}

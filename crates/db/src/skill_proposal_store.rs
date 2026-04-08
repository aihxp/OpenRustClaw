use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use openrustclaw_core::error::{DatabaseError, Error, Result};
use openrustclaw_core::types::{
    SkillProposal, SkillProposalReviewAction, SkillProposalReviewRequest, SkillProposalSourceKind,
    SkillProposalSourceRef, SkillProposalStatus, SkillProposalVerificationReport,
    SkillProposalVerificationStatus,
};
use uuid::Uuid;

use crate::models::SkillProposalRow as DbSkillProposalRow;

#[derive(Clone)]
pub struct SqliteSkillProposalStore {
    pool: SqlitePool,
}

impl std::fmt::Debug for SqliteSkillProposalStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteSkillProposalStore")
            .field("pool", &"<SqlitePool>")
            .finish()
    }
}

impl SqliteSkillProposalStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_proposals(
        &self,
        namespace: Option<&str>,
        status: Option<SkillProposalStatus>,
        limit: usize,
    ) -> Result<Vec<SkillProposal>> {
        let mut sql = String::from(
            r#"
            SELECT * FROM skill_proposals
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

        let mut query = sqlx::query_as::<_, DbSkillProposalRow>(&sql);
        if let Some(namespace) = namespace {
            query = query.bind(namespace);
        }
        if let Some(status) = status {
            query = query.bind(Self::status_to_string(status));
        }
        query = query.bind(limit.max(1) as i64);

        let rows = query.fetch_all(&self.pool).await.map_err(|error| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to list skill proposals: {error}"
            )))
        })?;

        rows.iter().map(Self::row_to_proposal).collect()
    }

    pub async fn get_proposal(&self, id: &str) -> Result<Option<SkillProposal>> {
        let row = sqlx::query_as::<_, DbSkillProposalRow>(
            r#"
            SELECT * FROM skill_proposals
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to fetch skill proposal: {error}"
            )))
        })?;

        row.as_ref().map(Self::row_to_proposal).transpose()
    }

    pub async fn create_proposal(&self, proposal: &SkillProposal) -> Result<SkillProposal> {
        sqlx::query(
            r#"
            INSERT INTO skill_proposals (
                id, namespace, skill_name, summary, body, rationale, status, verification_status,
                source_kind, source_id, source_detail, artifact_path, review_note, reviewed_by,
                verification_summary, verification_compiled_skill_name, verification_artifact_path,
                verification_blocked, verified_by, installed_skill_name, created_at, updated_at,
                reviewed_at, verified_at, installed_at, rolled_back_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&proposal.id)
        .bind(&proposal.namespace)
        .bind(&proposal.skill_name)
        .bind(&proposal.summary)
        .bind(&proposal.body)
        .bind(proposal.rationale.as_deref())
        .bind(Self::status_to_string(proposal.status))
        .bind(Self::verification_status_to_string(
            proposal.verification_status,
        ))
        .bind(Self::source_kind_to_string(proposal.source.kind))
        .bind(&proposal.source.source_id)
        .bind(proposal.source.detail.as_deref())
        .bind(&proposal.artifact_path)
        .bind(proposal.review_note.as_deref())
        .bind(proposal.reviewed_by.as_deref())
        .bind(
            proposal
                .verification_report
                .as_ref()
                .map(|report| report.summary.as_str()),
        )
        .bind(
            proposal
                .verification_report
                .as_ref()
                .and_then(|report| report.compiled_skill_name.as_deref()),
        )
        .bind(
            proposal
                .verification_report
                .as_ref()
                .and_then(|report| report.verification_artifact_path.as_deref()),
        )
        .bind(
            proposal
                .verification_report
                .as_ref()
                .map(|report| i64::from(report.blocked))
                .unwrap_or(0),
        )
        .bind(proposal.verified_by.as_deref())
        .bind(proposal.installed_skill_name.as_deref())
        .bind(proposal.created_at.to_rfc3339())
        .bind(proposal.updated_at.to_rfc3339())
        .bind(proposal.reviewed_at.map(|value| value.to_rfc3339()))
        .bind(proposal.verified_at.map(|value| value.to_rfc3339()))
        .bind(proposal.installed_at.map(|value| value.to_rfc3339()))
        .bind(proposal.rolled_back_at.map(|value| value.to_rfc3339()))
        .execute(&self.pool)
        .await
        .map_err(|error| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to insert skill proposal: {error}"
            )))
        })?;

        self.insert_history(
            &proposal.id,
            "queued",
            None,
            None,
            Some(Self::verification_status_to_string(
                proposal.verification_status,
            )),
            None,
        )
        .await?;

        self.get_proposal(&proposal.id).await?.ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "SkillProposal".to_string(),
                id: proposal.id.clone(),
            })
        })
    }

    pub async fn review_proposal(
        &self,
        id: &str,
        request: &SkillProposalReviewRequest,
    ) -> Result<SkillProposal> {
        let current = self.get_proposal(id).await?.ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "SkillProposal".to_string(),
                id: id.to_string(),
            })
        })?;
        let now = Utc::now().to_rfc3339();
        let status = match request.action {
            SkillProposalReviewAction::Approve => SkillProposalStatus::Approved,
            SkillProposalReviewAction::Reject => SkillProposalStatus::Rejected,
            SkillProposalReviewAction::Supersede => SkillProposalStatus::Superseded,
        };
        let note = request
            .review_note
            .as_deref()
            .or(current.review_note.as_deref());
        let reviewed_by = request
            .reviewed_by
            .as_deref()
            .or(current.reviewed_by.as_deref());

        sqlx::query(
            r#"
            UPDATE skill_proposals
            SET status = ?,
                review_note = ?,
                reviewed_by = ?,
                reviewed_at = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(Self::status_to_string(status))
        .bind(note)
        .bind(reviewed_by)
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to update skill proposal review: {error}"
            )))
        })?;

        self.insert_history(
            id,
            match request.action {
                SkillProposalReviewAction::Approve => "approved",
                SkillProposalReviewAction::Reject => "rejected",
                SkillProposalReviewAction::Supersede => "superseded",
            },
            reviewed_by,
            note,
            Some(Self::verification_status_to_string(
                current.verification_status,
            )),
            current.installed_skill_name.as_deref(),
        )
        .await?;

        self.get_proposal(id).await?.ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "SkillProposal".to_string(),
                id: id.to_string(),
            })
        })
    }

    pub async fn record_verification(
        &self,
        id: &str,
        report: &SkillProposalVerificationReport,
        actor: Option<&str>,
        note: Option<&str>,
    ) -> Result<SkillProposal> {
        let current = self.get_proposal(id).await?.ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "SkillProposal".to_string(),
                id: id.to_string(),
            })
        })?;
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE skill_proposals
            SET verification_status = ?,
                verification_summary = ?,
                verification_compiled_skill_name = ?,
                verification_artifact_path = ?,
                verification_blocked = ?,
                verified_by = ?,
                verified_at = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(Self::verification_status_to_string(report.status))
        .bind(&report.summary)
        .bind(report.compiled_skill_name.as_deref())
        .bind(report.verification_artifact_path.as_deref())
        .bind(i64::from(report.blocked))
        .bind(actor)
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to update skill proposal verification: {error}"
            )))
        })?;

        self.insert_history(
            id,
            "verified",
            actor,
            note.or(Some(report.summary.as_str())),
            Some(Self::verification_status_to_string(report.status)),
            current.installed_skill_name.as_deref(),
        )
        .await?;

        self.get_proposal(id).await?.ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "SkillProposal".to_string(),
                id: id.to_string(),
            })
        })
    }

    pub async fn mark_installed(
        &self,
        id: &str,
        installed_skill_name: &str,
        actor: Option<&str>,
        note: Option<&str>,
    ) -> Result<SkillProposal> {
        let current = self.get_proposal(id).await?.ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "SkillProposal".to_string(),
                id: id.to_string(),
            })
        })?;
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE skill_proposals
            SET status = 'installed',
                installed_skill_name = ?,
                updated_at = ?,
                installed_at = ?
            WHERE id = ?
            "#,
        )
        .bind(installed_skill_name)
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to mark skill proposal installed: {error}"
            )))
        })?;

        self.insert_history(
            id,
            "installed",
            actor,
            note,
            Some(Self::verification_status_to_string(
                current.verification_status,
            )),
            Some(installed_skill_name),
        )
        .await?;

        self.get_proposal(id).await?.ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "SkillProposal".to_string(),
                id: id.to_string(),
            })
        })
    }

    pub async fn mark_rolled_back(
        &self,
        id: &str,
        actor: Option<&str>,
        reason: Option<&str>,
    ) -> Result<SkillProposal> {
        let current = self.get_proposal(id).await?.ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "SkillProposal".to_string(),
                id: id.to_string(),
            })
        })?;
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE skill_proposals
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
                "Failed to mark skill proposal rolled back: {error}"
            )))
        })?;

        self.insert_history(
            id,
            "rolled_back",
            actor,
            reason,
            Some(Self::verification_status_to_string(
                current.verification_status,
            )),
            current.installed_skill_name.as_deref(),
        )
        .await?;

        self.get_proposal(id).await?.ok_or_else(|| {
            Error::Database(DatabaseError::NotFound {
                entity: "SkillProposal".to_string(),
                id: id.to_string(),
            })
        })
    }

    async fn insert_history(
        &self,
        proposal_id: &str,
        action: &str,
        actor: Option<&str>,
        note: Option<&str>,
        verification_status: Option<&str>,
        installed_skill_name: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO skill_proposal_history (
                id, proposal_id, action, actor, note, verification_status,
                installed_skill_name, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(proposal_id)
        .bind(action)
        .bind(actor)
        .bind(note)
        .bind(verification_status)
        .bind(installed_skill_name)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|error| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to insert skill proposal history: {error}"
            )))
        })?;
        Ok(())
    }

    fn row_to_proposal(row: &DbSkillProposalRow) -> Result<SkillProposal> {
        let verification_report =
            row.verification_summary
                .as_ref()
                .map(|summary| SkillProposalVerificationReport {
                    status: Self::verification_status_from_str(&row.verification_status)
                        .unwrap_or(SkillProposalVerificationStatus::Pending),
                    summary: summary.clone(),
                    compiled_skill_name: row.verification_compiled_skill_name.clone(),
                    verification_artifact_path: row.verification_artifact_path.clone(),
                    blocked: row.verification_blocked.unwrap_or(0) == 1,
                });

        Ok(SkillProposal {
            id: row.id.clone(),
            namespace: row.namespace.clone(),
            skill_name: row.skill_name.clone(),
            summary: row.summary.clone(),
            body: row.body.clone(),
            rationale: row.rationale.clone(),
            status: Self::status_from_str(&row.status)?,
            verification_status: Self::verification_status_from_str(&row.verification_status)?,
            source: SkillProposalSourceRef {
                kind: Self::source_kind_from_str(&row.source_kind)?,
                source_id: row.source_id.clone(),
                detail: row.source_detail.clone(),
            },
            artifact_path: row.artifact_path.clone(),
            review_note: row.review_note.clone(),
            reviewed_by: row.reviewed_by.clone(),
            verification_report,
            verified_by: row.verified_by.clone(),
            installed_skill_name: row.installed_skill_name.clone(),
            created_at: parse_timestamp(&row.created_at)?,
            updated_at: parse_timestamp(&row.updated_at)?,
            reviewed_at: row
                .reviewed_at
                .as_deref()
                .map(parse_timestamp)
                .transpose()?,
            verified_at: row
                .verified_at
                .as_deref()
                .map(parse_timestamp)
                .transpose()?,
            installed_at: row
                .installed_at
                .as_deref()
                .map(parse_timestamp)
                .transpose()?,
            rolled_back_at: row
                .rolled_back_at
                .as_deref()
                .map(parse_timestamp)
                .transpose()?,
        })
    }

    fn status_to_string(value: SkillProposalStatus) -> &'static str {
        match value {
            SkillProposalStatus::PendingReview => "pending_review",
            SkillProposalStatus::Approved => "approved",
            SkillProposalStatus::Rejected => "rejected",
            SkillProposalStatus::Superseded => "superseded",
            SkillProposalStatus::Installed => "installed",
            SkillProposalStatus::RolledBack => "rolled_back",
        }
    }

    fn status_from_str(value: &str) -> Result<SkillProposalStatus> {
        match value {
            "pending_review" => Ok(SkillProposalStatus::PendingReview),
            "approved" => Ok(SkillProposalStatus::Approved),
            "rejected" => Ok(SkillProposalStatus::Rejected),
            "superseded" => Ok(SkillProposalStatus::Superseded),
            "installed" => Ok(SkillProposalStatus::Installed),
            "rolled_back" => Ok(SkillProposalStatus::RolledBack),
            other => Err(Error::Database(DatabaseError::Query(format!(
                "Unknown skill proposal status '{other}'"
            )))),
        }
    }

    fn verification_status_to_string(value: SkillProposalVerificationStatus) -> &'static str {
        match value {
            SkillProposalVerificationStatus::Pending => "pending",
            SkillProposalVerificationStatus::Passed => "passed",
            SkillProposalVerificationStatus::Failed => "failed",
            SkillProposalVerificationStatus::Blocked => "blocked",
        }
    }

    fn verification_status_from_str(value: &str) -> Result<SkillProposalVerificationStatus> {
        match value {
            "pending" => Ok(SkillProposalVerificationStatus::Pending),
            "passed" => Ok(SkillProposalVerificationStatus::Passed),
            "failed" => Ok(SkillProposalVerificationStatus::Failed),
            "blocked" => Ok(SkillProposalVerificationStatus::Blocked),
            other => Err(Error::Database(DatabaseError::Query(format!(
                "Unknown skill proposal verification status '{other}'"
            )))),
        }
    }

    fn source_kind_to_string(value: SkillProposalSourceKind) -> &'static str {
        match value {
            SkillProposalSourceKind::LearningCandidate => "learning_candidate",
            SkillProposalSourceKind::Lesson => "lesson",
            SkillProposalSourceKind::Manual => "manual",
        }
    }

    fn source_kind_from_str(value: &str) -> Result<SkillProposalSourceKind> {
        match value {
            "learning_candidate" => Ok(SkillProposalSourceKind::LearningCandidate),
            "lesson" => Ok(SkillProposalSourceKind::Lesson),
            "manual" => Ok(SkillProposalSourceKind::Manual),
            other => Err(Error::Database(DatabaseError::Query(format!(
                "Unknown skill proposal source kind '{other}'"
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
    use crate::models::SkillProposalHistoryRow;
    use crate::{init_pool, run_migrations};
    use openrustclaw_core::types::{SkillProposalReviewAction, SkillProposalSourceRef};

    fn proposal(id: &str) -> SkillProposal {
        SkillProposal {
            id: id.to_string(),
            namespace: "user-1".to_string(),
            skill_name: "triage-helper".to_string(),
            summary: "Convert recurring triage steps into a reusable skill".to_string(),
            body: "# Triage Helper\n\nReusable triage instructions.\n".to_string(),
            rationale: Some("Seen repeatedly across approved candidates".to_string()),
            status: SkillProposalStatus::PendingReview,
            verification_status: SkillProposalVerificationStatus::Pending,
            source: SkillProposalSourceRef {
                kind: SkillProposalSourceKind::LearningCandidate,
                source_id: "candidate-1".to_string(),
                detail: Some("lesson-1".to_string()),
            },
            artifact_path: "/tmp/skill-proposals/triage-helper/SKILL.md".to_string(),
            review_note: None,
            reviewed_by: None,
            verification_report: None,
            verified_by: None,
            installed_skill_name: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            reviewed_at: None,
            verified_at: None,
            installed_at: None,
            rolled_back_at: None,
        }
    }

    #[tokio::test]
    async fn skill_proposal_store_round_trips_review_and_install() {
        let pool = init_pool("sqlite::memory:", 1).await.unwrap();
        run_migrations(&pool).await.unwrap();
        let store = SqliteSkillProposalStore::new(pool.clone());

        let created = store
            .create_proposal(&proposal("proposal-1"))
            .await
            .unwrap();
        assert_eq!(created.status, SkillProposalStatus::PendingReview);
        assert_eq!(
            created.verification_status,
            SkillProposalVerificationStatus::Pending
        );

        let reviewed = store
            .review_proposal(
                &created.id,
                &SkillProposalReviewRequest {
                    action: SkillProposalReviewAction::Approve,
                    reviewed_by: Some("operator".to_string()),
                    review_note: Some("Looks reusable".to_string()),
                },
            )
            .await
            .unwrap();
        assert_eq!(reviewed.status, SkillProposalStatus::Approved);

        let verified = store
            .record_verification(
                &created.id,
                &SkillProposalVerificationReport {
                    status: SkillProposalVerificationStatus::Passed,
                    summary: "compiled successfully".to_string(),
                    compiled_skill_name: Some("Triage Helper".to_string()),
                    verification_artifact_path: Some("/tmp/preview".to_string()),
                    blocked: false,
                },
                Some("operator"),
                Some("preview compile ok"),
            )
            .await
            .unwrap();
        assert_eq!(
            verified.verification_status,
            SkillProposalVerificationStatus::Passed
        );

        let installed = store
            .mark_installed(&created.id, "Triage Helper", Some("operator"), None)
            .await
            .unwrap();
        assert_eq!(installed.status, SkillProposalStatus::Installed);
        assert_eq!(
            installed.installed_skill_name.as_deref(),
            Some("Triage Helper")
        );

        let history = sqlx::query_as::<_, SkillProposalHistoryRow>(
            "SELECT * FROM skill_proposal_history WHERE proposal_id = ? ORDER BY created_at ASC",
        )
        .bind(&created.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert!(history.len() >= 4);
    }
}

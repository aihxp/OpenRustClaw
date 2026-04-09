use async_trait::async_trait;

use openrustclaw_core::error::{Error, MemoryError, Result};
use openrustclaw_core::types::{
    LearningCandidate, LearningCandidateCreateRequest, LearningCandidateEvidenceKind,
    LearningCandidatePromotionReport, LearningCandidatePromotionRequest,
    LearningCandidateQuarantineRequest, LearningCandidateReviewRequest,
    LearningCandidateRollbackRequest, LearningCandidateStatus,
};

use crate::autonomy_lessons_control::AutonomyLessonRequest;

#[async_trait]
pub trait LearningReviewSource: Send + Sync {
    async fn list_learning_candidates(
        &self,
        namespace: Option<&str>,
        status: Option<LearningCandidateStatus>,
        limit: usize,
    ) -> Result<Vec<LearningCandidate>>;

    async fn get_learning_candidate(&self, id: &str) -> Result<Option<LearningCandidate>>;

    async fn create_learning_candidate(
        &self,
        request: &LearningCandidateCreateRequest,
    ) -> Result<LearningCandidate>;

    async fn review_learning_candidate(
        &self,
        id: &str,
        request: &LearningCandidateReviewRequest,
    ) -> Result<LearningCandidate>;

    async fn mark_learning_candidate_promoted(
        &self,
        id: &str,
        lesson_id: &str,
        actor: Option<&str>,
        note: Option<&str>,
    ) -> Result<LearningCandidate>;

    async fn mark_learning_candidate_rolled_back(
        &self,
        id: &str,
        actor: Option<&str>,
        reason: Option<&str>,
    ) -> Result<LearningCandidate>;

    async fn mark_learning_candidate_quarantined(
        &self,
        id: &str,
        actor: Option<&str>,
        reason: Option<&str>,
    ) -> Result<LearningCandidate>;

    async fn create_lesson(&self, request: &AutonomyLessonRequest) -> Result<()>;

    async fn deactivate_lesson(&self, id: &str) -> Result<()>;
}

pub struct LearningReviewService<S> {
    source: S,
}

impl<S> LearningReviewService<S> {
    pub fn new(source: S) -> Self {
        Self { source }
    }
}

impl<S> LearningReviewService<S>
where
    S: LearningReviewSource,
{
    pub async fn list(
        &self,
        namespace: Option<&str>,
        status: Option<LearningCandidateStatus>,
        limit: usize,
    ) -> Result<Vec<LearningCandidate>> {
        self.source
            .list_learning_candidates(namespace, status, limit)
            .await
    }

    pub async fn get(&self, id: &str) -> Result<Option<LearningCandidate>> {
        self.source.get_learning_candidate(id).await
    }

    pub async fn queue(
        &self,
        request: &LearningCandidateCreateRequest,
    ) -> Result<LearningCandidate> {
        if request.signal.trim().is_empty() || request.recommendation.trim().is_empty() {
            return Err(Error::Memory(MemoryError::Store(
                "Learning candidates require both a signal and recommendation.".to_string(),
            )));
        }
        let mut request = request.clone();
        if !request.god_mode_origin && request.autonomy_level.as_deref() == Some("yolo") {
            request.god_mode_origin = true;
        }
        self.source.create_learning_candidate(&request).await
    }

    pub async fn review(
        &self,
        id: &str,
        request: &LearningCandidateReviewRequest,
    ) -> Result<LearningCandidate> {
        self.source.review_learning_candidate(id, request).await
    }

    pub async fn promote(
        &self,
        id: &str,
        request: &LearningCandidatePromotionRequest,
    ) -> Result<LearningCandidatePromotionReport> {
        let candidate = self
            .source
            .get_learning_candidate(id)
            .await?
            .ok_or_else(|| Error::Internal(format!("Learning candidate '{id}' not found")))?;

        if candidate.status != LearningCandidateStatus::Approved {
            return Err(Error::Memory(MemoryError::Store(format!(
                "Learning candidate '{}' must be approved before promotion.",
                candidate.id
            ))));
        }

        if candidate.impact.requires_review_evidence()
            && !candidate.evidence.iter().any(has_promotion_weight)
        {
            return Err(Error::Memory(MemoryError::Store(format!(
                "Learning candidate '{}' needs replay, evaluation, or audit evidence before promotion.",
                candidate.id
            ))));
        }

        let lesson_id = request
            .lesson_id
            .clone()
            .unwrap_or_else(|| format!("lesson-{}", candidate.id));
        self.source
            .create_lesson(&AutonomyLessonRequest {
                id: lesson_id.clone(),
                active: request.active,
                signal: candidate.signal.clone(),
                recommendation: candidate.recommendation.clone(),
                rationale: candidate.rationale.clone(),
                confidence: Some(candidate.confidence),
                source: Some(format!(
                    "learning_candidate:{}:{}",
                    candidate.source.source_id, candidate.kind
                )),
                task_id: candidate.task_id.clone(),
                category: candidate.category.clone(),
                claw_id: candidate.claw_id.clone(),
                model_profile_id: candidate.model_profile_id.clone(),
                provider: candidate.provider.clone(),
                autonomy_level: candidate.autonomy_level.clone(),
                execution_mode: candidate.execution_mode.clone(),
            })
            .await?;

        let candidate = match self
            .source
            .mark_learning_candidate_promoted(
                &candidate.id,
                &lesson_id,
                request.promoted_by.as_deref(),
                Some("promoted to active lesson"),
            )
            .await
        {
            Ok(candidate) => candidate,
            Err(error) => {
                if let Err(cleanup_error) = self.source.deactivate_lesson(&lesson_id).await {
                    return Err(Error::Internal(format!(
                        "failed to persist learning candidate promotion for lesson '{lesson_id}': {error}; cleanup failed: {cleanup_error}"
                    )));
                }
                return Err(error);
            }
        };

        Ok(LearningCandidatePromotionReport {
            candidate,
            lesson_id,
        })
    }

    pub async fn rollback(
        &self,
        id: &str,
        request: &LearningCandidateRollbackRequest,
    ) -> Result<LearningCandidate> {
        let candidate = self
            .source
            .get_learning_candidate(id)
            .await?
            .ok_or_else(|| Error::Internal(format!("Learning candidate '{id}' not found")))?;

        let lesson_id = candidate.promoted_lesson_id.clone().ok_or_else(|| {
            Error::Memory(MemoryError::Store(format!(
                "Learning candidate '{}' has no promoted lesson to roll back.",
                candidate.id
            )))
        })?;

        self.source.deactivate_lesson(&lesson_id).await?;
        self.source
            .mark_learning_candidate_rolled_back(
                id,
                request.rolled_back_by.as_deref(),
                request.reason.as_deref(),
            )
            .await
    }

    pub async fn quarantine(
        &self,
        id: &str,
        request: &LearningCandidateQuarantineRequest,
    ) -> Result<LearningCandidate> {
        let candidate = self
            .source
            .get_learning_candidate(id)
            .await?
            .ok_or_else(|| Error::Internal(format!("Learning candidate '{id}' not found")))?;

        if let Some(lesson_id) = candidate.promoted_lesson_id.as_deref() {
            self.source.deactivate_lesson(lesson_id).await?;
        }

        self.source
            .mark_learning_candidate_quarantined(
                id,
                request.quarantined_by.as_deref(),
                request.reason.as_deref(),
            )
            .await
    }
}

fn has_promotion_weight(evidence: &openrustclaw_core::types::LearningCandidateEvidence) -> bool {
    matches!(
        evidence.kind,
        LearningCandidateEvidenceKind::Replay
            | LearningCandidateEvidenceKind::Evaluation
            | LearningCandidateEvidenceKind::Audit
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    use chrono::Utc;

    use super::*;
    use openrustclaw_core::types::{
        LearningCandidateEvidence, LearningCandidateEvidenceInput, LearningCandidateImpact,
        LearningCandidateReviewAction, LearningCandidateSourceKind, LearningCandidateSourceRef,
    };

    #[derive(Clone, Default)]
    struct MockLearningReviewSource {
        candidates: Arc<Mutex<BTreeMap<String, LearningCandidate>>>,
        created_lessons: Arc<Mutex<Vec<String>>>,
        deactivated_lessons: Arc<Mutex<Vec<String>>>,
        fail_mark_promoted: Arc<Mutex<bool>>,
    }

    #[async_trait]
    impl LearningReviewSource for MockLearningReviewSource {
        async fn list_learning_candidates(
            &self,
            _namespace: Option<&str>,
            _status: Option<LearningCandidateStatus>,
            _limit: usize,
        ) -> Result<Vec<LearningCandidate>> {
            Ok(self.candidates.lock().unwrap().values().cloned().collect())
        }

        async fn get_learning_candidate(&self, id: &str) -> Result<Option<LearningCandidate>> {
            Ok(self.candidates.lock().unwrap().get(id).cloned())
        }

        async fn create_learning_candidate(
            &self,
            request: &LearningCandidateCreateRequest,
        ) -> Result<LearningCandidate> {
            let candidate = LearningCandidate {
                id: "candidate-1".to_string(),
                namespace: request.namespace.clone(),
                kind: request.kind.clone(),
                signal: request.signal.clone(),
                recommendation: request.recommendation.clone(),
                rationale: request.rationale.clone(),
                confidence: request.confidence,
                impact: request.impact,
                status: LearningCandidateStatus::PendingReview,
                source: request.source.clone(),
                evidence: request
                    .evidence
                    .iter()
                    .enumerate()
                    .map(|(index, evidence)| LearningCandidateEvidence {
                        id: format!("evidence-{index}"),
                        kind: evidence.kind,
                        summary: evidence.summary.clone(),
                        source_id: evidence.source_id.clone(),
                        recorded_by: evidence.recorded_by.clone(),
                        created_at: Utc::now(),
                    })
                    .collect(),
                review_note: None,
                reviewed_by: None,
                god_mode_origin: request.god_mode_origin,
                task_id: request.task_id.clone(),
                category: request.category.clone(),
                claw_id: request.claw_id.clone(),
                model_profile_id: request.model_profile_id.clone(),
                provider: request.provider.clone(),
                autonomy_level: request.autonomy_level.clone(),
                execution_mode: request.execution_mode.clone(),
                promoted_lesson_id: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                reviewed_at: None,
                quarantined_at: None,
                quarantined_by: None,
                quarantine_reason: None,
                rolled_back_at: None,
            };
            self.candidates
                .lock()
                .unwrap()
                .insert(candidate.id.clone(), candidate.clone());
            Ok(candidate)
        }

        async fn review_learning_candidate(
            &self,
            id: &str,
            request: &LearningCandidateReviewRequest,
        ) -> Result<LearningCandidate> {
            let mut candidates = self.candidates.lock().unwrap();
            let candidate = candidates.get_mut(id).unwrap();
            candidate.status = match request.action {
                LearningCandidateReviewAction::Approve => LearningCandidateStatus::Approved,
                LearningCandidateReviewAction::Reject => LearningCandidateStatus::Rejected,
                LearningCandidateReviewAction::Supersede => LearningCandidateStatus::Superseded,
            };
            candidate.review_note = request.review_note.clone();
            candidate.reviewed_by = request.reviewed_by.clone();
            candidate.reviewed_at = Some(Utc::now());
            Ok(candidate.clone())
        }

        async fn mark_learning_candidate_promoted(
            &self,
            id: &str,
            lesson_id: &str,
            actor: Option<&str>,
            _note: Option<&str>,
        ) -> Result<LearningCandidate> {
            if *self.fail_mark_promoted.lock().unwrap() {
                return Err(Error::Internal("promotion persistence failed".to_string()));
            }
            let mut candidates = self.candidates.lock().unwrap();
            let candidate = candidates.get_mut(id).unwrap();
            candidate.status = LearningCandidateStatus::Promoted;
            candidate.promoted_lesson_id = Some(lesson_id.to_string());
            candidate.reviewed_by = actor.map(ToString::to_string);
            Ok(candidate.clone())
        }

        async fn mark_learning_candidate_rolled_back(
            &self,
            id: &str,
            actor: Option<&str>,
            reason: Option<&str>,
        ) -> Result<LearningCandidate> {
            let mut candidates = self.candidates.lock().unwrap();
            let candidate = candidates.get_mut(id).unwrap();
            candidate.status = LearningCandidateStatus::RolledBack;
            candidate.reviewed_by = actor.map(ToString::to_string);
            candidate.review_note = reason.map(ToString::to_string);
            candidate.rolled_back_at = Some(Utc::now());
            Ok(candidate.clone())
        }

        async fn mark_learning_candidate_quarantined(
            &self,
            id: &str,
            actor: Option<&str>,
            reason: Option<&str>,
        ) -> Result<LearningCandidate> {
            let mut candidates = self.candidates.lock().unwrap();
            let candidate = candidates.get_mut(id).unwrap();
            candidate.quarantined_at = Some(Utc::now());
            candidate.quarantined_by = actor.map(ToString::to_string);
            candidate.quarantine_reason = reason.map(ToString::to_string);
            if candidate.promoted_lesson_id.is_some() {
                candidate.status = LearningCandidateStatus::RolledBack;
                candidate.rolled_back_at = Some(Utc::now());
            }
            Ok(candidate.clone())
        }

        async fn create_lesson(&self, request: &AutonomyLessonRequest) -> Result<()> {
            self.created_lessons
                .lock()
                .unwrap()
                .push(request.id.clone());
            Ok(())
        }

        async fn deactivate_lesson(&self, id: &str) -> Result<()> {
            self.deactivated_lessons
                .lock()
                .unwrap()
                .push(id.to_string());
            Ok(())
        }
    }

    fn create_request(impact: LearningCandidateImpact) -> LearningCandidateCreateRequest {
        LearningCandidateCreateRequest {
            namespace: "user-1".to_string(),
            kind: "worker_status".to_string(),
            signal: "worker failed".to_string(),
            recommendation: "route safer".to_string(),
            rationale: Some("test".to_string()),
            confidence: 0.8,
            impact,
            source: LearningCandidateSourceRef {
                kind: LearningCandidateSourceKind::ReflectionCandidate,
                source_id: "receipt-1:0".to_string(),
                detail: None,
            },
            evidence: vec![LearningCandidateEvidenceInput {
                kind: LearningCandidateEvidenceKind::Replay,
                summary: "replayed safely".to_string(),
                source_id: Some("eval-1".to_string()),
                recorded_by: Some("tester".to_string()),
            }],
            god_mode_origin: false,
            task_id: None,
            category: Some("code".to_string()),
            claw_id: None,
            model_profile_id: None,
            provider: None,
            autonomy_level: None,
            execution_mode: None,
        }
    }

    #[tokio::test]
    async fn learning_review_requires_approval_before_promotion() {
        let source = MockLearningReviewSource::default();
        let service = LearningReviewService::new(source.clone());
        let candidate = service
            .queue(&create_request(LearningCandidateImpact::Standard))
            .await
            .unwrap();

        let error = service
            .promote(
                &candidate.id,
                &LearningCandidatePromotionRequest {
                    lesson_id: None,
                    active: true,
                    promoted_by: Some("operator".to_string()),
                },
            )
            .await
            .unwrap_err();
        assert!(error.to_string().contains("must be approved"));
    }

    #[tokio::test]
    async fn learning_review_promotes_and_rolls_back_with_evidence() {
        let source = MockLearningReviewSource::default();
        let service = LearningReviewService::new(source.clone());
        let candidate = service
            .queue(&create_request(LearningCandidateImpact::High))
            .await
            .unwrap();
        service
            .review(
                &candidate.id,
                &LearningCandidateReviewRequest {
                    action: LearningCandidateReviewAction::Approve,
                    reviewed_by: Some("operator".to_string()),
                    review_note: Some("approved".to_string()),
                },
            )
            .await
            .unwrap();

        let promoted = service
            .promote(
                &candidate.id,
                &LearningCandidatePromotionRequest {
                    lesson_id: Some("lesson-1".to_string()),
                    active: true,
                    promoted_by: Some("operator".to_string()),
                },
            )
            .await
            .unwrap();
        assert_eq!(promoted.lesson_id, "lesson-1");

        let rolled_back = service
            .rollback(
                &candidate.id,
                &LearningCandidateRollbackRequest {
                    rolled_back_by: Some("operator".to_string()),
                    reason: Some("unsafe".to_string()),
                },
            )
            .await
            .unwrap();
        assert_eq!(rolled_back.status, LearningCandidateStatus::RolledBack);
    }

    #[tokio::test]
    async fn learning_review_quarantine_deactivates_promoted_lessons() {
        let source = MockLearningReviewSource::default();
        let service = LearningReviewService::new(source.clone());
        let candidate = service
            .queue(&create_request(LearningCandidateImpact::High))
            .await
            .unwrap();
        service
            .review(
                &candidate.id,
                &LearningCandidateReviewRequest {
                    action: LearningCandidateReviewAction::Approve,
                    reviewed_by: Some("operator".to_string()),
                    review_note: None,
                },
            )
            .await
            .unwrap();
        let report = service
            .promote(
                &candidate.id,
                &LearningCandidatePromotionRequest {
                    lesson_id: Some("lesson-god".to_string()),
                    active: true,
                    promoted_by: Some("operator".to_string()),
                },
            )
            .await
            .unwrap();

        let quarantined = service
            .quarantine(
                &candidate.id,
                &LearningCandidateQuarantineRequest {
                    quarantined_by: Some("operator".to_string()),
                    reason: Some("god mode audit".to_string()),
                },
            )
            .await
            .unwrap();

        assert_eq!(quarantined.status, LearningCandidateStatus::RolledBack);
        assert!(quarantined.quarantined_at.is_some());
        assert_eq!(
            source.deactivated_lessons.lock().unwrap().as_slice(),
            &[report.lesson_id]
        );
    }

    #[tokio::test]
    async fn learning_review_deactivates_lesson_when_promotion_persistence_fails() {
        let source = MockLearningReviewSource::default();
        let service = LearningReviewService::new(source.clone());
        let candidate = service
            .queue(&create_request(LearningCandidateImpact::Standard))
            .await
            .unwrap();
        service
            .review(
                &candidate.id,
                &LearningCandidateReviewRequest {
                    action: LearningCandidateReviewAction::Approve,
                    reviewed_by: Some("operator".to_string()),
                    review_note: Some("approved".to_string()),
                },
            )
            .await
            .unwrap();
        *source.fail_mark_promoted.lock().unwrap() = true;

        let error = service
            .promote(
                &candidate.id,
                &LearningCandidatePromotionRequest {
                    lesson_id: Some("lesson-rollback".to_string()),
                    active: true,
                    promoted_by: Some("operator".to_string()),
                },
            )
            .await
            .unwrap_err();

        assert!(error.to_string().contains("promotion persistence failed"));
        assert_eq!(
            source.created_lessons.lock().unwrap().as_slice(),
            &["lesson-rollback".to_string()]
        );
        assert_eq!(
            source.deactivated_lessons.lock().unwrap().as_slice(),
            &["lesson-rollback".to_string()]
        );
        assert_eq!(
            source
                .candidates
                .lock()
                .unwrap()
                .get(&candidate.id)
                .unwrap()
                .status,
            LearningCandidateStatus::Approved
        );
    }
}

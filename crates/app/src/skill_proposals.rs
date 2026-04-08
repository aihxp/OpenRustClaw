use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use openrustclaw_core::error::{Error, MemoryError, Result};
use openrustclaw_core::types::{
    SkillProposal, SkillProposalCreateRequest, SkillProposalInstallReport,
    SkillProposalInstallRequest, SkillProposalQuarantineRequest, SkillProposalReviewRequest,
    SkillProposalRollbackRequest, SkillProposalSourceKind, SkillProposalStatus,
    SkillProposalVerificationReport, SkillProposalVerificationStatus, SkillProposalVerifyRequest,
};

#[async_trait]
pub trait SkillProposalSource: Send + Sync {
    async fn list_skill_proposals(
        &self,
        namespace: Option<&str>,
        status: Option<SkillProposalStatus>,
        limit: usize,
    ) -> Result<Vec<SkillProposal>>;

    async fn get_skill_proposal(&self, id: &str) -> Result<Option<SkillProposal>>;

    async fn learning_candidate_god_mode_origin(&self, id: &str) -> Result<bool>;

    async fn create_skill_proposal(&self, proposal: &SkillProposal) -> Result<SkillProposal>;

    async fn review_skill_proposal(
        &self,
        id: &str,
        request: &SkillProposalReviewRequest,
    ) -> Result<SkillProposal>;

    async fn record_skill_proposal_verification(
        &self,
        id: &str,
        report: &SkillProposalVerificationReport,
        actor: Option<&str>,
        note: Option<&str>,
    ) -> Result<SkillProposal>;

    async fn mark_skill_proposal_installed(
        &self,
        id: &str,
        installed_skill_name: &str,
        actor: Option<&str>,
        note: Option<&str>,
    ) -> Result<SkillProposal>;

    async fn mark_skill_proposal_rolled_back(
        &self,
        id: &str,
        actor: Option<&str>,
        reason: Option<&str>,
    ) -> Result<SkillProposal>;

    async fn mark_skill_proposal_quarantined(
        &self,
        id: &str,
        actor: Option<&str>,
        reason: Option<&str>,
    ) -> Result<SkillProposal>;

    async fn write_proposal_artifact(&self, proposal: &SkillProposal) -> Result<String>;

    async fn verify_proposal_artifact(
        &self,
        proposal: &SkillProposal,
    ) -> Result<SkillProposalVerificationReport>;

    async fn install_skill_proposal(&self, proposal: &SkillProposal) -> Result<String>;

    async fn rollback_installed_skill(&self, skill_name: &str) -> Result<()>;
}

pub struct SkillProposalService<S> {
    source: S,
}

impl<S> SkillProposalService<S> {
    pub fn new(source: S) -> Self {
        Self { source }
    }
}

impl<S> SkillProposalService<S>
where
    S: SkillProposalSource,
{
    pub async fn list(
        &self,
        namespace: Option<&str>,
        status: Option<SkillProposalStatus>,
        limit: usize,
    ) -> Result<Vec<SkillProposal>> {
        self.source
            .list_skill_proposals(namespace, status, limit)
            .await
    }

    pub async fn get(&self, id: &str) -> Result<Option<SkillProposal>> {
        self.source.get_skill_proposal(id).await
    }

    pub async fn queue(&self, request: &SkillProposalCreateRequest) -> Result<SkillProposal> {
        if request.skill_name.trim().is_empty()
            || request.summary.trim().is_empty()
            || request.body.trim().is_empty()
        {
            return Err(Error::Memory(MemoryError::Store(
                "Skill proposals require a skill name, summary, and body.".to_string(),
            )));
        }

        let god_mode_origin = if request.god_mode_origin {
            true
        } else if request.source.kind == SkillProposalSourceKind::LearningCandidate {
            self.source
                .learning_candidate_god_mode_origin(&request.source.source_id)
                .await?
        } else {
            false
        };

        let now = Utc::now();
        let mut proposal = SkillProposal {
            id: Uuid::new_v4().to_string(),
            namespace: request.namespace.clone(),
            skill_name: request.skill_name.trim().to_string(),
            summary: request.summary.trim().to_string(),
            body: request.body.clone(),
            rationale: request.rationale.clone(),
            status: SkillProposalStatus::PendingReview,
            verification_status: SkillProposalVerificationStatus::Pending,
            source: request.source.clone(),
            artifact_path: String::new(),
            review_note: None,
            reviewed_by: None,
            god_mode_origin,
            verification_report: None,
            verified_by: None,
            installed_skill_name: None,
            created_at: now,
            updated_at: now,
            reviewed_at: None,
            quarantined_at: None,
            quarantined_by: None,
            quarantine_reason: None,
            verified_at: None,
            installed_at: None,
            rolled_back_at: None,
        };

        proposal.artifact_path = self.source.write_proposal_artifact(&proposal).await?;
        self.source.create_skill_proposal(&proposal).await
    }

    pub async fn review(
        &self,
        id: &str,
        request: &SkillProposalReviewRequest,
    ) -> Result<SkillProposal> {
        self.source.review_skill_proposal(id, request).await
    }

    pub async fn verify(
        &self,
        id: &str,
        request: &SkillProposalVerifyRequest,
    ) -> Result<SkillProposal> {
        let proposal = self
            .source
            .get_skill_proposal(id)
            .await?
            .ok_or_else(|| Error::Internal(format!("Skill proposal '{id}' not found")))?;

        if proposal.status != SkillProposalStatus::Approved {
            return Err(Error::Memory(MemoryError::Store(format!(
                "Skill proposal '{}' must be approved before verification.",
                proposal.id
            ))));
        }

        let report = self.source.verify_proposal_artifact(&proposal).await?;
        self.source
            .record_skill_proposal_verification(
                &proposal.id,
                &report,
                request.verified_by.as_deref(),
                request.note.as_deref(),
            )
            .await
    }

    pub async fn install(
        &self,
        id: &str,
        request: &SkillProposalInstallRequest,
    ) -> Result<SkillProposalInstallReport> {
        let proposal = self
            .source
            .get_skill_proposal(id)
            .await?
            .ok_or_else(|| Error::Internal(format!("Skill proposal '{id}' not found")))?;

        if proposal.status != SkillProposalStatus::Approved {
            return Err(Error::Memory(MemoryError::Store(format!(
                "Skill proposal '{}' must be approved before install.",
                proposal.id
            ))));
        }

        if proposal.verification_status != SkillProposalVerificationStatus::Passed {
            return Err(Error::Memory(MemoryError::Store(format!(
                "Skill proposal '{}' must pass verification before install.",
                proposal.id
            ))));
        }

        let installed_skill_name = self.source.install_skill_proposal(&proposal).await?;
        let proposal = self
            .source
            .mark_skill_proposal_installed(
                &proposal.id,
                &installed_skill_name,
                request.installed_by.as_deref(),
                request.note.as_deref(),
            )
            .await?;

        Ok(SkillProposalInstallReport {
            proposal,
            installed_skill_name,
        })
    }

    pub async fn rollback(
        &self,
        id: &str,
        request: &SkillProposalRollbackRequest,
    ) -> Result<SkillProposal> {
        let proposal = self
            .source
            .get_skill_proposal(id)
            .await?
            .ok_or_else(|| Error::Internal(format!("Skill proposal '{id}' not found")))?;

        let skill_name = proposal.installed_skill_name.clone().ok_or_else(|| {
            Error::Memory(MemoryError::Store(format!(
                "Skill proposal '{}' has no installed skill to roll back.",
                proposal.id
            )))
        })?;

        self.source.rollback_installed_skill(&skill_name).await?;
        self.source
            .mark_skill_proposal_rolled_back(
                id,
                request.rolled_back_by.as_deref(),
                request.reason.as_deref(),
            )
            .await
    }

    pub async fn quarantine(
        &self,
        id: &str,
        request: &SkillProposalQuarantineRequest,
    ) -> Result<SkillProposal> {
        let proposal = self
            .source
            .get_skill_proposal(id)
            .await?
            .ok_or_else(|| Error::Internal(format!("Skill proposal '{id}' not found")))?;

        if let Some(skill_name) = proposal.installed_skill_name.as_deref() {
            self.source.rollback_installed_skill(skill_name).await?;
        }

        self.source
            .mark_skill_proposal_quarantined(
                id,
                request.quarantined_by.as_deref(),
                request.reason.as_deref(),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    use super::*;
    use openrustclaw_core::types::{
        SkillProposalReviewAction, SkillProposalSourceKind, SkillProposalSourceRef,
    };

    #[derive(Clone, Default)]
    struct MockSkillProposalSource {
        proposals: Arc<Mutex<BTreeMap<String, SkillProposal>>>,
        installed: Arc<Mutex<Vec<String>>>,
        rolled_back: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl SkillProposalSource for MockSkillProposalSource {
        async fn list_skill_proposals(
            &self,
            _namespace: Option<&str>,
            _status: Option<SkillProposalStatus>,
            _limit: usize,
        ) -> Result<Vec<SkillProposal>> {
            Ok(self.proposals.lock().unwrap().values().cloned().collect())
        }

        async fn get_skill_proposal(&self, id: &str) -> Result<Option<SkillProposal>> {
            Ok(self.proposals.lock().unwrap().get(id).cloned())
        }

        async fn learning_candidate_god_mode_origin(&self, id: &str) -> Result<bool> {
            Ok(id == "candidate-god")
        }

        async fn create_skill_proposal(&self, proposal: &SkillProposal) -> Result<SkillProposal> {
            self.proposals
                .lock()
                .unwrap()
                .insert(proposal.id.clone(), proposal.clone());
            Ok(proposal.clone())
        }

        async fn review_skill_proposal(
            &self,
            id: &str,
            request: &SkillProposalReviewRequest,
        ) -> Result<SkillProposal> {
            let mut proposals = self.proposals.lock().unwrap();
            let proposal = proposals.get_mut(id).unwrap();
            proposal.status = match request.action {
                SkillProposalReviewAction::Approve => SkillProposalStatus::Approved,
                SkillProposalReviewAction::Reject => SkillProposalStatus::Rejected,
                SkillProposalReviewAction::Supersede => SkillProposalStatus::Superseded,
            };
            proposal.review_note = request.review_note.clone();
            proposal.reviewed_by = request.reviewed_by.clone();
            proposal.reviewed_at = Some(Utc::now());
            Ok(proposal.clone())
        }

        async fn record_skill_proposal_verification(
            &self,
            id: &str,
            report: &SkillProposalVerificationReport,
            actor: Option<&str>,
            _note: Option<&str>,
        ) -> Result<SkillProposal> {
            let mut proposals = self.proposals.lock().unwrap();
            let proposal = proposals.get_mut(id).unwrap();
            proposal.verification_status = report.status;
            proposal.verification_report = Some(report.clone());
            proposal.verified_by = actor.map(ToString::to_string);
            proposal.verified_at = Some(Utc::now());
            Ok(proposal.clone())
        }

        async fn mark_skill_proposal_installed(
            &self,
            id: &str,
            installed_skill_name: &str,
            _actor: Option<&str>,
            _note: Option<&str>,
        ) -> Result<SkillProposal> {
            let mut proposals = self.proposals.lock().unwrap();
            let proposal = proposals.get_mut(id).unwrap();
            proposal.status = SkillProposalStatus::Installed;
            proposal.installed_skill_name = Some(installed_skill_name.to_string());
            proposal.installed_at = Some(Utc::now());
            Ok(proposal.clone())
        }

        async fn mark_skill_proposal_rolled_back(
            &self,
            id: &str,
            _actor: Option<&str>,
            _reason: Option<&str>,
        ) -> Result<SkillProposal> {
            let mut proposals = self.proposals.lock().unwrap();
            let proposal = proposals.get_mut(id).unwrap();
            proposal.status = SkillProposalStatus::RolledBack;
            proposal.rolled_back_at = Some(Utc::now());
            Ok(proposal.clone())
        }

        async fn mark_skill_proposal_quarantined(
            &self,
            id: &str,
            actor: Option<&str>,
            reason: Option<&str>,
        ) -> Result<SkillProposal> {
            let mut proposals = self.proposals.lock().unwrap();
            let proposal = proposals.get_mut(id).unwrap();
            proposal.quarantined_at = Some(Utc::now());
            proposal.quarantined_by = actor.map(ToString::to_string);
            proposal.quarantine_reason = reason.map(ToString::to_string);
            if proposal.installed_skill_name.is_some() {
                proposal.status = SkillProposalStatus::RolledBack;
                proposal.rolled_back_at = Some(Utc::now());
            }
            Ok(proposal.clone())
        }

        async fn write_proposal_artifact(&self, proposal: &SkillProposal) -> Result<String> {
            Ok(format!("/tmp/{}/SKILL.md", proposal.id))
        }

        async fn verify_proposal_artifact(
            &self,
            proposal: &SkillProposal,
        ) -> Result<SkillProposalVerificationReport> {
            Ok(SkillProposalVerificationReport {
                status: SkillProposalVerificationStatus::Passed,
                summary: "compiled successfully".to_string(),
                compiled_skill_name: Some(proposal.skill_name.clone()),
                verification_artifact_path: Some(format!("/tmp/{}/preview", proposal.id)),
                blocked: false,
            })
        }

        async fn install_skill_proposal(&self, proposal: &SkillProposal) -> Result<String> {
            self.installed
                .lock()
                .unwrap()
                .push(proposal.skill_name.clone());
            Ok(proposal.skill_name.clone())
        }

        async fn rollback_installed_skill(&self, skill_name: &str) -> Result<()> {
            self.rolled_back
                .lock()
                .unwrap()
                .push(skill_name.to_string());
            Ok(())
        }
    }

    #[tokio::test]
    async fn proposal_service_requires_approval_and_verification_before_install() {
        let service = SkillProposalService::new(MockSkillProposalSource::default());
        let proposal = service
            .queue(&openrustclaw_core::types::SkillProposalCreateRequest {
                namespace: "user-1".to_string(),
                skill_name: "triage-helper".to_string(),
                summary: "Turn repeated triage into a skill".to_string(),
                body: "# Triage Helper\n".to_string(),
                rationale: Some("observed repeatedly".to_string()),
                source: SkillProposalSourceRef {
                    kind: SkillProposalSourceKind::LearningCandidate,
                    source_id: "candidate-1".to_string(),
                    detail: None,
                },
                god_mode_origin: false,
            })
            .await
            .unwrap();

        assert!(
            service
                .install(&proposal.id, &SkillProposalInstallRequest::default())
                .await
                .is_err()
        );

        service
            .review(
                &proposal.id,
                &SkillProposalReviewRequest {
                    action: SkillProposalReviewAction::Approve,
                    reviewed_by: Some("operator".to_string()),
                    review_note: None,
                },
            )
            .await
            .unwrap();

        assert!(
            service
                .install(&proposal.id, &SkillProposalInstallRequest::default())
                .await
                .is_err()
        );

        service
            .verify(
                &proposal.id,
                &SkillProposalVerifyRequest {
                    verified_by: Some("operator".to_string()),
                    note: None,
                },
            )
            .await
            .unwrap();

        let report = service
            .install(
                &proposal.id,
                &SkillProposalInstallRequest {
                    installed_by: Some("operator".to_string()),
                    note: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(report.installed_skill_name, "triage-helper");
    }

    #[tokio::test]
    async fn proposal_service_derives_god_mode_origin_and_quarantines_installs() {
        let service = SkillProposalService::new(MockSkillProposalSource::default());
        let proposal = service
            .queue(&openrustclaw_core::types::SkillProposalCreateRequest {
                namespace: "user-1".to_string(),
                skill_name: "danger-helper".to_string(),
                summary: "Danger helper".to_string(),
                body: "# Danger Helper\n".to_string(),
                rationale: None,
                source: SkillProposalSourceRef {
                    kind: SkillProposalSourceKind::LearningCandidate,
                    source_id: "candidate-god".to_string(),
                    detail: None,
                },
                god_mode_origin: false,
            })
            .await
            .unwrap();

        assert!(proposal.god_mode_origin);

        service
            .review(
                &proposal.id,
                &SkillProposalReviewRequest {
                    action: SkillProposalReviewAction::Approve,
                    reviewed_by: Some("operator".to_string()),
                    review_note: None,
                },
            )
            .await
            .unwrap();
        service
            .verify(
                &proposal.id,
                &SkillProposalVerifyRequest {
                    verified_by: Some("operator".to_string()),
                    note: None,
                },
            )
            .await
            .unwrap();
        service
            .install(
                &proposal.id,
                &SkillProposalInstallRequest {
                    installed_by: Some("operator".to_string()),
                    note: None,
                },
            )
            .await
            .unwrap();

        let quarantined = service
            .quarantine(
                &proposal.id,
                &SkillProposalQuarantineRequest {
                    quarantined_by: Some("operator".to_string()),
                    reason: Some("contain god mode artifact".to_string()),
                },
            )
            .await
            .unwrap();

        assert_eq!(quarantined.status, SkillProposalStatus::RolledBack);
        assert!(quarantined.quarantined_at.is_some());
        assert_eq!(
            service.source.rolled_back.lock().unwrap().as_slice(),
            &["danger-helper".to_string()]
        );
    }
}

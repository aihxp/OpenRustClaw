use openrustclaw_core::error::{DatabaseError, Error, MemoryError, Result};
use openrustclaw_core::traits::CoreMemoryStore;
use openrustclaw_core::types::{
    ModelArtifact, ModelArtifactProjection, ModelArtifactPromotionRequest,
    ModelArtifactUpdateRequest,
};
use openrustclaw_db::{SqliteCoreMemoryStore, SqliteMemoryStore};

use crate::{CoreMemoryManager, MemoryPolicies};

pub const MODEL_ARTIFACT_MAX_SUMMARY_CHARS: usize = 220;

pub fn reserved_model_artifact_core_keys() -> &'static [&'static str] {
    &[
        "model.user",
        "model.operator",
        "model.project",
        "model.archive",
    ]
}

pub fn build_model_artifact_projections(
    artifacts: &[ModelArtifact],
    max_summary_chars: usize,
) -> Vec<ModelArtifactProjection> {
    let mut active = artifacts
        .iter()
        .filter(|artifact| artifact.status.is_active())
        .cloned()
        .collect::<Vec<_>>();
    active.sort_by_key(|artifact| artifact.kind.reserved_core_key());

    active
        .into_iter()
        .map(|artifact| {
            let value = clip_summary(&artifact.summary, max_summary_chars.max(32));
            let token_count = ((artifact.kind.reserved_core_key().len() + value.len() + 2) / 4) + 1;
            ModelArtifactProjection {
                artifact_id: artifact.id,
                namespace: artifact.namespace,
                kind: artifact.kind,
                key: artifact.kind.reserved_core_key().to_string(),
                value,
                token_count,
            }
        })
        .collect()
}

pub struct ModelArtifactService {
    memory_store: SqliteMemoryStore,
    core_memory_store: SqliteCoreMemoryStore,
    policies: MemoryPolicies,
    max_summary_chars: usize,
}

impl ModelArtifactService {
    pub fn new(memory_store: SqliteMemoryStore, core_memory_store: SqliteCoreMemoryStore) -> Self {
        Self {
            memory_store,
            core_memory_store,
            policies: MemoryPolicies::default(),
            max_summary_chars: MODEL_ARTIFACT_MAX_SUMMARY_CHARS,
        }
    }

    pub fn with_policies(mut self, policies: MemoryPolicies) -> Self {
        self.policies = policies;
        self
    }

    pub async fn list(
        &self,
        namespace: Option<&str>,
        include_inactive: bool,
        limit: usize,
    ) -> Result<Vec<ModelArtifact>> {
        self.memory_store
            .list_model_artifacts(namespace, include_inactive, limit)
            .await
    }

    pub async fn get(&self, id: &str) -> Result<Option<ModelArtifact>> {
        self.memory_store.get_model_artifact(id).await
    }

    pub async fn promote(&self, request: &ModelArtifactPromotionRequest) -> Result<ModelArtifact> {
        let decision = self.policies.evaluate_model_artifact_promotion(request);
        if !decision.allowed {
            return Err(Error::Memory(MemoryError::Store(decision.reason)));
        }

        let artifact = self.memory_store.promote_model_artifact(request).await?;
        self.sync_projection(&request.namespace).await?;
        Ok(artifact)
    }

    pub async fn update(
        &self,
        id: &str,
        request: &ModelArtifactUpdateRequest,
    ) -> Result<ModelArtifact> {
        if let Some(summary) = request.summary.as_deref() {
            if summary.trim().len() < 8 {
                return Err(Error::Memory(MemoryError::Store(
                    "Structured model artifact corrections require a concrete summary value."
                        .to_string(),
                )));
            }
        }

        let artifact = self.memory_store.update_model_artifact(id, request).await?;
        self.sync_projection(&artifact.namespace).await?;
        Ok(artifact)
    }

    pub async fn projected_entries(&self, namespace: &str) -> Result<Vec<ModelArtifactProjection>> {
        let artifacts = self
            .memory_store
            .list_model_artifacts(Some(namespace), false, 16)
            .await?;
        Ok(build_model_artifact_projections(
            &artifacts,
            self.max_summary_chars,
        ))
    }

    async fn sync_projection(&self, namespace: &str) -> Result<()> {
        let projections = self.projected_entries(namespace).await?;
        for key in reserved_model_artifact_core_keys() {
            match self.core_memory_store.remove(namespace, key).await {
                Ok(()) => {}
                Err(Error::Database(DatabaseError::NotFound { entity, .. }))
                    if entity == "CoreMemory" => {}
                Err(error) => return Err(error),
            }
        }
        for projection in projections {
            let mut entry = CoreMemoryManager::new_entry(&projection.key, &projection.value, 0.9);
            entry.token_count = projection.token_count;
            self.core_memory_store.set(namespace, entry).await?;
        }
        Ok(())
    }
}

fn clip_summary(summary: &str, max_summary_chars: usize) -> String {
    let trimmed = summary.trim();
    if trimmed.chars().count() <= max_summary_chars {
        return trimmed.to_string();
    }
    let clipped: String = trimmed.chars().take(max_summary_chars).collect();
    format!("{}...", clipped.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use openrustclaw_core::types::{
        ModelArtifact, ModelArtifactKind, ModelArtifactSourceKind, ModelArtifactSourceRef,
        ModelArtifactStatus,
    };

    fn model_artifact(
        kind: ModelArtifactKind,
        summary: &str,
        status: ModelArtifactStatus,
    ) -> ModelArtifact {
        ModelArtifact {
            id: format!("{kind:?}"),
            namespace: "user-1".to_string(),
            kind,
            summary: summary.to_string(),
            status,
            importance: 0.8,
            confidence: 0.9,
            source_lineage: vec![ModelArtifactSourceRef {
                kind: ModelArtifactSourceKind::MemoryEntry,
                source_id: "mem-1".to_string(),
                detail: None,
            }],
            promoted_by: Some("test".to_string()),
            correction_note: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deactivated_at: None,
        }
    }

    #[test]
    fn build_model_artifact_projections_ignores_inactive_rows() {
        let projections = build_model_artifact_projections(
            &[
                model_artifact(
                    ModelArtifactKind::UserModel,
                    "User summary",
                    ModelArtifactStatus::Active,
                ),
                model_artifact(
                    ModelArtifactKind::ProjectMemory,
                    "Project summary",
                    ModelArtifactStatus::Inactive,
                ),
            ],
            120,
        );
        assert_eq!(projections.len(), 1);
        assert_eq!(projections[0].key, "model.user");
    }

    #[test]
    fn build_model_artifact_projections_clips_long_values() {
        let projections = build_model_artifact_projections(
            &[model_artifact(
                ModelArtifactKind::ArchiveSummary,
                &"x".repeat(80),
                ModelArtifactStatus::Active,
            )],
            32,
        );
        assert_eq!(projections.len(), 1);
        assert!(projections[0].value.ends_with("..."));
        assert!(projections[0].value.len() <= 35);
    }
}

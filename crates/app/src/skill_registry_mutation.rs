use async_trait::async_trait;
use openrustclaw_core::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceSkillCandidate {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub schema: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistrySkillMetadata {
    pub description: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegistryInstallOutcome {
    AlreadyInstalled,
    Installed {
        name: String,
        version: String,
        path: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegistryUpdateOutcome {
    UpToDate,
    Updated { from: String, to: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillMutationReport<Detail> {
    pub status: String,
    pub action: String,
    pub skill_name: String,
    pub message: String,
    pub skill: Option<Detail>,
    pub verified: Option<bool>,
}

#[async_trait]
pub trait SkillRegistryMutationSource {
    type Detail;

    async fn skill_exists(&self, name: &str) -> Result<bool>;
    async fn load_skill_detail(&self, name: &str) -> Result<Self::Detail>;
    async fn ensure_skills_dir(&self) -> Result<()>;
    async fn workspace_skill_candidate(
        &self,
        name: &str,
    ) -> Result<Option<WorkspaceSkillCandidate>>;
    async fn persist_workspace_install(&self, candidate: &WorkspaceSkillCandidate) -> Result<()>;
    async fn compile_workspace_skill(&self, candidate: &WorkspaceSkillCandidate) -> Result<()>;
    async fn registry_skill_metadata(&self, name: &str) -> Result<RegistrySkillMetadata>;
    async fn enforce_registry_skill_policy(
        &self,
        name: &str,
        metadata: &RegistrySkillMetadata,
    ) -> Result<()>;
    async fn registry_install(&self, name: &str) -> Result<RegistryInstallOutcome>;
    async fn persist_registry_install(
        &self,
        name: &str,
        version: &str,
        metadata: &RegistrySkillMetadata,
    ) -> Result<()>;
    async fn compile_registry_install(&self, name: &str, path: &str) -> Result<()>;
    async fn registry_update(&self, name: &str) -> Result<RegistryUpdateOutcome>;
    async fn persist_registry_update(
        &self,
        name: &str,
        from: &str,
        to: &str,
        metadata: &RegistrySkillMetadata,
    ) -> Result<()>;
    async fn compile_updated_skill(&self, name: &str) -> Result<()>;
    async fn registry_uninstall_best_effort(&self, name: &str) -> Result<()>;
    async fn remove_installed_skill(&self, name: &str) -> Result<()>;
}

pub struct SkillRegistryMutationService<S> {
    source: S,
}

impl<S> SkillRegistryMutationService<S> {
    pub fn new(source: S) -> Self {
        Self { source }
    }
}

impl<S> SkillRegistryMutationService<S>
where
    S: SkillRegistryMutationSource + Send + Sync,
    S::Detail: Send,
{
    pub async fn install(&self, name: &str) -> Result<SkillMutationReport<S::Detail>> {
        if self.source.skill_exists(name).await? {
            return Ok(SkillMutationReport {
                status: "noop".to_string(),
                action: "install".to_string(),
                skill_name: name.to_string(),
                message: format!("Skill '{}' is already installed", name),
                skill: Some(self.source.load_skill_detail(name).await?),
                verified: None,
            });
        }

        self.source.ensure_skills_dir().await?;

        if let Some(candidate) = self.source.workspace_skill_candidate(name).await? {
            self.source.persist_workspace_install(&candidate).await?;
            let _ = self.source.compile_workspace_skill(&candidate).await;

            return Ok(SkillMutationReport {
                status: "ok".to_string(),
                action: "install".to_string(),
                skill_name: candidate.name.clone(),
                message: format!(
                    "Installed workspace skill '{}' from {}",
                    candidate.name, candidate.path
                ),
                skill: Some(self.source.load_skill_detail(&candidate.name).await?),
                verified: Some(true),
            });
        }

        let metadata = self.source.registry_skill_metadata(name).await?;
        self.source
            .enforce_registry_skill_policy(name, &metadata)
            .await?;

        match self.source.registry_install(name).await? {
            RegistryInstallOutcome::AlreadyInstalled => Ok(SkillMutationReport {
                status: "noop".to_string(),
                action: "install".to_string(),
                skill_name: name.to_string(),
                message: format!(
                    "Skill '{}' was already installed in the registry store",
                    name
                ),
                skill: if self.source.skill_exists(name).await? {
                    Some(self.source.load_skill_detail(name).await?)
                } else {
                    None
                },
                verified: Some(false),
            }),
            RegistryInstallOutcome::Installed {
                name: installed_name,
                version,
                path,
            } => {
                self.source
                    .persist_registry_install(&installed_name, &version, &metadata)
                    .await?;
                let _ = self
                    .source
                    .compile_registry_install(&installed_name, &path)
                    .await;

                Ok(SkillMutationReport {
                    status: "ok".to_string(),
                    action: "install".to_string(),
                    skill_name: installed_name.clone(),
                    message: format!(
                        "Installed marketplace skill '{}' v{}",
                        installed_name, version
                    ),
                    skill: Some(self.source.load_skill_detail(&installed_name).await?),
                    verified: Some(false),
                })
            }
        }
    }

    pub async fn update(&self, name: &str) -> Result<SkillMutationReport<S::Detail>> {
        if !self.source.skill_exists(name).await? {
            return Ok(SkillMutationReport {
                status: "noop".to_string(),
                action: "update".to_string(),
                skill_name: name.to_string(),
                message: format!("Skill '{}' is not installed", name),
                skill: None,
                verified: None,
            });
        }

        match self.source.registry_update(name).await? {
            RegistryUpdateOutcome::UpToDate => Ok(SkillMutationReport {
                status: "noop".to_string(),
                action: "update".to_string(),
                skill_name: name.to_string(),
                message: format!("Skill '{}' is already up to date", name),
                skill: Some(self.source.load_skill_detail(name).await?),
                verified: None,
            }),
            RegistryUpdateOutcome::Updated { from, to } => {
                let metadata = self.source.registry_skill_metadata(name).await?;
                self.source
                    .enforce_registry_skill_policy(name, &metadata)
                    .await?;
                self.source
                    .persist_registry_update(name, &from, &to, &metadata)
                    .await?;
                let _ = self.source.compile_updated_skill(name).await;

                Ok(SkillMutationReport {
                    status: "ok".to_string(),
                    action: "update".to_string(),
                    skill_name: name.to_string(),
                    message: format!("Updated skill '{}' from v{} to v{}", name, from, to),
                    skill: Some(self.source.load_skill_detail(name).await?),
                    verified: Some(false),
                })
            }
        }
    }

    pub async fn uninstall(&self, name: &str) -> Result<SkillMutationReport<S::Detail>> {
        if !self.source.skill_exists(name).await? {
            return Ok(SkillMutationReport {
                status: "noop".to_string(),
                action: "uninstall".to_string(),
                skill_name: name.to_string(),
                message: format!("Skill '{}' is not installed", name),
                skill: None,
                verified: None,
            });
        }

        let _ = self.source.registry_uninstall_best_effort(name).await;
        self.source.remove_installed_skill(name).await?;

        Ok(SkillMutationReport {
            status: "ok".to_string(),
            action: "uninstall".to_string(),
            skill_name: name.to_string(),
            message: format!("Uninstalled skill '{}'", name),
            skill: None,
            verified: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openrustclaw_core::error::Error;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct TestSource {
        installed: Arc<Mutex<Vec<String>>>,
        events: Arc<Mutex<Vec<String>>>,
        workspace_candidate: Option<WorkspaceSkillCandidate>,
        registry_install: Option<RegistryInstallOutcome>,
        registry_update: Option<RegistryUpdateOutcome>,
        registry_metadata: Option<RegistrySkillMetadata>,
    }

    #[async_trait]
    impl SkillRegistryMutationSource for TestSource {
        type Detail = String;

        async fn skill_exists(&self, name: &str) -> Result<bool> {
            Ok(self
                .installed
                .lock()
                .unwrap()
                .iter()
                .any(|entry| entry == name))
        }

        async fn load_skill_detail(&self, name: &str) -> Result<Self::Detail> {
            Ok(format!("detail:{name}"))
        }

        async fn ensure_skills_dir(&self) -> Result<()> {
            self.events.lock().unwrap().push("ensure_dir".to_string());
            Ok(())
        }

        async fn workspace_skill_candidate(
            &self,
            _name: &str,
        ) -> Result<Option<WorkspaceSkillCandidate>> {
            Ok(self.workspace_candidate.clone())
        }

        async fn persist_workspace_install(
            &self,
            candidate: &WorkspaceSkillCandidate,
        ) -> Result<()> {
            self.installed.lock().unwrap().push(candidate.name.clone());
            self.events
                .lock()
                .unwrap()
                .push(format!("persist_workspace:{}", candidate.name));
            Ok(())
        }

        async fn compile_workspace_skill(&self, candidate: &WorkspaceSkillCandidate) -> Result<()> {
            self.events
                .lock()
                .unwrap()
                .push(format!("compile_workspace:{}", candidate.name));
            Ok(())
        }

        async fn registry_skill_metadata(&self, _name: &str) -> Result<RegistrySkillMetadata> {
            self.registry_metadata
                .clone()
                .ok_or_else(|| Error::Internal("missing registry metadata".to_string()))
        }

        async fn enforce_registry_skill_policy(
            &self,
            name: &str,
            _metadata: &RegistrySkillMetadata,
        ) -> Result<()> {
            self.events.lock().unwrap().push(format!("policy:{name}"));
            Ok(())
        }

        async fn registry_install(&self, _name: &str) -> Result<RegistryInstallOutcome> {
            self.registry_install
                .clone()
                .ok_or_else(|| Error::Internal("missing registry install outcome".to_string()))
        }

        async fn persist_registry_install(
            &self,
            name: &str,
            version: &str,
            _metadata: &RegistrySkillMetadata,
        ) -> Result<()> {
            self.installed.lock().unwrap().push(name.to_string());
            self.events
                .lock()
                .unwrap()
                .push(format!("persist_registry_install:{name}:{version}"));
            Ok(())
        }

        async fn compile_registry_install(&self, name: &str, _path: &str) -> Result<()> {
            self.events
                .lock()
                .unwrap()
                .push(format!("compile_registry_install:{name}"));
            Ok(())
        }

        async fn registry_update(&self, _name: &str) -> Result<RegistryUpdateOutcome> {
            self.registry_update
                .clone()
                .ok_or_else(|| Error::Internal("missing registry update outcome".to_string()))
        }

        async fn persist_registry_update(
            &self,
            name: &str,
            from: &str,
            to: &str,
            _metadata: &RegistrySkillMetadata,
        ) -> Result<()> {
            self.events
                .lock()
                .unwrap()
                .push(format!("persist_registry_update:{name}:{from}:{to}"));
            Ok(())
        }

        async fn compile_updated_skill(&self, name: &str) -> Result<()> {
            self.events
                .lock()
                .unwrap()
                .push(format!("compile_updated:{name}"));
            Ok(())
        }

        async fn registry_uninstall_best_effort(&self, name: &str) -> Result<()> {
            self.events
                .lock()
                .unwrap()
                .push(format!("registry_uninstall:{name}"));
            Ok(())
        }

        async fn remove_installed_skill(&self, name: &str) -> Result<()> {
            self.installed.lock().unwrap().retain(|entry| entry != name);
            self.events
                .lock()
                .unwrap()
                .push(format!("remove_installed:{name}"));
            Ok(())
        }
    }

    #[tokio::test]
    async fn skill_registry_mutation_installs_workspace_skill() -> Result<()> {
        let source = TestSource {
            workspace_candidate: Some(WorkspaceSkillCandidate {
                name: "demo".to_string(),
                path: "skills/demo/SKILL.md".to_string(),
                description: Some("Demo".to_string()),
                version: None,
                signature: None,
                capabilities: vec!["file_read".to_string()],
                schema: None,
            }),
            ..Default::default()
        };

        let report = SkillRegistryMutationService::new(source.clone())
            .install("demo")
            .await?;

        assert_eq!(report.status, "ok");
        assert_eq!(report.action, "install");
        assert_eq!(report.skill_name, "demo");
        assert_eq!(report.skill, Some("detail:demo".to_string()));
        assert_eq!(report.verified, Some(true));
        assert_eq!(
            source.events.lock().unwrap().clone(),
            vec![
                "ensure_dir".to_string(),
                "persist_workspace:demo".to_string(),
                "compile_workspace:demo".to_string()
            ]
        );
        Ok(())
    }

    #[tokio::test]
    async fn skill_registry_mutation_updates_and_uninstalls_marketplace_skill() -> Result<()> {
        let source = TestSource {
            installed: Arc::new(Mutex::new(vec!["demo".to_string()])),
            registry_update: Some(RegistryUpdateOutcome::Updated {
                from: "1.0.0".to_string(),
                to: "1.1.0".to_string(),
            }),
            registry_metadata: Some(RegistrySkillMetadata {
                description: "Demo".to_string(),
                capabilities: vec!["network_access".to_string()],
                signature: Some("deadbeef".to_string()),
            }),
            ..Default::default()
        };

        let service = SkillRegistryMutationService::new(source.clone());
        let update = service.update("demo").await?;
        assert_eq!(update.status, "ok");
        assert_eq!(update.verified, Some(false));

        let uninstall = service.uninstall("demo").await?;
        assert_eq!(uninstall.status, "ok");
        assert_eq!(uninstall.action, "uninstall");
        assert!(!source.skill_exists("demo").await?);
        Ok(())
    }
}

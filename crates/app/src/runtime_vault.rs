use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RuntimeVaultState {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub entries: BTreeMap<String, String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeVaultMutationReport {
    pub status: String,
    pub key: String,
    pub count: usize,
    pub updated_at: String,
}

pub struct RuntimeVaultService;

impl Default for RuntimeVaultService {
    fn default() -> Self {
        Self
    }
}

impl RuntimeVaultService {
    pub fn new() -> Self {
        Self
    }

    pub fn set_secret(
        &self,
        mut vault: RuntimeVaultState,
        key: &str,
        value: &str,
        updated_at: String,
    ) -> (RuntimeVaultState, RuntimeVaultMutationReport) {
        vault.version = 1;
        vault.entries.insert(key.to_string(), value.to_string());
        vault.updated_at = Some(updated_at.clone());
        let report = RuntimeVaultMutationReport {
            status: "ok".to_string(),
            key: key.to_string(),
            count: vault.entries.len(),
            updated_at,
        };
        (vault, report)
    }

    pub fn delete_secret(
        &self,
        mut vault: RuntimeVaultState,
        key: &str,
        updated_at: String,
    ) -> (RuntimeVaultState, RuntimeVaultMutationReport) {
        vault.entries.remove(key);
        vault.updated_at = Some(updated_at.clone());
        let report = RuntimeVaultMutationReport {
            status: "ok".to_string(),
            key: key.to_string(),
            count: vault.entries.len(),
            updated_at,
        };
        (vault, report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_vault_set_secret_initializes_version_and_updates_count() {
        let (vault, report) = RuntimeVaultService::new().set_secret(
            RuntimeVaultState::default(),
            "OPENAI_API_KEY",
            "secret",
            "2026-03-28T12:00:00Z".to_string(),
        );

        assert_eq!(vault.version, 1);
        assert_eq!(
            vault.entries.get("OPENAI_API_KEY"),
            Some(&"secret".to_string())
        );
        assert_eq!(vault.updated_at.as_deref(), Some("2026-03-28T12:00:00Z"));
        assert_eq!(report.status, "ok");
        assert_eq!(report.count, 1);
    }

    #[test]
    fn runtime_vault_delete_secret_removes_entry_and_keeps_report_stable() {
        let mut entries = BTreeMap::new();
        entries.insert("OPENAI_API_KEY".to_string(), "secret".to_string());
        entries.insert("ANTHROPIC_API_KEY".to_string(), "other".to_string());

        let (vault, report) = RuntimeVaultService::new().delete_secret(
            RuntimeVaultState {
                version: 1,
                entries,
                updated_at: None,
            },
            "OPENAI_API_KEY",
            "2026-03-28T13:00:00Z".to_string(),
        );

        assert!(!vault.entries.contains_key("OPENAI_API_KEY"));
        assert_eq!(vault.entries.len(), 1);
        assert_eq!(vault.updated_at.as_deref(), Some("2026-03-28T13:00:00Z"));
        assert_eq!(report.status, "ok");
        assert_eq!(report.key, "OPENAI_API_KEY");
        assert_eq!(report.count, 1);
    }
}

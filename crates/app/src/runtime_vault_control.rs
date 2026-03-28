use openrustclaw_core::error::Result;
use serde::Serialize;

pub trait RuntimeVaultControlSource {
    fn vault_path(&self) -> String;
    fn vault_present(&self) -> bool;
    fn list_vault_keys(&self) -> Result<Vec<String>>;
    fn set_vault_secret(&self, key: &str, value: &str) -> Result<()>;
    fn delete_vault_secret(&self, key: &str) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuntimeVaultStatusResponse {
    pub path: String,
    pub present: bool,
    pub entries: Vec<String>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuntimeVaultMutationResponse {
    pub status: String,
    pub key: String,
}

pub struct RuntimeVaultControlService<S> {
    source: S,
}

impl<S> RuntimeVaultControlService<S> {
    pub fn new(source: S) -> Self {
        Self { source }
    }
}

impl<S> RuntimeVaultControlService<S>
where
    S: RuntimeVaultControlSource,
{
    pub fn status(&self) -> Result<RuntimeVaultStatusResponse> {
        let entries = self.source.list_vault_keys()?;
        Ok(RuntimeVaultStatusResponse {
            path: self.source.vault_path(),
            present: self.source.vault_present(),
            count: entries.len(),
            entries,
        })
    }

    pub fn set_secret(&self, key: &str, value: &str) -> Result<RuntimeVaultMutationResponse> {
        self.source.set_vault_secret(key, value)?;
        Ok(RuntimeVaultMutationResponse {
            status: "ok".to_string(),
            key: key.to_string(),
        })
    }

    pub fn delete_secret(&self, key: &str) -> Result<RuntimeVaultMutationResponse> {
        self.source.delete_vault_secret(key)?;
        Ok(RuntimeVaultMutationResponse {
            status: "ok".to_string(),
            key: key.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openrustclaw_core::error::Error;
    use std::cell::RefCell;

    #[derive(Default)]
    struct MockRuntimeVaultControlSource {
        path: String,
        present: bool,
        entries: RefCell<Vec<String>>,
    }

    impl RuntimeVaultControlSource for MockRuntimeVaultControlSource {
        fn vault_path(&self) -> String {
            self.path.clone()
        }

        fn vault_present(&self) -> bool {
            self.present
        }

        fn list_vault_keys(&self) -> Result<Vec<String>> {
            Ok(self.entries.borrow().clone())
        }

        fn set_vault_secret(&self, key: &str, _value: &str) -> Result<()> {
            let mut entries = self.entries.borrow_mut();
            if !entries.iter().any(|entry| entry == key) {
                entries.push(key.to_string());
                entries.sort();
            }
            Ok(())
        }

        fn delete_vault_secret(&self, key: &str) -> Result<()> {
            let mut entries = self.entries.borrow_mut();
            entries.retain(|entry| entry != key);
            Ok(())
        }
    }

    #[test]
    fn runtime_vault_control_status_reports_path_and_entry_count() -> Result<()> {
        let service = RuntimeVaultControlService::new(MockRuntimeVaultControlSource {
            path: ".claw/control/runtime-vault.json".to_string(),
            present: true,
            entries: RefCell::new(vec![
                "ANTHROPIC_API_KEY".to_string(),
                "OPENAI_API_KEY".to_string(),
            ]),
        });

        let response = service.status()?;
        assert_eq!(response.path, ".claw/control/runtime-vault.json");
        assert!(response.present);
        assert_eq!(response.count, 2);
        Ok(())
    }

    #[test]
    fn runtime_vault_control_set_and_delete_return_stable_reports() -> Result<()> {
        let service = RuntimeVaultControlService::new(MockRuntimeVaultControlSource::default());

        let set = service.set_secret("OPENAI_API_KEY", "secret")?;
        assert_eq!(set.status, "ok");
        assert_eq!(set.key, "OPENAI_API_KEY");

        let delete = service.delete_secret("OPENAI_API_KEY")?;
        assert_eq!(delete.status, "ok");
        assert_eq!(delete.key, "OPENAI_API_KEY");
        Ok(())
    }

    #[test]
    fn runtime_vault_control_propagates_source_errors() {
        struct ErrorSource;

        impl RuntimeVaultControlSource for ErrorSource {
            fn vault_path(&self) -> String {
                "path".to_string()
            }

            fn vault_present(&self) -> bool {
                false
            }

            fn list_vault_keys(&self) -> Result<Vec<String>> {
                Err(Error::Internal("boom".to_string()))
            }

            fn set_vault_secret(&self, _key: &str, _value: &str) -> Result<()> {
                Err(Error::Internal("boom".to_string()))
            }

            fn delete_vault_secret(&self, _key: &str) -> Result<()> {
                Err(Error::Internal("boom".to_string()))
            }
        }

        let service = RuntimeVaultControlService::new(ErrorSource);
        assert!(service.status().is_err());
        assert!(service.set_secret("A", "B").is_err());
        assert!(service.delete_secret("A").is_err());
    }
}

//! WASM sandbox for untrusted skills.
//!
//! Uses wasmtime to execute skill code in an isolated environment.
//! Capabilities are enforced: a skill without NetworkAccess cannot make HTTP calls.

use std::collections::HashSet;

use openrustclaw_core::error::{Error, Result, ToolError};
use openrustclaw_core::types::SkillCapability;

/// WASM sandbox configuration.
pub struct SandboxConfig {
    /// Maximum memory in bytes (default: 64MB).
    pub max_memory_bytes: usize,
    /// Maximum execution time in milliseconds.
    pub max_execution_ms: u64,
    /// Granted capabilities.
    pub capabilities: HashSet<SkillCapability>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 64 * 1024 * 1024,
            max_execution_ms: 30_000,
            capabilities: HashSet::new(),
        }
    }
}

/// WASM sandbox for executing untrusted skill code.
pub struct WasmSandbox {
    config: SandboxConfig,
}

impl WasmSandbox {
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }

    /// Check if a capability is granted.
    pub fn has_capability(&self, cap: &SkillCapability) -> bool {
        self.capabilities().contains(cap)
    }

    /// Verify that all required capabilities are granted.
    pub fn check_capabilities(&self, required: &[SkillCapability]) -> Result<()> {
        for cap in required {
            if !self.has_capability(cap) {
                return Err(Error::Tool(ToolError::CapabilityDenied {
                    tool: "wasm_sandbox".to_string(),
                    capability: format!("{:?}", cap),
                }));
            }
        }
        Ok(())
    }

    /// Get granted capabilities.
    pub fn capabilities(&self) -> &HashSet<SkillCapability> {
        &self.config.capabilities
    }

    /// Execute WASM bytes in the sandbox.
    ///
    /// TODO: Implement actual wasmtime execution.
    pub async fn execute(
        &self,
        _wasm_bytes: &[u8],
        _input: serde_json::Value,
    ) -> Result<serde_json::Value> {
        // Placeholder - actual wasmtime integration deferred
        Err(Error::Internal(
            "WASM execution not yet implemented".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SandboxConfig tests ─────────────────────────────────────────────

    #[test]
    fn test_sandbox_config_default_memory() {
        let config = SandboxConfig::default();
        assert_eq!(config.max_memory_bytes, 64 * 1024 * 1024); // 64 MB
    }

    #[test]
    fn test_sandbox_config_default_timeout() {
        let config = SandboxConfig::default();
        assert_eq!(config.max_execution_ms, 30_000); // 30 seconds
    }

    #[test]
    fn test_sandbox_config_default_no_capabilities() {
        let config = SandboxConfig::default();
        assert!(config.capabilities.is_empty());
    }

    #[test]
    fn test_sandbox_config_custom_memory() {
        let config = SandboxConfig {
            max_memory_bytes: 128 * 1024 * 1024,
            ..Default::default()
        };
        assert_eq!(config.max_memory_bytes, 128 * 1024 * 1024);
    }

    #[test]
    fn test_sandbox_config_custom_timeout() {
        let config = SandboxConfig {
            max_execution_ms: 5_000,
            ..Default::default()
        };
        assert_eq!(config.max_execution_ms, 5_000);
    }

    // ── WasmSandbox capability tests ────────────────────────────────────

    #[test]
    fn test_sandbox_no_capabilities_by_default() {
        let sandbox = WasmSandbox::new(SandboxConfig::default());
        assert!(sandbox.capabilities().is_empty());
        assert!(!sandbox.has_capability(&SkillCapability::FileRead));
        assert!(!sandbox.has_capability(&SkillCapability::NetworkAccess));
    }

    #[test]
    fn test_sandbox_has_granted_capability() {
        let mut caps = HashSet::new();
        caps.insert(SkillCapability::FileRead);
        caps.insert(SkillCapability::NetworkAccess);

        let sandbox = WasmSandbox::new(SandboxConfig {
            capabilities: caps,
            ..Default::default()
        });

        assert!(sandbox.has_capability(&SkillCapability::FileRead));
        assert!(sandbox.has_capability(&SkillCapability::NetworkAccess));
        assert!(!sandbox.has_capability(&SkillCapability::ShellExec));
        assert!(!sandbox.has_capability(&SkillCapability::DatabaseAccess));
    }

    #[test]
    fn test_sandbox_check_capabilities_all_granted() {
        let mut caps = HashSet::new();
        caps.insert(SkillCapability::FileRead);
        caps.insert(SkillCapability::FileWrite);
        caps.insert(SkillCapability::NetworkAccess);

        let sandbox = WasmSandbox::new(SandboxConfig {
            capabilities: caps,
            ..Default::default()
        });

        let result = sandbox.check_capabilities(&[
            SkillCapability::FileRead,
            SkillCapability::FileWrite,
        ]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sandbox_check_capabilities_denied() {
        let sandbox = WasmSandbox::new(SandboxConfig::default());

        let result = sandbox.check_capabilities(&[SkillCapability::ShellExec]);
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(err_str.contains("capability"), "Error should mention capability: {}", err_str);
    }

    #[test]
    fn test_sandbox_check_capabilities_partial_denied() {
        let mut caps = HashSet::new();
        caps.insert(SkillCapability::FileRead);

        let sandbox = WasmSandbox::new(SandboxConfig {
            capabilities: caps,
            ..Default::default()
        });

        // FileRead is granted, but ShellExec is not
        let result = sandbox.check_capabilities(&[
            SkillCapability::FileRead,
            SkillCapability::ShellExec,
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn test_sandbox_check_empty_requirements_always_ok() {
        let sandbox = WasmSandbox::new(SandboxConfig::default());
        let result = sandbox.check_capabilities(&[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sandbox_all_capability_variants() {
        let all_caps: HashSet<SkillCapability> = [
            SkillCapability::FileRead,
            SkillCapability::FileWrite,
            SkillCapability::NetworkAccess,
            SkillCapability::ShellExec,
            SkillCapability::DatabaseAccess,
            SkillCapability::MemoryWrite,
        ]
        .into_iter()
        .collect();

        let sandbox = WasmSandbox::new(SandboxConfig {
            capabilities: all_caps.clone(),
            ..Default::default()
        });

        assert_eq!(sandbox.capabilities().len(), 6);
        for cap in &all_caps {
            assert!(sandbox.has_capability(cap));
        }
    }

    #[tokio::test]
    async fn test_sandbox_execute_returns_not_implemented() {
        let sandbox = WasmSandbox::new(SandboxConfig::default());
        let result = sandbox
            .execute(&[], serde_json::json!({}))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not yet implemented"),
            "Expected 'not yet implemented', got: {}",
            err
        );
    }

    #[test]
    fn test_sandbox_capabilities_returns_reference() {
        let mut caps = HashSet::new();
        caps.insert(SkillCapability::MemoryWrite);
        caps.insert(SkillCapability::DatabaseAccess);

        let sandbox = WasmSandbox::new(SandboxConfig {
            capabilities: caps,
            ..Default::default()
        });

        let returned_caps = sandbox.capabilities();
        assert_eq!(returned_caps.len(), 2);
        assert!(returned_caps.contains(&SkillCapability::MemoryWrite));
        assert!(returned_caps.contains(&SkillCapability::DatabaseAccess));
    }
}

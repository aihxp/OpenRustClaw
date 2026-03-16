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

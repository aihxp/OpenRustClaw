//! WASM sandbox for untrusted skills.
//!
//! Uses wasmtime to execute skill code in an isolated environment.
//! Capabilities are enforced: a skill without NetworkAccess cannot make HTTP calls.

use std::collections::HashSet;
use std::time::Duration;

use openrustclaw_core::error::{Error, Result, ToolError};
use openrustclaw_core::types::SkillCapability;
use wasmtime::{
    Config, Engine, Instance, Memory, Module, Store, StoreLimits, StoreLimitsBuilder, TypedFunc,
};

use crate::{declared_sensitive_capability_names, parse_capability_names};

/// WASM sandbox configuration.
#[derive(Debug)]
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

impl SandboxConfig {
    /// Build a sandbox config that grants exactly the declared capability names.
    pub fn with_declared_capabilities(declared: &[String]) -> Result<Self> {
        Ok(Self {
            capabilities: parse_capability_names(declared)?,
            ..Self::default()
        })
    }

    /// Validate whether a declared capability set is allowed for the current verification state.
    pub fn validate_declared_capability_policy(declared: &[String], verified: bool) -> Result<()> {
        let sensitive = declared_sensitive_capability_names(declared)?;
        if !verified && !sensitive.is_empty() {
            return Err(Error::Tool(ToolError::CapabilityDenied {
                tool: "wasm_sandbox".to_string(),
                capability: sensitive.join(","),
            }));
        }
        Ok(())
    }

    /// Whether a declared capability set requires verification before execution.
    pub fn requires_verified_declared_capabilities(declared: &[String]) -> Result<bool> {
        Ok(!declared_sensitive_capability_names(declared)?.is_empty())
    }
}

/// WASM sandbox for executing untrusted skill code.
pub struct WasmSandbox {
    config: SandboxConfig,
}

struct SandboxStore {
    limits: StoreLimits,
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
    pub async fn execute(
        &self,
        wasm_bytes: &[u8],
        input: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let wasm = wasm_bytes.to_vec();
        let config = SandboxConfig {
            max_memory_bytes: self.config.max_memory_bytes,
            max_execution_ms: self.config.max_execution_ms,
            capabilities: self.config.capabilities.clone(),
        };

        tokio::task::spawn_blocking(move || Self::execute_blocking(config, &wasm, input))
            .await
            .map_err(|e| Error::Internal(format!("WASM sandbox task failed: {}", e)))?
    }

    /// Execute only if the supplied capabilities are granted by the sandbox.
    pub async fn execute_with_capabilities(
        &self,
        wasm_bytes: &[u8],
        required: &[SkillCapability],
        input: serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.check_capabilities(required)?;
        self.execute(wasm_bytes, input).await
    }

    /// Execute only if the declared capability names are granted by the sandbox.
    pub async fn execute_with_declared_capabilities(
        &self,
        wasm_bytes: &[u8],
        declared: &[String],
        input: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let parsed = parse_capability_names(declared)?;
        let required: Vec<SkillCapability> = parsed.into_iter().collect();
        self.execute_with_capabilities(wasm_bytes, &required, input)
            .await
    }

    /// Execute only if declared capabilities are granted and the skill is verified when required.
    pub async fn execute_with_verified_declared_capabilities(
        &self,
        wasm_bytes: &[u8],
        declared: &[String],
        verified: bool,
        input: serde_json::Value,
    ) -> Result<serde_json::Value> {
        SandboxConfig::validate_declared_capability_policy(declared, verified)?;
        self.execute_with_declared_capabilities(wasm_bytes, declared, input)
            .await
    }

    fn execute_blocking(
        config: SandboxConfig,
        wasm_bytes: &[u8],
        input: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let mut engine_config = Config::new();
        engine_config.epoch_interruption(true);
        let engine = Engine::new(&engine_config)
            .map_err(|e| Error::Internal(format!("Failed to initialize wasmtime engine: {}", e)))?;

        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| Error::Internal(format!("Failed to compile wasm module: {}", e)))?;

        if module.imports().next().is_some() {
            return Err(Error::Tool(ToolError::SandboxViolation(
                "WASM sandbox does not allow module imports".to_string(),
            )));
        }

        let store_limits = StoreLimitsBuilder::new()
            .memory_size(config.max_memory_bytes)
            .instances(1)
            .tables(0)
            .memories(1)
            .build();
        let mut store = Store::new(
            &engine,
            SandboxStore {
                limits: store_limits,
            },
        );
        store.limiter(|state| &mut state.limits);
        store.set_epoch_deadline(1);

        let timer_engine = engine.clone();
        let timeout = config.max_execution_ms;
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(timeout));
            timer_engine.increment_epoch();
        });

        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|e| Error::Internal(format!("Failed to instantiate wasm module: {}", e)))?;

        let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| {
            Error::Tool(ToolError::SandboxViolation(
                "WASM module must export memory".to_string(),
            ))
        })?;
        let alloc: TypedFunc<i32, i32> =
            instance.get_typed_func(&mut store, "alloc").map_err(|e| {
                Error::Tool(ToolError::SandboxViolation(format!(
                    "WASM module must export alloc(i32) -> i32: {}",
                    e
                )))
            })?;
        let run: TypedFunc<(i32, i32), i64> =
            instance.get_typed_func(&mut store, "run").map_err(|e| {
                Error::Tool(ToolError::SandboxViolation(format!(
                    "WASM module must export run(i32, i32) -> i64: {}",
                    e
                )))
            })?;

        let input_bytes = serde_json::to_vec(&input)
            .map_err(|e| Error::Internal(format!("Failed to serialize sandbox input: {}", e)))?;
        if input_bytes.len() > i32::MAX as usize {
            return Err(Error::Tool(ToolError::InputValidation {
                tool: "wasm_sandbox".to_string(),
                message: "Input payload is too large for wasm ABI".to_string(),
            }));
        }

        let input_ptr = alloc
            .call(&mut store, input_bytes.len() as i32)
            .map_err(|e| map_wasm_trap("alloc", e, config.max_execution_ms))?;
        write_memory(&mut store, &memory, input_ptr, &input_bytes)?;

        let packed =
            run.call(&mut store, (input_ptr, input_bytes.len() as i32))
                .map_err(|e| map_wasm_trap("run", e, config.max_execution_ms))? as u64;

        let output_ptr = (packed >> 32) as usize;
        let output_len = (packed & 0xFFFF_FFFF) as usize;
        let output_bytes = read_memory(&store, &memory, output_ptr, output_len)?;
        serde_json::from_slice(&output_bytes).map_err(|e| {
            Error::Tool(ToolError::ExecutionFailed {
                tool: "wasm_sandbox".to_string(),
                message: format!("WASM output is not valid JSON: {}", e),
            })
        })
    }
}

fn write_memory(
    store: &mut Store<SandboxStore>,
    memory: &Memory,
    ptr: i32,
    bytes: &[u8],
) -> Result<()> {
    if ptr < 0 {
        return Err(Error::Tool(ToolError::SandboxViolation(
            "WASM alloc returned a negative pointer".to_string(),
        )));
    }

    memory.write(store, ptr as usize, bytes).map_err(|e| {
        Error::Tool(ToolError::SandboxViolation(format!(
            "Failed to write input into wasm memory: {}",
            e
        )))
    })
}

fn read_memory(
    store: &Store<SandboxStore>,
    memory: &Memory,
    ptr: usize,
    len: usize,
) -> Result<Vec<u8>> {
    let mut buffer = vec![0u8; len];
    memory.read(store, ptr, &mut buffer).map_err(|e| {
        Error::Tool(ToolError::SandboxViolation(format!(
            "Failed to read output from wasm memory: {}",
            e
        )))
    })?;
    Ok(buffer)
}

fn map_wasm_trap(stage: &str, error: wasmtime::Error, timeout_ms: u64) -> Error {
    if matches!(
        error.downcast_ref::<wasmtime::Trap>(),
        Some(wasmtime::Trap::Interrupt)
    ) {
        Error::Tool(ToolError::Timeout {
            tool: "wasm_sandbox".to_string(),
            timeout_ms,
        })
    } else {
        Error::Tool(ToolError::ExecutionFailed {
            tool: "wasm_sandbox".to_string(),
            message: format!("WASM {} failed: {}", stage, error),
        })
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
    fn test_sandbox_config_with_declared_capabilities() {
        let config = SandboxConfig::with_declared_capabilities(&[
            "network_access".to_string(),
            "file_read".to_string(),
        ])
        .unwrap();
        assert!(
            config
                .capabilities
                .contains(&SkillCapability::NetworkAccess)
        );
        assert!(config.capabilities.contains(&SkillCapability::FileRead));
    }

    #[test]
    fn test_sandbox_config_with_declared_capabilities_rejects_unknown_values() {
        let error = SandboxConfig::with_declared_capabilities(&["launch_missiles".to_string()])
            .unwrap_err();
        assert!(error.to_string().contains("Unknown skill capability"));
    }

    #[test]
    fn test_sandbox_config_rejects_unverified_sensitive_declared_capabilities() {
        let error =
            SandboxConfig::validate_declared_capability_policy(&["shell_exec".to_string()], false)
                .unwrap_err();
        assert!(error.to_string().contains("shell_exec"));
    }

    #[test]
    fn test_sandbox_config_allows_unverified_nonsensitive_declared_capabilities() {
        SandboxConfig::validate_declared_capability_policy(&["file_read".to_string()], false)
            .unwrap();
    }

    #[test]
    fn test_sandbox_config_allows_verified_sensitive_declared_capabilities() {
        SandboxConfig::validate_declared_capability_policy(
            &["network_access".to_string(), "file_read".to_string()],
            true,
        )
        .unwrap();
    }

    #[test]
    fn test_sandbox_config_reports_when_declared_capabilities_require_verification() {
        assert!(
            SandboxConfig::requires_verified_declared_capabilities(
                &["database_access".to_string()]
            )
            .unwrap()
        );
        assert!(
            !SandboxConfig::requires_verified_declared_capabilities(&["file_read".to_string()])
                .unwrap()
        );
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

        let result =
            sandbox.check_capabilities(&[SkillCapability::FileRead, SkillCapability::FileWrite]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sandbox_check_capabilities_denied() {
        let sandbox = WasmSandbox::new(SandboxConfig::default());

        let result = sandbox.check_capabilities(&[SkillCapability::ShellExec]);
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(
            err_str.contains("capability"),
            "Error should mention capability: {}",
            err_str
        );
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
        let result =
            sandbox.check_capabilities(&[SkillCapability::FileRead, SkillCapability::ShellExec]);
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
    async fn test_sandbox_execute_round_trips_json_input() {
        let sandbox = WasmSandbox::new(SandboxConfig::default());
        let module = br#"
            (module
              (memory (export "memory") 1 1)
              (global $heap (mut i32) (i32.const 4096))
              (func (export "alloc") (param $len i32) (result i32)
                (local $ptr i32)
                global.get $heap
                local.set $ptr
                global.get $heap
                local.get $len
                i32.add
                global.set $heap
                local.get $ptr)
              (func (export "run") (param $ptr i32) (param $len i32) (result i64)
                local.get $ptr
                i64.extend_i32_u
                i64.const 32
                i64.shl
                local.get $len
                i64.extend_i32_u
                i64.or))
        "#;

        let input = serde_json::json!({"echo": "hello", "count": 2});
        let result = sandbox.execute(module, input.clone()).await.unwrap();
        assert_eq!(result, input);
    }

    #[tokio::test]
    async fn test_sandbox_execute_with_declared_capabilities_rejects_missing_capability() {
        let sandbox = WasmSandbox::new(SandboxConfig::default());
        let module = br#"
            (module
              (memory (export "memory") 1 1)
              (func (export "alloc") (param i32) (result i32) i32.const 0)
              (func (export "run") (param i32 i32) (result i64) i64.const 0))
        "#;

        let result = sandbox
            .execute_with_declared_capabilities(
                module,
                &["network_access".to_string()],
                serde_json::json!({}),
            )
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("capability"));
    }

    #[tokio::test]
    async fn test_sandbox_execute_with_verified_declared_capabilities_rejects_unverified_sensitive()
    {
        let sandbox = WasmSandbox::new(SandboxConfig::default());
        let module = br#"
            (module
              (memory (export "memory") 1 1)
              (func (export "alloc") (param i32) (result i32) i32.const 0)
              (func (export "run") (param i32 i32) (result i64) i64.const 0))
        "#;

        let result = sandbox
            .execute_with_verified_declared_capabilities(
                module,
                &["shell_exec".to_string()],
                false,
                serde_json::json!({}),
            )
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("shell_exec"));
    }

    #[tokio::test]
    async fn test_sandbox_rejects_imports() {
        let sandbox = WasmSandbox::new(SandboxConfig::default());
        let module = br#"
            (module
              (import "env" "noop" (func $noop))
              (memory (export "memory") 1 1)
              (func (export "alloc") (param i32) (result i32) i32.const 0)
              (func (export "run") (param i32 i32) (result i64)
                call $noop
                i64.const 0))
        "#;

        let result = sandbox.execute(module, serde_json::json!({})).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("does not allow module imports")
        );
    }

    #[tokio::test]
    async fn test_sandbox_times_out_infinite_loop() {
        let sandbox = WasmSandbox::new(SandboxConfig {
            max_execution_ms: 25,
            ..Default::default()
        });
        let module = br#"
            (module
              (memory (export "memory") 1 1)
              (func (export "alloc") (param i32) (result i32) i32.const 0)
              (func (export "run") (param i32 i32) (result i64)
                (loop $spin
                  br $spin)
                i64.const 0))
        "#;

        let result = sandbox.execute(module, serde_json::json!({})).await;
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("exceeded 25ms"));
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

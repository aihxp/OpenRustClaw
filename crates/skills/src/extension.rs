//! Rust-native extension manifest generation over compiled skills.

use std::fs;
use std::path::{Path, PathBuf};

use openrustclaw_core::error::{Error, Result};
use openrustclaw_core::types::SkillSource;
use serde::{Deserialize, Serialize};

use crate::compiler::{CompiledSkillArtifact, CompiledSkillStatus};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionImplementationStatus {
    Available,
    Declared,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionTriggerBinding {
    pub name: String,
    pub kind: String,
    pub status: ExtensionImplementationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionToolBinding {
    pub name: String,
    pub kind: String,
    pub status: ExtensionImplementationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionComponentBinding {
    pub name: String,
    pub kind: String,
    pub status: ExtensionImplementationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub source: SkillSource,
    pub verified: bool,
    pub compiled_status: CompiledSkillStatus,
    pub verification_policy: String,
    pub local_path: String,
    pub artifact_root: String,
    pub capabilities: Vec<String>,
    pub allowed_tools: Vec<String>,
    pub runtime_modes: Vec<String>,
    pub triggers: Vec<ExtensionTriggerBinding>,
    pub tools: Vec<ExtensionToolBinding>,
    pub components: Vec<ExtensionComponentBinding>,
    pub notes: Vec<String>,
    pub compiled_at: chrono::DateTime<chrono::Utc>,
}

pub fn build_extension_manifest(
    artifact: &CompiledSkillArtifact,
    output_root: &Path,
) -> ExtensionManifest {
    let sanitized_name = sanitize_name(&artifact.manifest.name);
    let artifact_root = output_root.join(&sanitized_name);
    let blocked = matches!(artifact.manifest.status, CompiledSkillStatus::Blocked);
    let available_status = if blocked {
        ExtensionImplementationStatus::Blocked
    } else {
        ExtensionImplementationStatus::Available
    };
    let declared_status = if blocked {
        ExtensionImplementationStatus::Blocked
    } else {
        ExtensionImplementationStatus::Declared
    };

    let mut runtime_modes = vec!["compiled_metadata".to_string()];
    let mut triggers = vec![ExtensionTriggerBinding {
        name: "openrustclaw.skills.invoke".to_string(),
        kind: "command_hook".to_string(),
        status: available_status,
        command_preview: Some(artifact.cli_schema.usage.clone()),
        args_hint: artifact.cli_schema.args_hint.clone(),
        source_path: None,
    }];
    let mut tools = artifact
        .mcp_schema
        .tool_names
        .iter()
        .map(|tool| ExtensionToolBinding {
            name: tool.clone(),
            kind: "mcp_tool".to_string(),
            status: available_status,
            source_path: matching_script_path(&artifact.manifest.scripts, tool),
            description: Some("Generated from compiled skill metadata.".to_string()),
        })
        .collect::<Vec<_>>();
    let mut components = Vec::new();

    if !artifact.manifest.scripts.is_empty() {
        runtime_modes.push("tool_injection".to_string());
    }
    if !artifact.manifest.declared_command_hooks.is_empty() {
        runtime_modes.push("command_hooks".to_string());
        for hook in &artifact.manifest.declared_command_hooks {
            triggers.push(ExtensionTriggerBinding {
                name: hook.clone(),
                kind: "declared_command_hook".to_string(),
                status: declared_status,
                command_preview: None,
                args_hint: artifact.manifest.argument_hint.clone(),
                source_path: None,
            });
        }
    }
    if !artifact.manifest.declared_tool_injections.is_empty() {
        if !runtime_modes.iter().any(|mode| mode == "tool_injection") {
            runtime_modes.push("tool_injection".to_string());
        }
        for binding in &artifact.manifest.declared_tool_injections {
            tools.push(ExtensionToolBinding {
                name: binding.clone(),
                kind: "declared_tool_injection".to_string(),
                status: declared_status,
                source_path: None,
                description: Some(
                    "Declared in skill metadata for a future Rust-native extension runtime."
                        .to_string(),
                ),
            });
        }
    }
    if !artifact.manifest.declared_background_services.is_empty() {
        runtime_modes.push("background_services".to_string());
        for service in &artifact.manifest.declared_background_services {
            components.push(ExtensionComponentBinding {
                name: service.clone(),
                kind: "background_service".to_string(),
                status: declared_status,
                source_path: None,
            });
        }
    }
    if !artifact.manifest.declared_auth_providers.is_empty() {
        runtime_modes.push("auth_plugins".to_string());
        for provider in &artifact.manifest.declared_auth_providers {
            triggers.push(ExtensionTriggerBinding {
                name: format!("openrustclaw.skills.auth.{}", provider),
                kind: "auth_provider".to_string(),
                status: declared_status,
                command_preview: None,
                args_hint: Some("authorization code flow via the runtime vault".to_string()),
                source_path: None,
            });
        }
    }
    if !artifact.manifest.declared_voice_call_plugins.is_empty() {
        runtime_modes.push("voice_call_plugins".to_string());
        for plugin in &artifact.manifest.declared_voice_call_plugins {
            triggers.push(ExtensionTriggerBinding {
                name: format!("openrustclaw.skills.voice-call.{}", plugin),
                kind: "voice_call_plugin".to_string(),
                status: declared_status,
                command_preview: None,
                args_hint: Some("bounded voice call session hooks".to_string()),
                source_path: None,
            });
        }
    }

    let mut component_candidates = artifact
        .manifest
        .declared_wasi_components
        .iter()
        .cloned()
        .map(|component| ExtensionComponentBinding {
            name: component,
            kind: "wasi_component".to_string(),
            status: declared_status,
            source_path: None,
        })
        .collect::<Vec<_>>();
    for path in artifact
        .manifest
        .scripts
        .iter()
        .chain(artifact.manifest.references.iter())
        .filter(|path| path.ends_with(".wasm") || path.ends_with(".wat"))
    {
        component_candidates.push(ExtensionComponentBinding {
            name: Path::new(path)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(path)
                .to_string(),
            kind: "wasi_component".to_string(),
            status: available_status,
            source_path: Some(path.clone()),
        });
    }
    if !component_candidates.is_empty() {
        runtime_modes.push("wasi_component".to_string());
        components.extend(component_candidates);
    }

    let mut notes = vec![
        "This manifest defines the Rust-native extension contract over compiled skill artifacts."
            .to_string(),
    ];
    if blocked {
        notes.push(
            "The extension is blocked by compile-time scanning and remains inspection-only."
                .to_string(),
        );
    } else if artifact.manifest.scripts.is_empty() {
        notes.push(
            "This extension currently ships metadata, references, and bounded invoke surfaces without executable script parity."
                .to_string(),
        );
    } else {
        notes.push(
            "Executable plugin parity remains a later Phase 7 track; the current runtime exposes bounded invoke and MCP retrieval surfaces."
                .to_string(),
        );
    }
    if artifact.manifest.declared_background_services.is_empty()
        && artifact.manifest.declared_auth_providers.is_empty()
        && artifact.manifest.declared_voice_call_plugins.is_empty()
        && artifact.manifest.declared_command_hooks.is_empty()
        && artifact.manifest.declared_tool_injections.is_empty()
        && artifact.manifest.declared_wasi_components.is_empty()
    {
        notes.push(
            "No long-term runtime declarations were found in the skill frontmatter; runtime_modes are inferred from compiled artifacts."
                .to_string(),
        );
    }

    runtime_modes.sort();
    runtime_modes.dedup();

    ExtensionManifest {
        name: artifact.manifest.name.clone(),
        version: artifact.manifest.version.clone(),
        description: artifact.manifest.description.clone(),
        author: artifact.manifest.author.clone(),
        source: artifact.manifest.source,
        verified: artifact.manifest.verified,
        compiled_status: artifact.manifest.status,
        verification_policy: artifact.manifest.verification_policy.clone(),
        local_path: artifact.manifest.local_path.clone(),
        artifact_root: artifact_root.display().to_string(),
        capabilities: artifact.manifest.capabilities.clone(),
        allowed_tools: artifact.manifest.allowed_tools.clone(),
        runtime_modes,
        triggers,
        tools,
        components,
        notes,
        compiled_at: artifact.manifest.compiled_at,
    }
}

pub fn load_extension_manifest(output_root: &Path, name: &str) -> Result<ExtensionManifest> {
    let skill_root = output_root.join(sanitize_name(name));
    read_json(skill_root.join("extension_manifest.json"))
}

pub fn list_extension_manifests(output_root: &Path) -> Result<Vec<ExtensionManifest>> {
    if !output_root.exists() {
        return Ok(Vec::new());
    }

    let mut manifests: Vec<ExtensionManifest> = Vec::new();
    for entry in fs::read_dir(output_root).map_err(|error| {
        Error::Internal(format!(
            "Failed to read {}: {}",
            output_root.display(),
            error
        ))
    })? {
        let entry = entry.map_err(|error| {
            Error::Internal(format!("Failed to read directory entry: {}", error))
        })?;
        let path = entry.path().join("extension_manifest.json");
        if path.exists() {
            manifests.push(read_json(path)?);
        }
    }
    manifests.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(manifests)
}

fn matching_script_path(scripts: &[String], tool_name: &str) -> Option<String> {
    scripts
        .iter()
        .find(|script| tool_name.ends_with(&sanitize_name(script)))
        .cloned()
}

fn read_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Result<T> {
    let bytes = fs::read(&path).map_err(|error| {
        Error::Internal(format!("Failed to read {}: {}", path.display(), error))
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| Error::Internal(format!("Failed to parse {}: {}", path.display(), error)))
}

fn sanitize_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '/' => ch,
            _ => '-',
        })
        .collect::<String>()
        .replace('/', "__")
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use openrustclaw_core::types::SkillSource;

    use crate::compiler::{
        CompiledCliSchema, CompiledHelpIndex, CompiledMcpSchema, CompiledSkillArtifact,
        CompiledSkillManifest, CompiledSkillScanReport, CompiledSkillStatus,
    };

    use super::*;

    #[test]
    fn builds_extension_manifest_from_compiled_skill() {
        let artifact = CompiledSkillArtifact {
            manifest: CompiledSkillManifest {
                name: "demo".to_string(),
                description: "Demo extension".to_string(),
                version: "1.0.0".to_string(),
                author: Some("Example".to_string()),
                source: SkillSource::Workspace,
                status: CompiledSkillStatus::Approved,
                verified: true,
                local_path: "/tmp/demo/SKILL.md".to_string(),
                body_sha256: "abc".to_string(),
                verification_policy: "trusted-local".to_string(),
                capabilities: vec!["shell_exec".to_string()],
                allowed_tools: vec!["Bash".to_string()],
                argument_hint: Some("<repo>".to_string()),
                scripts: vec!["scripts/review.sh".to_string()],
                references: vec!["references/guide.md".to_string()],
                declared_wasi_components: vec!["analyzer".to_string()],
                declared_background_services: vec!["sync-loop".to_string()],
                declared_auth_providers: vec!["okta-prod".to_string()],
                declared_voice_call_plugins: vec!["support-line".to_string()],
                declared_command_hooks: vec!["post_install".to_string()],
                declared_tool_injections: vec!["repo_audit".to_string()],
                compiled_at: Utc::now(),
            },
            help_index: CompiledHelpIndex {
                summary: "Demo".to_string(),
                body_excerpt: "Demo".to_string(),
                argument_hint: Some("<repo>".to_string()),
                allowed_tools: vec!["Bash".to_string()],
                capabilities: vec!["shell_exec".to_string()],
                scripts: vec!["scripts/review.sh".to_string()],
                references: vec!["references/guide.md".to_string()],
                examples: Vec::new(),
                safety_notes: Vec::new(),
            },
            mcp_schema: CompiledMcpSchema {
                prompt_name: "demo".to_string(),
                prompt_description: "Demo".to_string(),
                tool_names: vec!["skill.demo.scripts__review.sh".to_string()],
                resource_names: vec!["skill.demo.references__guide.md".to_string()],
                retrieval_tools: Vec::new(),
            },
            cli_schema: CompiledCliSchema {
                command: "openrustclaw skills invoke demo".to_string(),
                summary: "Demo".to_string(),
                usage: "openrustclaw skills invoke demo <repo>".to_string(),
                args_hint: Some("<repo>".to_string()),
                examples: Vec::new(),
            },
            scan_report: CompiledSkillScanReport {
                scanned_at: Utc::now(),
                status: CompiledSkillStatus::Approved,
                findings: Vec::new(),
                script_count: 1,
                reference_count: 1,
            },
        };

        let manifest = build_extension_manifest(&artifact, Path::new(".claw/skills/compiled"));
        assert!(
            manifest
                .runtime_modes
                .iter()
                .any(|mode| mode == "compiled_metadata")
        );
        assert!(
            manifest
                .runtime_modes
                .iter()
                .any(|mode| mode == "wasi_component")
        );
        assert!(
            manifest
                .runtime_modes
                .iter()
                .any(|mode| mode == "background_services")
        );
        assert!(
            manifest
                .runtime_modes
                .iter()
                .any(|mode| mode == "auth_plugins")
        );
        assert!(
            manifest
                .runtime_modes
                .iter()
                .any(|mode| mode == "voice_call_plugins")
        );
        assert!(
            manifest
                .runtime_modes
                .iter()
                .any(|mode| mode == "command_hooks")
        );
        assert!(
            manifest
                .runtime_modes
                .iter()
                .any(|mode| mode == "tool_injection")
        );
        assert!(
            manifest
                .triggers
                .iter()
                .any(|trigger| trigger.name == "post_install")
        );
        assert!(manifest.tools.iter().any(|tool| tool.name == "repo_audit"));
        assert!(
            manifest
                .components
                .iter()
                .any(|component| component.name == "sync-loop")
        );
    }
}

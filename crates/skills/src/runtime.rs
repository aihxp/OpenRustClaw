//! Shared bounded runtime helpers for compiled skills.

use std::fs;
use std::path::{Path, PathBuf};

use openrustclaw_core::error::{Error, Result};
use openrustclaw_core::types::SkillSource;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::compiler::{CompiledSkillArtifact, CompiledSkillStatus};
use crate::sandbox::{SandboxConfig, WasmSandbox};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompiledBackgroundService {
    pub name: String,
    pub source: String,
    pub executable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledSkillExecutionResult {
    pub skill_name: String,
    pub component: String,
    pub source_path: String,
    pub verified_execution: bool,
    pub runtime: String,
    pub input: Value,
    pub output: Value,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,
}

pub fn compiled_skill_executable_components(artifact: &CompiledSkillArtifact) -> Vec<String> {
    artifact
        .manifest
        .scripts
        .iter()
        .chain(artifact.manifest.references.iter())
        .filter(|path| path.ends_with(".wasm") || path.ends_with(".wat"))
        .cloned()
        .collect()
}

pub fn compiled_skill_background_services(
    artifact: &CompiledSkillArtifact,
) -> Vec<CompiledBackgroundService> {
    let executable_components = compiled_skill_executable_components(artifact);
    let mut services = Vec::new();

    for service in &artifact.manifest.declared_background_services {
        let component = match_background_component(&executable_components, service).or_else(|| {
            if executable_components.len() == 1 {
                executable_components.first().cloned()
            } else {
                None
            }
        });
        services.push(CompiledBackgroundService {
            name: service.clone(),
            source: "declared_background_service".to_string(),
            executable: component.is_some()
                && !matches!(artifact.manifest.status, CompiledSkillStatus::Blocked),
            component,
        });
    }

    if services.is_empty() {
        for component in executable_components {
            services.push(CompiledBackgroundService {
                name: component_service_name(&component),
                source: "compiled_component".to_string(),
                executable: !matches!(artifact.manifest.status, CompiledSkillStatus::Blocked),
                component: Some(component),
            });
        }
    }

    services.sort_by(|left, right| left.name.cmp(&right.name));
    services.dedup_by(|left, right| left.name == right.name && left.component == right.component);
    services
}

pub fn resolve_compiled_skill_background_service(
    artifact: &CompiledSkillArtifact,
    requested_service: Option<&str>,
    requested_component: Option<&str>,
) -> Result<CompiledBackgroundService> {
    let executable_components = compiled_skill_executable_components(artifact);
    let mut services = compiled_skill_background_services(artifact);

    if let Some(component) = requested_component {
        if !executable_components
            .iter()
            .any(|candidate| candidate == component)
        {
            return Err(Error::Internal(format!(
                "Component '{}' is not executable for compiled skill '{}'",
                component, artifact.manifest.name
            )));
        }
    }

    let mut resolved = if let Some(service_name) = requested_service {
        services
            .into_iter()
            .find(|service| service.name == service_name)
            .ok_or_else(|| {
                Error::Internal(format!(
                    "Background service '{}' was not found for '{}'",
                    service_name, artifact.manifest.name
                ))
            })?
    } else if services.len() == 1 {
        services.remove(0)
    } else if services.is_empty() {
        if let Some(component) = requested_component {
            CompiledBackgroundService {
                name: component_service_name(component),
                source: "compiled_component".to_string(),
                executable: true,
                component: Some(component.to_string()),
            }
        } else {
            return Err(Error::Internal(format!(
                "Compiled skill '{}' does not expose a background service or executable component",
                artifact.manifest.name
            )));
        }
    } else {
        return Err(Error::Internal(format!(
            "Compiled skill '{}' exposes multiple background services; choose one explicitly",
            artifact.manifest.name
        )));
    };

    if resolved.component.is_none() {
        resolved.component = requested_component.map(ToString::to_string).or_else(|| {
            if executable_components.len() == 1 {
                executable_components.first().cloned()
            } else {
                None
            }
        });
    } else if let Some(component) = requested_component {
        resolved.component = Some(component.to_string());
    }

    if resolved.component.is_none() {
        return Err(Error::Internal(format!(
            "Background service '{}' for '{}' has no executable component mapping yet; pass an explicit component",
            resolved.name, artifact.manifest.name
        )));
    }

    resolved.executable = !matches!(artifact.manifest.status, CompiledSkillStatus::Blocked);
    Ok(resolved)
}

pub async fn execute_compiled_skill_artifact(
    artifact: &CompiledSkillArtifact,
    requested_component: Option<&str>,
    input: Value,
) -> Result<CompiledSkillExecutionResult> {
    if matches!(artifact.manifest.status, CompiledSkillStatus::Blocked) {
        return Err(Error::Internal(format!(
            "Compiled skill '{}' is blocked; executable runtime is disabled",
            artifact.manifest.name
        )));
    }

    let component = resolve_executable_component(artifact, requested_component)?;
    let source_path = resolve_skill_relative_path(artifact, &component)?;
    let wasm_bytes = fs::read(&source_path).map_err(|error| {
        Error::Internal(format!(
            "Failed to read {}: {}",
            source_path.display(),
            error
        ))
    })?;

    let verified_execution = artifact.manifest.verified
        || matches!(
            artifact.manifest.source,
            SkillSource::Workspace | SkillSource::Bundled
        );
    let sandbox = WasmSandbox::new(SandboxConfig::with_declared_capabilities(
        &artifact.manifest.capabilities,
    )?);
    let output = sandbox
        .execute_with_verified_declared_capabilities(
            &wasm_bytes,
            &artifact.manifest.capabilities,
            verified_execution,
            input.clone(),
        )
        .await?;

    Ok(CompiledSkillExecutionResult {
        skill_name: artifact.manifest.name.clone(),
        component,
        source_path: source_path.display().to_string(),
        verified_execution,
        runtime: "rust_wasm_sandbox".to_string(),
        input,
        output,
        capabilities: artifact.manifest.capabilities.clone(),
    })
}

fn resolve_executable_component(
    artifact: &CompiledSkillArtifact,
    requested: Option<&str>,
) -> Result<String> {
    let candidates = compiled_skill_executable_components(artifact);
    if let Some(component) = requested {
        if candidates.iter().any(|candidate| candidate == component) {
            return Ok(component.to_string());
        }
        return Err(Error::Internal(format!(
            "Component '{}' is not an executable `.wasm`/`.wat` artifact for '{}'",
            component, artifact.manifest.name
        )));
    }

    match candidates.as_slice() {
        [only] => Ok(only.clone()),
        [] => Err(Error::Internal(format!(
            "Skill '{}' does not expose an executable `.wasm` or `.wat` artifact yet",
            artifact.manifest.name
        ))),
        _ => Err(Error::Internal(format!(
            "Skill '{}' has multiple executable components; choose one explicitly",
            artifact.manifest.name
        ))),
    }
}

fn resolve_skill_relative_path(
    artifact: &CompiledSkillArtifact,
    relative: &str,
) -> Result<PathBuf> {
    let skill_file = PathBuf::from(&artifact.manifest.local_path);
    let skill_root = skill_file.parent().ok_or_else(|| {
        Error::Internal(format!(
            "Compiled skill '{}' does not have a resolvable root",
            artifact.manifest.name
        ))
    })?;
    let canonical_root = skill_root.canonicalize().map_err(|error| {
        Error::Internal(format!(
            "Failed to canonicalize {}: {}",
            skill_root.display(),
            error
        ))
    })?;
    let candidate = skill_root.join(relative);
    let canonical_candidate = candidate.canonicalize().map_err(|error| {
        Error::Internal(format!(
            "Failed to resolve {}: {}",
            candidate.display(),
            error
        ))
    })?;
    if !canonical_candidate.starts_with(&canonical_root) {
        return Err(Error::Internal(format!(
            "Path '{}' escapes the skill root for '{}'",
            relative, artifact.manifest.name
        )));
    }
    Ok(canonical_candidate)
}

fn component_service_name(component: &str) -> String {
    Path::new(component)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(component)
        .to_string()
}

fn match_background_component(components: &[String], service: &str) -> Option<String> {
    components
        .iter()
        .find(|candidate| {
            *candidate == service
                || Path::new(candidate)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value == service)
                || Path::new(candidate)
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value == service)
        })
        .cloned()
}

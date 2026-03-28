//! Workspace instruction/context artifact registry and model-aware sync.

use glob::glob;
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use openrustclaw_core::error::{Error, MemoryError, Result};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactClass {
    UniversalInstructions,
    AiManifest,
    Context,
    Architecture,
    SkillGuide,
    Persona,
    Memory,
    ClaudeInstructions,
    ClaudeLocal,
    GeminiInstructions,
    CopilotInstructions,
    CursorRules,
    ContinueRules,
    ModelPackage,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactVisibility {
    Shared,
    Local,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceArtifact {
    pub path: PathBuf,
    pub class: ArtifactClass,
    pub visibility: ArtifactVisibility,
    pub model_family: Option<String>,
    pub content_hash: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResolvedArtifactBundle {
    pub model_family: String,
    pub included: Vec<WorkspaceArtifact>,
    pub merged_instructions: String,
    pub persona: Option<String>,
}

pub struct WorkspaceArtifactRegistry;

impl WorkspaceArtifactRegistry {
    pub fn scan(root: &Path) -> Result<Vec<WorkspaceArtifact>> {
        let mut discovered = Vec::new();
        let mut seen = HashSet::new();

        for relative in EXACT_PATHS {
            let path = root.join(relative);
            if path.is_file() {
                let artifact = artifact_from_path(root, path)?;
                if seen.insert(artifact.path.clone()) {
                    discovered.push(artifact);
                }
            }
        }

        for pattern in GLOB_PATHS {
            let absolute = root.join(pattern).display().to_string();
            for entry in glob(&absolute).map_err(glob_to_memory_error)? {
                let path = entry.map_err(glob_to_memory_error)?;
                if path.is_file() {
                    let artifact = artifact_from_path(root, path)?;
                    if seen.insert(artifact.path.clone()) {
                        discovered.push(artifact);
                    }
                }
            }
        }

        discovered.sort_by_key(artifact_precedence);
        Ok(discovered)
    }

    pub fn resolve(root: &Path, model_id: &str) -> Result<ResolvedArtifactBundle> {
        let family = model_family_for_id(model_id);
        let artifacts = Self::scan(root)?;
        let mut included = Vec::new();
        let mut sections = Vec::new();
        let mut persona = None;

        for artifact in artifacts {
            if artifact_applies_to_family(&artifact, family) {
                let content =
                    fs::read_to_string(root.join(&artifact.path)).map_err(io_to_memory)?;
                if artifact.class == ArtifactClass::Persona {
                    persona = Some(content.clone());
                }
                sections.push(format!(
                    "[{}: {}]\n{}",
                    artifact_label(&artifact.class),
                    artifact.path.display(),
                    content.trim()
                ));
                included.push(artifact);
            }
        }

        Ok(ResolvedArtifactBundle {
            model_family: family.to_string(),
            included,
            merged_instructions: sections.join("\n\n"),
            persona,
        })
    }

    pub fn sync_preferred(root: &Path, model_id: &str) -> Result<Vec<PathBuf>> {
        let bundle = Self::resolve(root, model_id)?;
        let mut written = Vec::new();
        let preferred_targets = preferred_sync_targets(&bundle.model_family);
        for target in preferred_targets {
            let path = root.join(target);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(io_to_memory)?;
            }
            fs::write(&path, bundle.merged_instructions.as_bytes()).map_err(io_to_memory)?;
            written.push(path);
        }

        let generated = root.join(".claw/artifacts/registry.json");
        if let Some(parent) = generated.parent() {
            fs::create_dir_all(parent).map_err(io_to_memory)?;
        }
        let registry_json = serde_json::to_string_pretty(&bundle.included).map_err(|e| {
            Error::Memory(MemoryError::Search(format!(
                "Failed to serialize artifact registry: {}",
                e
            )))
        })?;
        fs::write(&generated, registry_json.as_bytes()).map_err(io_to_memory)?;
        written.push(generated);

        let gitignore = root.join(".claw/.gitignore");
        if let Some(parent) = gitignore.parent() {
            fs::create_dir_all(parent).map_err(io_to_memory)?;
        }
        let mut ignores = BTreeSet::new();
        ignores.insert("artifacts/registry.json".to_string());
        ignores.insert("local/".to_string());
        ignores.insert("memory/views/".to_string());
        let content = ignores.into_iter().collect::<Vec<_>>().join("\n") + "\n";
        fs::write(gitignore, content.as_bytes()).map_err(io_to_memory)?;

        Ok(written)
    }
}

const EXACT_PATHS: &[&str] = &[
    "AGENTS.md",
    "AI.md",
    "CONTEXT.md",
    "ARCHITECTURE.md",
    "SKILL.md",
    "CLAUDE.md",
    "CLAUDE.local.md",
    "GEMINI.md",
    "Modelfile",
    ".github/copilot-instructions.md",
    ".cursorrules",
    ".claw/persona/SOUL.md",
    ".claw/persona/profile.md",
    ".claw/memory/MEMORY.md",
    ".claw/control/CLAW_RUNTIME.md",
];

const GLOB_PATHS: &[&str] = &[
    ".github/instructions/*.instructions.md",
    ".cursor/rules/*",
    ".continue/rules/*",
    ".claw/artifacts/*.md",
    ".claw/local/*.md",
];

fn artifact_from_path(root: &Path, absolute_path: PathBuf) -> Result<WorkspaceArtifact> {
    let relative = absolute_path
        .strip_prefix(root)
        .unwrap_or(&absolute_path)
        .to_path_buf();
    let content = fs::read(&absolute_path).map_err(io_to_memory)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let content_hash = format!("{:x}", hasher.finalize());

    let relative_str = relative.to_string_lossy();
    let (class, model_family, visibility) = classify_relative_path(&relative_str);
    Ok(WorkspaceArtifact {
        path: relative,
        class,
        visibility,
        model_family,
        content_hash,
    })
}

fn classify_relative_path(path: &str) -> (ArtifactClass, Option<String>, ArtifactVisibility) {
    let visibility = if path.contains(".local") || path.starts_with(".claw/local/") {
        ArtifactVisibility::Local
    } else {
        ArtifactVisibility::Shared
    };

    let class = match path {
        "AGENTS.md" => ArtifactClass::UniversalInstructions,
        "AI.md" => ArtifactClass::AiManifest,
        "CONTEXT.md" => ArtifactClass::Context,
        "ARCHITECTURE.md" => ArtifactClass::Architecture,
        "SKILL.md" => ArtifactClass::SkillGuide,
        "CLAUDE.md" => ArtifactClass::ClaudeInstructions,
        "CLAUDE.local.md" => ArtifactClass::ClaudeLocal,
        "GEMINI.md" => ArtifactClass::GeminiInstructions,
        "Modelfile" => ArtifactClass::ModelPackage,
        ".github/copilot-instructions.md" => ArtifactClass::CopilotInstructions,
        ".cursorrules" => ArtifactClass::CursorRules,
        ".claw/persona/SOUL.md" | ".claw/persona/profile.md" => ArtifactClass::Persona,
        ".claw/memory/MEMORY.md" => ArtifactClass::Memory,
        ".claw/control/CLAW_RUNTIME.md" => ArtifactClass::Context,
        _ if path.starts_with(".github/instructions/") => ArtifactClass::CopilotInstructions,
        _ if path.starts_with(".cursor/rules/") => ArtifactClass::CursorRules,
        _ if path.starts_with(".continue/rules/") => ArtifactClass::ContinueRules,
        _ if path.starts_with(".claw/persona/") => ArtifactClass::Persona,
        _ if path.starts_with(".claw/memory/") => ArtifactClass::Memory,
        _ => ArtifactClass::Unknown,
    };

    let model_family = match class {
        ArtifactClass::ClaudeInstructions | ArtifactClass::ClaudeLocal => {
            Some("claude".to_string())
        }
        ArtifactClass::GeminiInstructions => Some("gemini".to_string()),
        ArtifactClass::CopilotInstructions => Some("copilot".to_string()),
        ArtifactClass::CursorRules => Some("cursor".to_string()),
        ArtifactClass::ContinueRules => Some("continue".to_string()),
        ArtifactClass::ModelPackage => Some("ollama".to_string()),
        _ => None,
    };

    (class, model_family, visibility)
}

fn artifact_label(class: &ArtifactClass) -> &'static str {
    match class {
        ArtifactClass::UniversalInstructions => "agents",
        ArtifactClass::AiManifest => "ai",
        ArtifactClass::Context => "context",
        ArtifactClass::Architecture => "architecture",
        ArtifactClass::SkillGuide => "skill",
        ArtifactClass::Persona => "persona",
        ArtifactClass::Memory => "memory",
        ArtifactClass::ClaudeInstructions => "claude",
        ArtifactClass::ClaudeLocal => "claude_local",
        ArtifactClass::GeminiInstructions => "gemini",
        ArtifactClass::CopilotInstructions => "copilot",
        ArtifactClass::CursorRules => "cursor",
        ArtifactClass::ContinueRules => "continue",
        ArtifactClass::ModelPackage => "modelfile",
        ArtifactClass::Unknown => "unknown",
    }
}

fn model_family_for_id(model_id: &str) -> &str {
    let model_id = model_id.to_lowercase();
    if model_id.contains("claude") {
        "claude"
    } else if model_id.contains("gemini") {
        "gemini"
    } else if model_id.contains("copilot") {
        "copilot"
    } else if model_id.contains("cursor") {
        "cursor"
    } else if model_id.contains("continue") {
        "continue"
    } else if model_id.contains("ollama")
        || model_id.contains("llama")
        || model_id.contains("deepseek")
        || model_id.contains("qwen")
        || model_id.contains("mistral")
    {
        "ollama"
    } else {
        "generic"
    }
}

fn artifact_precedence(artifact: &WorkspaceArtifact) -> (u8, u8, String) {
    let visibility = match artifact.visibility {
        ArtifactVisibility::Shared => 0,
        ArtifactVisibility::Local => 1,
    };
    let class_order = match artifact.class {
        ArtifactClass::UniversalInstructions => 0,
        ArtifactClass::AiManifest => 1,
        ArtifactClass::Context => 2,
        ArtifactClass::Architecture => 3,
        ArtifactClass::SkillGuide => 4,
        ArtifactClass::Persona => 5,
        ArtifactClass::Memory => 6,
        ArtifactClass::ClaudeInstructions
        | ArtifactClass::ClaudeLocal
        | ArtifactClass::GeminiInstructions
        | ArtifactClass::CopilotInstructions
        | ArtifactClass::CursorRules
        | ArtifactClass::ContinueRules
        | ArtifactClass::ModelPackage => 7,
        ArtifactClass::Unknown => 8,
    };
    (visibility, class_order, artifact.path.display().to_string())
}

fn artifact_applies_to_family(artifact: &WorkspaceArtifact, family: &str) -> bool {
    match artifact.class {
        ArtifactClass::ClaudeLocal => family == "claude",
        ArtifactClass::ClaudeInstructions
        | ArtifactClass::GeminiInstructions
        | ArtifactClass::CopilotInstructions
        | ArtifactClass::CursorRules
        | ArtifactClass::ContinueRules
        | ArtifactClass::ModelPackage => {
            artifact.model_family.as_deref() == Some(family)
                || (family == "generic" && artifact.visibility == ArtifactVisibility::Shared)
        }
        ArtifactClass::Unknown => false,
        _ => true,
    }
}

fn preferred_sync_targets(family: &str) -> &'static [&'static str] {
    match family {
        "claude" => &["CLAUDE.md"],
        "gemini" => &["GEMINI.md"],
        "copilot" => &[".github/copilot-instructions.md"],
        "cursor" => &[".cursorrules"],
        "continue" => &[".continue/rules/openrustclaw.md"],
        "ollama" => &["AGENTS.md", ".claw/models/Modelfile.instructions.md"],
        _ => &["AGENTS.md"],
    }
}

fn io_to_memory(error: std::io::Error) -> Error {
    Error::Memory(MemoryError::Search(error.to_string()))
}

fn glob_to_memory_error<E: std::fmt::Display>(error: E) -> Error {
    Error::Memory(MemoryError::Search(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn scan_and_resolve_workspace_artifacts() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("AGENTS.md"), "Shared instructions").unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "Claude instructions").unwrap();
        fs::create_dir_all(dir.path().join(".claw/persona")).unwrap();
        fs::write(dir.path().join(".claw/persona/SOUL.md"), "Be concise").unwrap();

        let artifacts = WorkspaceArtifactRegistry::scan(dir.path()).unwrap();
        assert_eq!(artifacts.len(), 3);

        let bundle = WorkspaceArtifactRegistry::resolve(dir.path(), "claude-sonnet").unwrap();
        assert!(bundle.merged_instructions.contains("Shared instructions"));
        assert!(bundle.merged_instructions.contains("Claude instructions"));
        assert_eq!(bundle.persona.as_deref(), Some("Be concise"));
    }

    #[test]
    fn sync_writes_preferred_target_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("AGENTS.md"), "Shared instructions").unwrap();

        let written = WorkspaceArtifactRegistry::sync_preferred(dir.path(), "gemini-2.0").unwrap();
        assert!(written.iter().any(|path| path.ends_with("GEMINI.md")));
        assert!(dir.path().join("GEMINI.md").is_file());
    }
}

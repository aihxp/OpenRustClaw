use openrustclaw_core::error::{Error, Result};
use openrustclaw_skills::{
    CompiledSkillArtifact, CompiledSkillManifest, list_compiled_manifests, load_compiled_artifact,
};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

pub struct CompiledSkillOverviewService {
    root: PathBuf,
}

impl CompiledSkillOverviewService {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn manifests(&self) -> Result<Vec<CompiledSkillManifest>> {
        list_compiled_manifests(&self.root).map_err(|error| {
            Error::Internal(format!("failed to load compiled skill manifests: {error}"))
        })
    }

    pub fn artifact(&self, name: &str) -> Result<CompiledSkillArtifact> {
        load_compiled_artifact(&self.root, name).map_err(|error| {
            Error::Internal(format!("failed to load compiled skill '{name}': {error}"))
        })
    }

    pub fn artifacts(&self) -> Vec<CompiledSkillArtifact> {
        let Ok(manifests) = self.manifests() else {
            return Vec::new();
        };

        manifests
            .into_iter()
            .filter_map(|manifest| self.artifact(&manifest.name).ok())
            .collect()
    }

    pub fn executable_components(artifact: &CompiledSkillArtifact) -> Vec<String> {
        artifact
            .manifest
            .scripts
            .iter()
            .chain(artifact.manifest.references.iter())
            .filter(|path| path.ends_with(".wasm") || path.ends_with(".wat"))
            .cloned()
            .collect()
    }

    pub fn read_reference(
        &self,
        artifact: &CompiledSkillArtifact,
        reference: &str,
        max_chars: Option<usize>,
    ) -> Result<Value> {
        if !artifact
            .manifest
            .references
            .iter()
            .any(|entry| entry == reference)
        {
            return Err(Error::Internal(format!(
                "reference '{reference}' is not part of compiled skill '{}'",
                artifact.manifest.name
            )));
        }

        let canonical_candidate = resolve_skill_relative_path(artifact, reference)?;
        let bytes = fs::read(&canonical_candidate).map_err(|error| {
            Error::Internal(format!(
                "failed to read {}: {error}",
                canonical_candidate.display()
            ))
        })?;
        let metadata = fs::metadata(&canonical_candidate).map_err(|error| {
            Error::Internal(format!(
                "failed to stat {}: {error}",
                canonical_candidate.display()
            ))
        })?;
        let max_chars = max_chars.unwrap_or(4000).max(1);

        match String::from_utf8(bytes) {
            Ok(text) => {
                let char_len = text.chars().count();
                let truncated = char_len > max_chars;
                let content = if truncated {
                    text.chars().take(max_chars).collect::<String>()
                } else {
                    text
                };
                Ok(json!({
                    "reference": reference,
                    "path": canonical_candidate.display().to_string(),
                    "binary": false,
                    "bytes": metadata.len(),
                    "truncated": truncated,
                    "content": content,
                }))
            }
            Err(error) => Ok(json!({
                "reference": reference,
                "path": canonical_candidate.display().to_string(),
                "binary": true,
                "bytes": metadata.len(),
                "encoding_error": error.to_string(),
            })),
        }
    }
}

fn resolve_skill_relative_path(
    artifact: &CompiledSkillArtifact,
    relative: &str,
) -> Result<PathBuf> {
    let skill_file = PathBuf::from(&artifact.manifest.local_path);
    let skill_root = skill_file.parent().ok_or_else(|| {
        Error::Internal(format!(
            "compiled skill '{}' does not have a resolvable root",
            artifact.manifest.name
        ))
    })?;
    let canonical_root = skill_root.canonicalize().map_err(|error| {
        Error::Internal(format!(
            "failed to canonicalize {}: {error}",
            skill_root.display()
        ))
    })?;
    let candidate = skill_root.join(relative);
    let canonical_candidate = candidate.canonicalize().map_err(|error| {
        Error::Internal(format!(
            "failed to resolve {}: {error}",
            candidate.display()
        ))
    })?;
    if !canonical_candidate.starts_with(&canonical_root) {
        return Err(Error::Internal(format!(
            "path '{relative}' escapes the skill root for '{}'",
            artifact.manifest.name
        )));
    }
    Ok(canonical_candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openrustclaw_core::types::SkillSource;
    use openrustclaw_skills::compile_skill_to_dir;
    use tempfile::tempdir;

    #[test]
    fn compiled_skill_overview_lists_artifacts_and_reads_references() -> Result<()> {
        let tmp = tempdir().expect("tempdir");
        let skill_dir = tmp.path().join("demo");
        fs::create_dir_all(skill_dir.join("references"))
            .map_err(|error| Error::Internal(error.to_string()))?;
        fs::write(
            skill_dir.join("SKILL.md"),
            "# Demo Skill\n\nUseful compiled skill.\n",
        )
        .map_err(|error| Error::Internal(error.to_string()))?;
        fs::write(
            skill_dir.join("references").join("guide.md"),
            "This is the guide for the demo skill.",
        )
        .map_err(|error| Error::Internal(error.to_string()))?;
        compile_skill_to_dir(
            &skill_dir.join("SKILL.md"),
            &tmp.path().join("compiled"),
            SkillSource::Workspace,
            true,
        )
        .map_err(|error| Error::Internal(error.to_string()))?;

        let service = CompiledSkillOverviewService::new(tmp.path().join("compiled"));
        let manifests = service.manifests()?;
        assert_eq!(manifests.len(), 1);

        let artifact = service.artifact("Demo Skill")?;
        let payload = service.read_reference(&artifact, "references/guide.md", Some(4))?;
        assert_eq!(payload["reference"], "references/guide.md");
        assert_eq!(payload["content"], "This");
        assert_eq!(payload["truncated"], true);
        Ok(())
    }
}

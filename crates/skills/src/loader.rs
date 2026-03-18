//! Skill discovery and lazy loading.

use std::path::{Path, PathBuf};

use openrustclaw_core::error::{Error, Result};
use openrustclaw_core::types::{SkillCapability, SkillSource};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    declared_sensitive_capability_names, normalize_capability_names, parse_capability_names,
};

/// Metadata parsed from a SKILL.md file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
    pub source: SkillSource,
    pub capabilities: Vec<String>,
    pub author: Option<String>,
}

impl SkillMetadata {
    /// Return the parsed capability set for this skill metadata.
    pub fn parsed_capabilities(&self) -> Result<std::collections::HashSet<SkillCapability>> {
        parse_capability_names(&self.capabilities)
    }

    /// Return the sensitive declared capabilities for this skill.
    pub fn sensitive_capabilities(&self) -> Result<Vec<String>> {
        declared_sensitive_capability_names(&self.capabilities)
    }

    /// Whether any declared capabilities are considered sensitive.
    pub fn has_sensitive_capabilities(&self) -> Result<bool> {
        Ok(!self.sensitive_capabilities()?.is_empty())
    }

    /// Whether this skill should require verification before privileged execution.
    pub fn requires_verified_signature(&self) -> Result<bool> {
        self.has_sensitive_capabilities()
    }

    /// Return a short verification policy summary for operators and logs.
    pub fn verification_policy_summary(&self) -> Result<String> {
        let sensitive = self.sensitive_capabilities()?;
        if sensitive.is_empty() {
            return Ok("verification optional".to_string());
        }

        Ok(format!(
            "verification required for sensitive capabilities: {}",
            sensitive.join(", ")
        ))
    }
}

/// Loads skills from the filesystem.
pub struct SkillLoader {
    skill_dirs: Vec<PathBuf>,
}

impl SkillLoader {
    pub fn new(skill_dirs: Vec<PathBuf>) -> Self {
        Self { skill_dirs }
    }

    /// Discover all SKILL.md files in configured directories.
    pub fn discover(&self) -> Result<Vec<SkillMetadata>> {
        let mut skills = Vec::new();

        for dir in &self.skill_dirs {
            if !dir.exists() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let skill_file = path.join("SKILL.md");
                        if skill_file.exists() {
                            match self.parse_skill_md(&skill_file) {
                                Ok(meta) => {
                                    info!(name = %meta.name, "Discovered skill");
                                    skills.push(meta);
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        path = %skill_file.display(),
                                        error = %e,
                                        "Failed to parse SKILL.md"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(skills)
    }

    /// Parse a SKILL.md file into metadata.
    fn parse_skill_md(&self, path: &Path) -> Result<SkillMetadata> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::Internal(format!("Failed to read SKILL.md: {}", e)))?;

        // Simple YAML front-matter parser
        let mut name = String::new();
        let mut description = String::new();
        let mut version = "0.1.0".to_string();
        let mut capabilities = Vec::new();
        let mut author = None;

        let mut in_frontmatter = false;
        for line in content.lines() {
            if line.trim() == "---" {
                if in_frontmatter {
                    // End of front-matter
                    break;
                }
                in_frontmatter = true;
                continue;
            }
            if in_frontmatter {
                if let Some(val) = line.strip_prefix("name:") {
                    name = val.trim().trim_matches('"').to_string();
                } else if let Some(val) = line.strip_prefix("description:") {
                    description = val.trim().trim_matches('"').to_string();
                } else if let Some(val) = line.strip_prefix("version:") {
                    version = val.trim().trim_matches('"').to_string();
                } else if let Some(val) = line.strip_prefix("author:") {
                    author = Some(val.trim().trim_matches('"').to_string());
                } else if let Some(val) = line.strip_prefix("capabilities:") {
                    capabilities = val
                        .split(',')
                        .map(|entry| entry.trim().trim_matches('"'))
                        .filter(|entry| !entry.is_empty())
                        .map(ToString::to_string)
                        .collect();
                }
            }
        }

        if name.is_empty() {
            // Fall back to directory name
            name = path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
        }
        let capabilities = normalize_capability_names(&capabilities)?;

        Ok(SkillMetadata {
            name,
            description,
            version,
            source: SkillSource::Workspace,
            capabilities,
            author,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Helper: create a temp directory with a skill sub-directory containing a SKILL.md
    fn create_skill_dir(base: &Path, skill_name: &str, content: &str) -> PathBuf {
        let skill_dir = base.join(skill_name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), content).unwrap();
        skill_dir
    }

    // ── SkillMetadata tests ─────────────────────────────────────────────

    #[test]
    fn test_skill_metadata_serialization_roundtrip() {
        let meta = SkillMetadata {
            name: "test-skill".to_string(),
            description: "A test".to_string(),
            version: "1.2.3".to_string(),
            source: SkillSource::Workspace,
            capabilities: vec!["cap1".to_string(), "cap2".to_string()],
            author: Some("Author".to_string()),
        };

        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: SkillMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "test-skill");
        assert_eq!(deserialized.description, "A test");
        assert_eq!(deserialized.version, "1.2.3");
        assert_eq!(deserialized.capabilities.len(), 2);
        assert_eq!(deserialized.author, Some("Author".to_string()));
    }

    #[test]
    fn test_skill_metadata_without_author() {
        let meta = SkillMetadata {
            name: "anon-skill".to_string(),
            description: "No author".to_string(),
            version: "0.1.0".to_string(),
            source: SkillSource::Bundled,
            capabilities: vec![],
            author: None,
        };

        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: SkillMetadata = serde_json::from_str(&json).unwrap();
        assert!(deserialized.author.is_none());
    }

    #[test]
    fn test_skill_metadata_clone() {
        let meta = SkillMetadata {
            name: "cloneable".to_string(),
            description: "Can be cloned".to_string(),
            version: "1.0.0".to_string(),
            source: SkillSource::Managed,
            capabilities: vec!["network".to_string()],
            author: Some("dev".to_string()),
        };

        let cloned = meta.clone();
        assert_eq!(cloned.name, meta.name);
        assert_eq!(cloned.description, meta.description);
        assert_eq!(cloned.version, meta.version);
        assert_eq!(cloned.capabilities, meta.capabilities);
    }

    #[test]
    fn test_skill_metadata_requires_verification_for_sensitive_caps() {
        let meta = SkillMetadata {
            name: "danger".to_string(),
            description: "Sensitive".to_string(),
            version: "1.0.0".to_string(),
            source: SkillSource::Managed,
            capabilities: vec!["shell_exec".to_string(), "file_read".to_string()],
            author: None,
        };

        assert!(meta.requires_verified_signature().unwrap());
        assert_eq!(
            meta.sensitive_capabilities().unwrap(),
            vec!["shell_exec".to_string()]
        );
        assert!(meta.has_sensitive_capabilities().unwrap());
        assert_eq!(
            meta.verification_policy_summary().unwrap(),
            "verification required for sensitive capabilities: shell_exec"
        );
    }

    // ── SkillLoader construction tests ──────────────────────────────────

    #[test]
    fn test_loader_new_with_empty_dirs() {
        let loader = SkillLoader::new(vec![]);
        let skills = loader.discover().unwrap();
        assert!(skills.is_empty());
    }

    #[test]
    fn test_loader_new_with_nonexistent_dirs() {
        let loader = SkillLoader::new(vec![
            PathBuf::from("/nonexistent/path/1"),
            PathBuf::from("/nonexistent/path/2"),
        ]);
        let skills = loader.discover().unwrap();
        assert!(skills.is_empty());
    }

    // ── SKILL.md parsing tests ──────────────────────────────────────────

    #[test]
    fn test_parse_skill_md_full_frontmatter() {
        let tmp = tempfile::tempdir().unwrap();
        let content = r#"---
name: "my-awesome-skill"
description: "Does awesome things"
version: "2.1.0"
author: "Jane Doe"
capabilities: "network_access, file_read"
---

# My Awesome Skill

This skill does awesome things.
"#;
        create_skill_dir(tmp.path(), "my-awesome-skill", content);

        let loader = SkillLoader::new(vec![tmp.path().to_path_buf()]);
        let skills = loader.discover().unwrap();

        assert_eq!(skills.len(), 1);
        let skill = &skills[0];
        assert_eq!(skill.name, "my-awesome-skill");
        assert_eq!(skill.description, "Does awesome things");
        assert_eq!(skill.version, "2.1.0");
        assert_eq!(skill.author, Some("Jane Doe".to_string()));
        assert_eq!(skill.capabilities, vec!["file_read", "network_access"]);
        let parsed = skill.parsed_capabilities().unwrap();
        assert!(parsed.contains(&SkillCapability::NetworkAccess));
        assert!(parsed.contains(&SkillCapability::FileRead));
    }

    #[test]
    fn test_parse_skill_md_minimal_frontmatter() {
        let tmp = tempfile::tempdir().unwrap();
        let content = r#"---
name: minimal-skill
---
"#;
        create_skill_dir(tmp.path(), "minimal-skill", content);

        let loader = SkillLoader::new(vec![tmp.path().to_path_buf()]);
        let skills = loader.discover().unwrap();

        assert_eq!(skills.len(), 1);
        let skill = &skills[0];
        assert_eq!(skill.name, "minimal-skill");
        assert_eq!(skill.version, "0.1.0"); // default version
        assert!(skill.author.is_none());
    }

    #[test]
    fn test_parse_skill_md_no_name_uses_dir_name() {
        let tmp = tempfile::tempdir().unwrap();
        let content = r#"---
description: "A skill without a name field"
version: "1.0.0"
---
"#;
        create_skill_dir(tmp.path(), "dir-based-name", content);

        let loader = SkillLoader::new(vec![tmp.path().to_path_buf()]);
        let skills = loader.discover().unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "dir-based-name");
    }

    #[test]
    fn test_parse_skill_md_no_frontmatter_uses_dir_name() {
        let tmp = tempfile::tempdir().unwrap();
        let content = "# Just a markdown file\n\nNo front matter here.\n";
        create_skill_dir(tmp.path(), "fallback-name", content);

        let loader = SkillLoader::new(vec![tmp.path().to_path_buf()]);
        let skills = loader.discover().unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "fallback-name");
        assert_eq!(skills[0].version, "0.1.0");
    }

    #[test]
    fn test_parse_skill_md_quoted_values() {
        let tmp = tempfile::tempdir().unwrap();
        let content = r#"---
name: "quoted-name"
description: "A quoted description"
version: "3.0.0"
author: "Quoted Author"
---
"#;
        create_skill_dir(tmp.path(), "quoted", content);

        let loader = SkillLoader::new(vec![tmp.path().to_path_buf()]);
        let skills = loader.discover().unwrap();

        assert_eq!(skills.len(), 1);
        let skill = &skills[0];
        assert_eq!(skill.name, "quoted-name");
        assert_eq!(skill.description, "A quoted description");
        assert_eq!(skill.version, "3.0.0");
        assert_eq!(skill.author, Some("Quoted Author".to_string()));
    }

    #[test]
    fn test_discover_skips_skill_with_unknown_capability() {
        let tmp = tempfile::tempdir().unwrap();
        let content = r#"---
name: invalid-skill
capabilities: "launch_missiles"
---
"#;
        create_skill_dir(tmp.path(), "invalid-skill", content);

        let loader = SkillLoader::new(vec![tmp.path().to_path_buf()]);
        let skills = loader.discover().unwrap();

        assert!(skills.is_empty());
    }

    // ── discover() tests ────────────────────────────────────────────────

    #[test]
    fn test_discover_multiple_skills() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..5 {
            let content = format!(
                "---\nname: skill-{}\ndescription: Skill number {}\nversion: 1.0.{}\n---\n",
                i, i, i
            );
            create_skill_dir(tmp.path(), &format!("skill-{}", i), &content);
        }

        let loader = SkillLoader::new(vec![tmp.path().to_path_buf()]);
        let skills = loader.discover().unwrap();

        assert_eq!(skills.len(), 5);
    }

    #[test]
    fn test_discover_skips_dirs_without_skill_md() {
        let tmp = tempfile::tempdir().unwrap();
        // Create a dir with SKILL.md
        create_skill_dir(tmp.path(), "with-skill", "---\nname: with-skill\n---\n");
        // Create a dir without SKILL.md
        fs::create_dir_all(tmp.path().join("without-skill")).unwrap();
        fs::write(
            tmp.path().join("without-skill").join("README.md"),
            "Not a skill",
        )
        .unwrap();

        let loader = SkillLoader::new(vec![tmp.path().to_path_buf()]);
        let skills = loader.discover().unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "with-skill");
    }

    #[test]
    fn test_discover_skips_files_at_root_level() {
        let tmp = tempfile::tempdir().unwrap();
        // SKILL.md at root level (not inside a subdirectory) should be ignored
        fs::write(tmp.path().join("SKILL.md"), "---\nname: root\n---\n").unwrap();
        // Also add a valid one inside a subdir
        create_skill_dir(tmp.path(), "valid", "---\nname: valid\n---\n");

        let loader = SkillLoader::new(vec![tmp.path().to_path_buf()]);
        let skills = loader.discover().unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "valid");
    }

    #[test]
    fn test_discover_from_multiple_directories() {
        let tmp1 = tempfile::tempdir().unwrap();
        let tmp2 = tempfile::tempdir().unwrap();

        create_skill_dir(tmp1.path(), "skill-a", "---\nname: skill-a\n---\n");
        create_skill_dir(tmp2.path(), "skill-b", "---\nname: skill-b\n---\n");

        let loader = SkillLoader::new(vec![tmp1.path().to_path_buf(), tmp2.path().to_path_buf()]);
        let skills = loader.discover().unwrap();

        assert_eq!(skills.len(), 2);
        let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"skill-a"));
        assert!(names.contains(&"skill-b"));
    }

    #[test]
    fn test_discover_all_skills_have_workspace_source() {
        let tmp = tempfile::tempdir().unwrap();
        create_skill_dir(tmp.path(), "ws-skill", "---\nname: ws-skill\n---\n");

        let loader = SkillLoader::new(vec![tmp.path().to_path_buf()]);
        let skills = loader.discover().unwrap();

        assert_eq!(skills.len(), 1);
        assert!(matches!(skills[0].source, SkillSource::Workspace));
    }
}

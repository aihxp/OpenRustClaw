//! Skill discovery and lazy loading.

use std::path::{Path, PathBuf};

use openrustclaw_core::error::{Error, Result};
use openrustclaw_core::types::SkillSource;
use serde::{Deserialize, Serialize};
use tracing::info;

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
        let capabilities = Vec::new();
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

//! Skill capability parsing and normalization helpers.

use std::collections::HashSet;

use openrustclaw_core::error::{Error, Result, ToolError};
use openrustclaw_core::types::SkillCapability;

pub fn canonical_capability_name(capability: &SkillCapability) -> &'static str {
    match capability {
        SkillCapability::FileRead => "file_read",
        SkillCapability::FileWrite => "file_write",
        SkillCapability::NetworkAccess => "network_access",
        SkillCapability::ShellExec => "shell_exec",
        SkillCapability::DatabaseAccess => "database_access",
        SkillCapability::MemoryWrite => "memory_write",
    }
}

pub fn parse_capability_name(name: &str) -> Result<SkillCapability> {
    let normalized = name.trim().to_ascii_lowercase().replace('-', "_");
    match normalized.as_str() {
        "file_read" => Ok(SkillCapability::FileRead),
        "file_write" => Ok(SkillCapability::FileWrite),
        "network_access" | "network" => Ok(SkillCapability::NetworkAccess),
        "shell_exec" | "shell" => Ok(SkillCapability::ShellExec),
        "database_access" | "database" | "db" => Ok(SkillCapability::DatabaseAccess),
        "memory_write" | "memory" => Ok(SkillCapability::MemoryWrite),
        _ => Err(Error::Tool(ToolError::InputValidation {
            tool: "skill_capabilities".to_string(),
            message: format!("Unknown skill capability '{}'", name.trim()),
        })),
    }
}

pub fn parse_capability_names(capabilities: &[String]) -> Result<HashSet<SkillCapability>> {
    capabilities
        .iter()
        .map(|capability| parse_capability_name(capability))
        .collect()
}

pub fn normalize_capability_names(capabilities: &[String]) -> Result<Vec<String>> {
    let mut normalized: Vec<String> = parse_capability_names(capabilities)?
        .into_iter()
        .map(|capability| canonical_capability_name(&capability).to_string())
        .collect();
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

pub fn is_sensitive_capability(capability: &SkillCapability) -> bool {
    matches!(
        capability,
        SkillCapability::FileWrite
            | SkillCapability::NetworkAccess
            | SkillCapability::ShellExec
            | SkillCapability::DatabaseAccess
            | SkillCapability::MemoryWrite
    )
}

pub fn sensitive_capabilities(capabilities: &std::collections::HashSet<SkillCapability>) -> Vec<SkillCapability> {
    let mut sensitive: Vec<SkillCapability> = capabilities
        .iter()
        .filter(|capability| is_sensitive_capability(capability))
        .cloned()
        .collect();
    sensitive.sort_by_key(|capability| canonical_capability_name(capability));
    sensitive
}

pub fn declared_sensitive_capability_names(capabilities: &[String]) -> Result<Vec<String>> {
    Ok(sensitive_capabilities(&parse_capability_names(capabilities)?)
        .into_iter()
        .map(|capability| canonical_capability_name(&capability).to_string())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_capabilities() {
        assert_eq!(
            parse_capability_name("network_access").unwrap(),
            SkillCapability::NetworkAccess
        );
        assert_eq!(
            parse_capability_name("file-read").unwrap(),
            SkillCapability::FileRead
        );
    }

    #[test]
    fn rejects_unknown_capabilities() {
        let error = parse_capability_name("launch_missiles").unwrap_err();
        assert!(error.to_string().contains("Unknown skill capability"));
    }

    #[test]
    fn normalizes_capability_sets() {
        let normalized = normalize_capability_names(&[
            "network".to_string(),
            "file-read".to_string(),
            "network_access".to_string(),
        ])
        .unwrap();

        assert_eq!(
            normalized,
            vec!["file_read".to_string(), "network_access".to_string()]
        );
    }

    #[test]
    fn identifies_sensitive_capabilities() {
        assert!(is_sensitive_capability(&SkillCapability::ShellExec));
        assert!(!is_sensitive_capability(&SkillCapability::FileRead));
    }

    #[test]
    fn filters_declared_sensitive_capabilities() {
        let sensitive = declared_sensitive_capability_names(&[
            "file_read".to_string(),
            "network".to_string(),
            "shell_exec".to_string(),
        ])
        .unwrap();

        assert_eq!(
            sensitive,
            vec!["network_access".to_string(), "shell_exec".to_string()]
        );
    }
}

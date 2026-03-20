//! Skill compile pipeline for cached help/schema artifacts.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use openrustclaw_core::error::{Error, Result};
use openrustclaw_core::types::SkillSource;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::declared_sensitive_capability_names;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompiledSkillStatus {
    Discovered,
    Scanned,
    Compiled,
    Approved,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledSkillScanFinding {
    pub severity: FindingSeverity,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledSkillScanReport {
    pub scanned_at: DateTime<Utc>,
    pub status: CompiledSkillStatus,
    pub findings: Vec<CompiledSkillScanFinding>,
    pub script_count: usize,
    pub reference_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledSkillManifest {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: Option<String>,
    pub source: SkillSource,
    pub status: CompiledSkillStatus,
    pub verified: bool,
    pub local_path: String,
    pub body_sha256: String,
    pub verification_policy: String,
    pub capabilities: Vec<String>,
    pub allowed_tools: Vec<String>,
    pub argument_hint: Option<String>,
    pub scripts: Vec<String>,
    pub references: Vec<String>,
    pub compiled_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledHelpIndex {
    pub summary: String,
    pub body_excerpt: String,
    pub argument_hint: Option<String>,
    pub allowed_tools: Vec<String>,
    pub capabilities: Vec<String>,
    pub scripts: Vec<String>,
    pub references: Vec<String>,
    pub examples: Vec<String>,
    pub safety_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledMcpSchema {
    pub prompt_name: String,
    pub prompt_description: String,
    pub tool_names: Vec<String>,
    pub resource_names: Vec<String>,
    pub retrieval_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledCliSchema {
    pub command: String,
    pub summary: String,
    pub usage: String,
    pub args_hint: Option<String>,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledSkillArtifact {
    pub manifest: CompiledSkillManifest,
    pub help_index: CompiledHelpIndex,
    pub mcp_schema: CompiledMcpSchema,
    pub cli_schema: CompiledCliSchema,
    pub scan_report: CompiledSkillScanReport,
}

#[derive(Debug, Clone)]
struct ParsedSkillDocument {
    name: String,
    description: String,
    version: String,
    author: Option<String>,
    argument_hint: Option<String>,
    capabilities: Vec<String>,
    allowed_tools: Vec<String>,
    body: String,
}

pub fn compile_skill_file(
    path: &Path,
    source: SkillSource,
    verified: bool,
) -> Result<CompiledSkillArtifact> {
    let content = fs::read_to_string(path).map_err(|error| {
        Error::Internal(format!("Failed to read {}: {}", path.display(), error))
    })?;
    let parsed = parse_skill_document(path, &content);
    let scripts = list_relative_files(path.parent().unwrap_or_else(|| Path::new(".")), "scripts");
    let references = list_relative_files(
        path.parent().unwrap_or_else(|| Path::new(".")),
        "references",
    );
    let findings = scan_skill(
        &content,
        &scripts,
        &parsed.allowed_tools,
        &parsed.capabilities,
        path.parent().unwrap_or_else(|| Path::new(".")),
    );
    let status = derive_status(source, verified, &findings);
    let compiled_at = Utc::now();
    let body_sha256 = hash_bytes(content.as_bytes());
    let safety_notes = findings
        .iter()
        .filter(|finding| {
            matches!(
                finding.severity,
                FindingSeverity::Warning | FindingSeverity::Error | FindingSeverity::Critical
            )
        })
        .map(|finding| finding.message.clone())
        .collect::<Vec<_>>();
    let examples = collect_examples(&parsed.body);
    let summary = if !parsed.description.is_empty() {
        parsed.description.clone()
    } else {
        first_body_paragraph(&parsed.body)
    };
    let help_index = CompiledHelpIndex {
        summary: summary.clone(),
        body_excerpt: excerpt(&parsed.body, 420),
        argument_hint: parsed.argument_hint.clone(),
        allowed_tools: parsed.allowed_tools.clone(),
        capabilities: parsed.capabilities.clone(),
        scripts: scripts.clone(),
        references: references.clone(),
        examples: examples.clone(),
        safety_notes,
    };
    let manifest = CompiledSkillManifest {
        name: parsed.name.clone(),
        description: summary.clone(),
        version: parsed.version.clone(),
        author: parsed.author.clone(),
        source,
        status,
        verified,
        local_path: path.display().to_string(),
        body_sha256,
        verification_policy: verification_policy(source, verified, &parsed.capabilities),
        capabilities: parsed.capabilities.clone(),
        allowed_tools: parsed.allowed_tools.clone(),
        argument_hint: parsed.argument_hint.clone(),
        scripts: scripts.clone(),
        references: references.clone(),
        compiled_at,
    };
    let mcp_schema = CompiledMcpSchema {
        prompt_name: parsed.name.clone(),
        prompt_description: summary.clone(),
        tool_names: scripts
            .iter()
            .map(|entry| {
                format!(
                    "skill.{}.{}",
                    sanitize_name(&parsed.name),
                    sanitize_name(entry)
                )
            })
            .collect(),
        resource_names: references
            .iter()
            .map(|entry| {
                format!(
                    "skill.{}.{}",
                    sanitize_name(&parsed.name),
                    sanitize_name(entry)
                )
            })
            .collect(),
        retrieval_tools: vec![
            "get_skill_summary".to_string(),
            "get_skill_details".to_string(),
            "get_skill_related_file".to_string(),
        ],
    };
    let cli_schema = CompiledCliSchema {
        command: format!("openrustclaw skills invoke {}", parsed.name),
        summary,
        usage: match parsed.argument_hint.as_deref() {
            Some(hint) => format!("openrustclaw skills invoke {} {}", parsed.name, hint),
            None => format!("openrustclaw skills invoke {}", parsed.name),
        },
        args_hint: parsed.argument_hint.clone(),
        examples,
    };
    let scan_report = CompiledSkillScanReport {
        scanned_at: compiled_at,
        status,
        findings,
        script_count: scripts.len(),
        reference_count: references.len(),
    };

    Ok(CompiledSkillArtifact {
        manifest,
        help_index,
        mcp_schema,
        cli_schema,
        scan_report,
    })
}

pub fn compile_skill_to_dir(
    path: &Path,
    output_root: &Path,
    source: SkillSource,
    verified: bool,
) -> Result<CompiledSkillArtifact> {
    let artifact = compile_skill_file(path, source, verified)?;
    let skill_root = output_root.join(sanitize_name(&artifact.manifest.name));
    fs::create_dir_all(&skill_root).map_err(|error| {
        Error::Internal(format!(
            "Failed to create compiled skill directory {}: {}",
            skill_root.display(),
            error
        ))
    })?;
    write_json(
        skill_root.join("compiled_manifest.json"),
        &artifact.manifest,
    )?;
    write_json(skill_root.join("help_index.json"), &artifact.help_index)?;
    write_json(skill_root.join("mcp_schema.json"), &artifact.mcp_schema)?;
    write_json(skill_root.join("cli_schema.json"), &artifact.cli_schema)?;
    write_json(skill_root.join("scan_report.json"), &artifact.scan_report)?;
    Ok(artifact)
}

pub fn load_compiled_artifact(output_root: &Path, name: &str) -> Result<CompiledSkillArtifact> {
    let skill_root = output_root.join(sanitize_name(name));
    Ok(CompiledSkillArtifact {
        manifest: read_json(skill_root.join("compiled_manifest.json"))?,
        help_index: read_json(skill_root.join("help_index.json"))?,
        mcp_schema: read_json(skill_root.join("mcp_schema.json"))?,
        cli_schema: read_json(skill_root.join("cli_schema.json"))?,
        scan_report: read_json(skill_root.join("scan_report.json"))?,
    })
}

pub fn list_compiled_manifests(output_root: &Path) -> Result<Vec<CompiledSkillManifest>> {
    if !output_root.exists() {
        return Ok(Vec::new());
    }

    let mut manifests: Vec<CompiledSkillManifest> = Vec::new();
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
        let path = entry.path().join("compiled_manifest.json");
        if path.exists() {
            manifests.push(read_json(path)?);
        }
    }
    manifests.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(manifests)
}

pub fn remove_compiled_artifact(output_root: &Path, name: &str) -> Result<()> {
    let skill_root = output_root.join(sanitize_name(name));
    if skill_root.exists() {
        fs::remove_dir_all(&skill_root).map_err(|error| {
            Error::Internal(format!(
                "Failed to remove compiled skill directory {}: {}",
                skill_root.display(),
                error
            ))
        })?;
    }
    Ok(())
}

fn parse_skill_document(path: &Path, content: &str) -> ParsedSkillDocument {
    let (frontmatter, body) = split_frontmatter(content);
    let fields = parse_frontmatter(frontmatter.unwrap_or_default());
    let title = body
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .unwrap_or_else(|| {
            path.parent()
                .and_then(|parent| parent.file_name())
                .and_then(|name| name.to_str())
                .unwrap_or("unknown")
        });
    let description =
        field_string(&fields, &["description"]).unwrap_or_else(|| first_body_paragraph(body));
    let version = field_string(&fields, &["version"]).unwrap_or_else(|| "0.1.0".to_string());
    let author = field_string(&fields, &["author"]);
    let argument_hint = field_string(&fields, &["argument-hint", "argument_hint"]);
    let capabilities = field_list(&fields, &["capabilities"]);
    let allowed_tools = field_list(&fields, &["allowed-tools", "allowed_tools"]);
    let name = field_string(&fields, &["name"]).unwrap_or_else(|| title.to_string());

    ParsedSkillDocument {
        name,
        description,
        version,
        author,
        argument_hint,
        capabilities,
        allowed_tools,
        body: body.trim().to_string(),
    }
}

fn split_frontmatter(content: &str) -> (Option<&str>, &str) {
    if !content.starts_with("---\n") {
        return (None, content);
    }

    let rest = &content[4..];
    if let Some(end) = rest.find("\n---\n") {
        let frontmatter = &rest[..end];
        let body = &rest[end + 5..];
        (Some(frontmatter), body)
    } else {
        (None, content)
    }
}

fn parse_frontmatter(frontmatter: &str) -> BTreeMap<String, Vec<String>> {
    let mut fields: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current_list_key: Option<String> = None;
    for raw_line in frontmatter.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(item) = trimmed.strip_prefix("- ")
            && let Some(key) = current_list_key.as_ref()
        {
            fields
                .entry(key.clone())
                .or_default()
                .push(item.trim_matches('"').to_string());
            continue;
        }

        current_list_key = None;
        if let Some((key, value)) = trimmed.split_once(':') {
            let normalized = key.trim().to_lowercase();
            let clean = value.trim().trim_matches('"');
            if clean.is_empty() {
                fields.entry(normalized.clone()).or_default();
                current_list_key = Some(normalized);
            } else {
                let values = clean
                    .split(',')
                    .map(|entry| entry.trim().trim_matches('"'))
                    .filter(|entry| !entry.is_empty())
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                fields.insert(normalized, values);
            }
        }
    }
    fields
}

fn field_string(fields: &BTreeMap<String, Vec<String>>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| fields.get(*key).and_then(|values| values.first()).cloned())
}

fn field_list(fields: &BTreeMap<String, Vec<String>>, keys: &[&str]) -> Vec<String> {
    keys.iter()
        .find_map(|key| fields.get(*key).cloned())
        .unwrap_or_default()
}

fn first_body_paragraph(body: &str) -> String {
    let mut paragraph = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        paragraph.push(trimmed);
    }
    paragraph.join(" ").trim().to_string()
}

fn excerpt(body: &str, max_chars: usize) -> String {
    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= max_chars {
        compact
    } else {
        let clipped = compact
            .chars()
            .take(max_chars.saturating_sub(1))
            .collect::<String>();
        format!("{clipped}...")
    }
}

fn collect_examples(body: &str) -> Vec<String> {
    let mut examples = Vec::new();
    let mut in_block = false;
    let mut block = Vec::new();
    for line in body.lines() {
        if line.trim_start().starts_with("```") {
            if in_block {
                let joined = block.join("\n").trim().to_string();
                if !joined.is_empty() {
                    examples.push(joined);
                }
                block.clear();
                in_block = false;
            } else {
                in_block = true;
            }
            continue;
        }
        if in_block {
            block.push(line.to_string());
        }
        if examples.len() == 3 {
            break;
        }
    }
    examples
}

fn scan_skill(
    content: &str,
    scripts: &[String],
    allowed_tools: &[String],
    capabilities: &[String],
    skill_root: &Path,
) -> Vec<CompiledSkillScanFinding> {
    let mut findings = Vec::new();

    if content.contains("ignore previous instructions")
        || content.contains("ignore all previous instructions")
        || content.contains("disregard prior instructions")
    {
        findings.push(finding(
            FindingSeverity::Warning,
            "prompt_injection_pattern",
            "Skill body contains prompt-injection style language that should be reviewed.",
        ));
    }

    if content.chars().any(is_hidden_unicode) {
        findings.push(finding(
            FindingSeverity::Warning,
            "hidden_unicode",
            "Skill contains hidden or bidi Unicode characters.",
        ));
    }

    if content.contains("AKIA") || content.contains("ghp_") || content.contains("BEGIN PRIVATE KEY")
    {
        findings.push(finding(
            FindingSeverity::Error,
            "possible_secret",
            "Skill content appears to contain credential-like material.",
        ));
    }

    if allowed_tools.iter().any(|tool| tool.trim() == "*") {
        findings.push(finding(
            FindingSeverity::Error,
            "broad_tool_permission",
            "Skill declares wildcard tool permissions.",
        ));
    }

    for capability in capabilities {
        if matches!(
            capability.as_str(),
            "shell_exec" | "network_access" | "file_write" | "database_access"
        ) {
            findings.push(finding(
                FindingSeverity::Info,
                "sensitive_capability",
                &format!("Skill requests sensitive capability `{}`.", capability),
            ));
        }
    }

    for script in scripts {
        let script_path = skill_root.join(script);
        let script_content = fs::read_to_string(&script_path).unwrap_or_default();
        for (needle, severity, code, message) in [
            (
                "rm -rf",
                FindingSeverity::Critical,
                "dangerous_rm",
                "Script contains `rm -rf` and requires explicit review.",
            ),
            (
                "curl | bash",
                FindingSeverity::Critical,
                "curl_bash",
                "Script pipes `curl` into `bash` and is blocked pending review.",
            ),
            (
                "chmod 777",
                FindingSeverity::Error,
                "chmod_777",
                "Script uses `chmod 777`, which is overly broad.",
            ),
            (
                "eval ",
                FindingSeverity::Warning,
                "eval_usage",
                "Script uses `eval`, which should be reviewed.",
            ),
        ] {
            if script_content.contains(needle) {
                findings.push(finding(
                    severity,
                    code,
                    &format!("{} ({})", message, script),
                ));
            }
        }
    }

    findings
}

fn derive_status(
    source: SkillSource,
    verified: bool,
    findings: &[CompiledSkillScanFinding],
) -> CompiledSkillStatus {
    if findings
        .iter()
        .any(|finding| matches!(finding.severity, FindingSeverity::Critical))
    {
        return CompiledSkillStatus::Blocked;
    }

    if verified || matches!(source, SkillSource::Workspace | SkillSource::Bundled) {
        return CompiledSkillStatus::Approved;
    }

    if findings.is_empty() {
        CompiledSkillStatus::Compiled
    } else {
        CompiledSkillStatus::Scanned
    }
}

fn verification_policy(source: SkillSource, verified: bool, capabilities: &[String]) -> String {
    let sensitive = declared_sensitive_capability_names(capabilities).unwrap_or_default();
    if verified {
        return "verified".to_string();
    }
    if matches!(source, SkillSource::Workspace | SkillSource::Bundled) {
        return "trusted-local".to_string();
    }
    if sensitive.is_empty() {
        "review-before-enable".to_string()
    } else {
        format!("review-sensitive-capabilities: {}", sensitive.join(", "))
    }
}

fn list_relative_files(root: &Path, child: &str) -> Vec<String> {
    let base = root.join(child);
    if !base.exists() {
        return Vec::new();
    }

    let mut files = Vec::new();
    let mut stack = vec![base.clone()];
    while let Some(path) = stack.pop() {
        let Ok(entries) = fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                stack.push(entry_path);
            } else if let Ok(relative) = entry_path.strip_prefix(root) {
                files.push(relative.display().to_string());
            }
        }
    }
    files.sort();
    files
}

fn write_json<T: Serialize>(path: PathBuf, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        Error::Internal(format!("Failed to serialize {}: {}", path.display(), error))
    })?;
    fs::write(&path, bytes).map_err(|error| {
        Error::Internal(format!("Failed to write {}: {}", path.display(), error))
    })?;
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Result<T> {
    let bytes = fs::read(&path).map_err(|error| {
        Error::Internal(format!("Failed to read {}: {}", path.display(), error))
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| Error::Internal(format!("Failed to parse {}: {}", path.display(), error)))
}

fn finding(severity: FindingSeverity, code: &str, message: &str) -> CompiledSkillScanFinding {
    CompiledSkillScanFinding {
        severity,
        code: code.to_string(),
        message: message.to_string(),
    }
}

fn hash_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{:02x}", byte)).collect()
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

fn is_hidden_unicode(ch: char) -> bool {
    matches!(
        ch,
        '\u{200B}'
            | '\u{200C}'
            | '\u{200D}'
            | '\u{2060}'
            | '\u{FEFF}'
            | '\u{202A}'
            | '\u{202B}'
            | '\u{202D}'
            | '\u{202E}'
            | '\u{2066}'
            | '\u{2067}'
            | '\u{2068}'
            | '\u{2069}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn compiles_skill_into_cached_artifacts() {
        let temp = TempDir::new().unwrap();
        let skill_dir = temp.path().join("my-skill");
        fs::create_dir_all(skill_dir.join("scripts")).unwrap();
        fs::create_dir_all(skill_dir.join("references")).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            r#"---
name: my-skill
description: "Review repository hygiene"
version: "1.2.0"
argument-hint: "<repo> [--fix]"
allowed-tools:
  - Bash
  - Read
capabilities:
  - shell_exec
---

# My Skill

Use this skill to review repository hygiene.

```bash
openrustclaw skills invoke my-skill repo --fix
```
"#,
        )
        .unwrap();
        fs::write(
            skill_dir.join("scripts").join("review.sh"),
            "#!/usr/bin/env bash\necho ok\n",
        )
        .unwrap();
        fs::write(skill_dir.join("references").join("guide.md"), "guide\n").unwrap();

        let output_root = temp.path().join(".claw/skills/compiled");
        let artifact = compile_skill_to_dir(
            &skill_dir.join("SKILL.md"),
            &output_root,
            SkillSource::Workspace,
            true,
        )
        .unwrap();

        assert_eq!(artifact.manifest.name, "my-skill");
        assert_eq!(artifact.manifest.status, CompiledSkillStatus::Approved);
        assert_eq!(
            artifact.help_index.argument_hint.as_deref(),
            Some("<repo> [--fix]")
        );
        assert!(
            artifact
                .mcp_schema
                .tool_names
                .iter()
                .any(|name| name.contains("review.sh"))
        );
        assert!(
            output_root
                .join("my-skill")
                .join("compiled_manifest.json")
                .exists()
        );
        assert!(
            output_root
                .join("my-skill")
                .join("help_index.json")
                .exists()
        );
    }

    #[test]
    fn blocks_critical_scripts() {
        let temp = TempDir::new().unwrap();
        let skill_dir = temp.path().join("blocked");
        fs::create_dir_all(skill_dir.join("scripts")).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# blocked\n\nDanger\n").unwrap();
        fs::write(skill_dir.join("scripts").join("oops.sh"), "rm -rf /\n").unwrap();

        let artifact =
            compile_skill_file(&skill_dir.join("SKILL.md"), SkillSource::Marketplace, false)
                .unwrap();
        assert_eq!(artifact.manifest.status, CompiledSkillStatus::Blocked);
        assert!(
            artifact
                .scan_report
                .findings
                .iter()
                .any(|finding| matches!(finding.severity, FindingSeverity::Critical))
        );
    }
}

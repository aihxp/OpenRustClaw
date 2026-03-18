//! File-backed task manifest support for the durable scheduler.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use openrustclaw_core::error::SchedulerError;

use crate::Result;

pub const DEFAULT_TASKS_DIR: &str = ".claw/tasks";

fn default_manifest_version() -> u32 {
    1
}

fn default_priority() -> i64 {
    100
}

fn default_true() -> bool {
    true
}

fn default_timezone() -> String {
    "UTC".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskManifest {
    #[serde(default = "default_manifest_version")]
    pub version: u32,
    pub task: TaskSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    pub id: Option<String>,
    pub name: String,
    pub workflow: String,
    pub description: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default = "default_priority")]
    pub priority: i64,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub max_retries: Option<u32>,
    #[serde(default)]
    pub disabled_until: Option<String>,
    pub trigger: TaskTrigger,
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub metadata: Value,
    #[serde(default)]
    pub delivery_policy: Option<Value>,
    #[serde(default)]
    pub hook_policy: Option<Value>,
    #[serde(default)]
    pub routing: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskTrigger {
    Interval { every_seconds: u64 },
    Absolute { at: String },
    Event { event_name: String },
    Dependency { depends_on: Vec<String> },
}

#[derive(Debug, Clone)]
pub struct LoadedTaskManifest {
    pub path: PathBuf,
    pub manifest: TaskManifest,
    pub raw: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskManifestRecord {
    pub job_id: String,
    pub manifest_path: PathBuf,
    pub manifest_hash: String,
    pub version: u32,
    pub origin: String,
}

pub fn tasks_dir_for_root(root: impl AsRef<Path>) -> PathBuf {
    root.as_ref().join(DEFAULT_TASKS_DIR)
}

pub fn load_task_manifest(path: impl AsRef<Path>) -> Result<LoadedTaskManifest> {
    let path = path.as_ref().to_path_buf();
    let raw = fs::read_to_string(&path).map_err(|e| {
        SchedulerError::WorkflowFailed(format!(
            "failed to read task manifest '{}': {e}",
            path.display()
        ))
    })?;
    let manifest = serde_yaml::from_str::<TaskManifest>(&raw).map_err(|e| {
        SchedulerError::WorkflowFailed(format!("invalid task manifest '{}': {e}", path.display()))
    })?;
    validate_manifest(&manifest, &path)?;

    Ok(LoadedTaskManifest {
        path,
        hash: digest_manifest(&raw),
        manifest,
        raw,
    })
}

fn validate_manifest(manifest: &TaskManifest, path: &Path) -> Result<()> {
    if manifest.task.name.trim().is_empty() {
        return Err(SchedulerError::WorkflowFailed(format!(
            "task manifest '{}' is missing a task.name",
            path.display()
        )));
    }
    if manifest.task.workflow.trim().is_empty() {
        return Err(SchedulerError::WorkflowFailed(format!(
            "task manifest '{}' is missing a task.workflow",
            path.display()
        )));
    }

    if let Some(until) = &manifest.task.disabled_until {
        DateTime::parse_from_rfc3339(until).map_err(|e| {
            SchedulerError::WorkflowFailed(format!(
                "task manifest '{}' has invalid disabled_until '{}': {e}",
                path.display(),
                until
            ))
        })?;
    }

    match &manifest.task.trigger {
        TaskTrigger::Interval { every_seconds } if *every_seconds == 0 => {
            return Err(SchedulerError::WorkflowFailed(format!(
                "task manifest '{}' interval must be > 0 seconds",
                path.display()
            )));
        }
        TaskTrigger::Absolute { at } => {
            DateTime::parse_from_rfc3339(at).map_err(|e| {
                SchedulerError::WorkflowFailed(format!(
                    "task manifest '{}' has invalid trigger.at '{}': {e}",
                    path.display(),
                    at
                ))
            })?;
        }
        TaskTrigger::Dependency { depends_on } if depends_on.is_empty() => {
            return Err(SchedulerError::WorkflowFailed(format!(
                "task manifest '{}' dependency trigger requires depends_on entries",
                path.display()
            )));
        }
        TaskTrigger::Event { event_name } if event_name.trim().is_empty() => {
            return Err(SchedulerError::WorkflowFailed(format!(
                "task manifest '{}' event trigger requires event_name",
                path.display()
            )));
        }
        _ => {}
    }

    Ok(())
}

pub fn manifest_job_id(path: impl AsRef<Path>, manifest: &TaskManifest) -> String {
    if let Some(id) = &manifest.task.id {
        return id.clone();
    }

    let path = path.as_ref();
    let mut hasher = Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    let digest = encode_hex(&hasher.finalize());
    let slug = slugify(&manifest.task.name);
    format!("task-{slug}-{}", &digest[..12])
}

pub fn render_task_manifest(spec: &TaskSpec) -> Result<String> {
    serde_yaml::to_string(&TaskManifest {
        version: 1,
        task: spec.clone(),
    })
    .map_err(|e| SchedulerError::WorkflowFailed(format!("failed to render task manifest: {e}")))
}

fn slugify(input: &str) -> String {
    let mut slug = String::with_capacity(input.len());
    let mut last_dash = false;
    for ch in input.chars() {
        let next = if ch.is_ascii_alphanumeric() {
            last_dash = false;
            ch.to_ascii_lowercase()
        } else if !last_dash {
            last_dash = true;
            '-'
        } else {
            continue;
        };
        slug.push(next);
    }
    slug.trim_matches('-').to_string()
}

fn digest_manifest(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    encode_hex(&hasher.finalize())
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

pub fn default_export_path(root: impl AsRef<Path>, name: &str) -> PathBuf {
    tasks_dir_for_root(root).join(format!("{}.yaml", slugify(name)))
}

pub fn disabled_until_utc(value: Option<&str>) -> Result<Option<DateTime<Utc>>> {
    value
        .map(|raw| {
            DateTime::parse_from_rfc3339(raw)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| {
                    SchedulerError::WorkflowFailed(format!(
                        "invalid disabled_until timestamp '{}': {e}",
                        raw
                    ))
                })
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn manifest_job_id_prefers_explicit_id() {
        let manifest = TaskManifest {
            version: 1,
            task: TaskSpec {
                id: Some("task-123".to_string()),
                name: "Daily Digest".to_string(),
                workflow: "reminder".to_string(),
                description: None,
                notes: None,
                priority: 100,
                enabled: true,
                timezone: "UTC".to_string(),
                owner: None,
                tags: vec![],
                max_retries: None,
                disabled_until: None,
                trigger: TaskTrigger::Interval { every_seconds: 60 },
                payload: serde_json::json!({}),
                metadata: serde_json::json!({}),
                delivery_policy: None,
                hook_policy: None,
                routing: None,
            },
        };

        assert_eq!(manifest_job_id("x.yaml", &manifest), "task-123");
    }

    #[test]
    fn load_task_manifest_round_trips_yaml() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("digest.yaml");
        fs::write(
            &path,
            r#"
version: 1
task:
  name: Daily Digest
  workflow: reminder
  priority: 50
  trigger:
    type: interval
    every_seconds: 300
  payload:
    message: hello
"#,
        )
        .unwrap();

        let loaded = load_task_manifest(&path).unwrap();
        assert_eq!(loaded.manifest.task.name, "Daily Digest");
        assert_eq!(loaded.manifest.task.priority, 50);
        assert!(!loaded.hash.is_empty());
    }
}

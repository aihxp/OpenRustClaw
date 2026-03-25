use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::{Context, Result};
use chrono::Utc;
use openrustclaw_observability::{Env, RequestIdLayer};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::sync::broadcast;
use tracing::{Event, Subscriber};
use tracing_subscriber::{
    EnvFilter,
    layer::SubscriberExt,
    layer::{Context as LayerContext, Layer},
    util::SubscriberInitExt,
};

pub const DEFAULT_RUNTIME_LOG_PATH: &str = ".claw/control/runtime.log";
pub const DEFAULT_RUNTIME_LOG_ARCHIVE_ROOT: &str = ".claw/control/runtime-log-archives";
const DEFAULT_RECENT_CAPACITY: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeLogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub fields: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeLogRotationSummary {
    pub rotated_at: String,
    pub log_path: String,
    pub archive_root: String,
    pub rotated: bool,
    pub previous_size_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_path: Option<String>,
    pub retained_archives: Vec<String>,
    pub removed_archives: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone)]
struct RuntimeLogStore {
    file: Arc<Mutex<File>>,
    recent: Arc<Mutex<VecDeque<RuntimeLogEntry>>>,
    tx: broadcast::Sender<RuntimeLogEntry>,
    capacity: usize,
}

impl RuntimeLogStore {
    fn append(&self, entry: RuntimeLogEntry) {
        if let Ok(mut file) = self.file.lock()
            && let Ok(line) = serde_json::to_string(&entry)
        {
            let _ = writeln!(file, "{line}");
            let _ = file.flush();
        }

        if let Ok(mut recent) = self.recent.lock() {
            recent.push_back(entry.clone());
            while recent.len() > self.capacity {
                recent.pop_front();
            }
        }

        let _ = self.tx.send(entry);
    }

    fn recent(&self, limit: usize) -> Vec<RuntimeLogEntry> {
        if let Ok(recent) = self.recent.lock() {
            let take = limit.max(1).min(recent.len());
            recent
                .iter()
                .skip(recent.len().saturating_sub(take))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    fn subscribe(&self) -> broadcast::Receiver<RuntimeLogEntry> {
        self.tx.subscribe()
    }
}

static STORE: OnceLock<RuntimeLogStore> = OnceLock::new();

struct RuntimeLogLayer {
    store: RuntimeLogStore,
}

impl<S> Layer<S> for RuntimeLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: LayerContext<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = JsonFieldVisitor::default();
        event.record(&mut visitor);

        let mut fields = visitor.fields;
        let message = fields
            .remove("message")
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_else(|| {
                if fields.is_empty() {
                    metadata.name().to_string()
                } else {
                    serde_json::to_string(&fields).unwrap_or_else(|_| metadata.name().to_string())
                }
            });

        self.store.append(RuntimeLogEntry {
            timestamp: Utc::now().to_rfc3339(),
            level: metadata.level().to_string(),
            target: metadata.target().to_string(),
            module_path: metadata.module_path().map(str::to_string),
            file: metadata.file().map(str::to_string),
            line: metadata.line(),
            message,
            fields,
        });
    }
}

#[derive(Default)]
struct JsonFieldVisitor {
    fields: Map<String, Value>,
}

impl tracing::field::Visit for JsonFieldVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields
            .insert(field.name().to_string(), Value::String(value.to_string()));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), Value::Bool(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), Value::Number(value.into()));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), Value::Number(value.into()));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        let number =
            serde_json::Number::from_f64(value).unwrap_or_else(|| serde_json::Number::from(0));
        self.fields
            .insert(field.name().to_string(), Value::Number(number));
    }

    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        self.fields
            .insert(field.name().to_string(), Value::String(value.to_string()));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields.insert(
            field.name().to_string(),
            Value::String(format!("{value:?}")),
        );
    }
}

fn default_env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,openrustclaw=debug"))
}

pub fn runtime_log_path_for(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root.as_ref().join(DEFAULT_RUNTIME_LOG_PATH)
}

pub fn runtime_log_archive_root_for(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root
        .as_ref()
        .join(DEFAULT_RUNTIME_LOG_ARCHIVE_ROOT)
}

pub fn init_runtime_logging(workspace_root: &Path) -> Result<()> {
    if STORE.get().is_some() {
        return Ok(());
    }

    let path = runtime_log_path_for(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create '{}'", parent.display()))?;
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("Failed to open runtime log file '{}'", path.display()))?;
    let (tx, _) = broadcast::channel(256);
    let store = RuntimeLogStore {
        file: Arc::new(Mutex::new(file)),
        recent: Arc::new(Mutex::new(VecDeque::with_capacity(DEFAULT_RECENT_CAPACITY))),
        tx,
        capacity: DEFAULT_RECENT_CAPACITY,
    };

    let init_result = match Env::detect() {
        Env::Production => tracing_subscriber::registry()
            .with(default_env_filter())
            .with(RequestIdLayer)
            .with(RuntimeLogLayer {
                store: store.clone(),
            })
            .with(tracing_subscriber::fmt::layer().json())
            .try_init(),
        Env::Development => tracing_subscriber::registry()
            .with(default_env_filter())
            .with(RequestIdLayer)
            .with(RuntimeLogLayer {
                store: store.clone(),
            })
            .with(tracing_subscriber::fmt::layer().pretty())
            .try_init(),
        Env::Test => tracing_subscriber::registry()
            .with(default_env_filter())
            .with(RuntimeLogLayer {
                store: store.clone(),
            })
            .with(tracing_subscriber::fmt::layer().compact())
            .try_init(),
    };

    if let Err(error) = init_result {
        eprintln!(
            "OpenRustClaw warning: tracing subscriber was already initialized; runtime log capture may be unavailable: {error}"
        );
    }

    let _ = STORE.set(store);
    Ok(())
}

pub fn recent_runtime_logs(workspace_root: &Path, limit: usize) -> Result<Vec<RuntimeLogEntry>> {
    if let Some(store) = STORE.get() {
        let recent = store.recent(limit);
        if !recent.is_empty() {
            return Ok(recent);
        }
    }
    read_recent_logs(workspace_root, limit)
}

pub fn rotate_runtime_logs(
    workspace_root: &Path,
    keep: usize,
    max_bytes: Option<u64>,
    force: bool,
) -> Result<RuntimeLogRotationSummary> {
    let log_path = runtime_log_path_for(workspace_root);
    let archive_root = runtime_log_archive_root_for(workspace_root);
    let rotated_at = Utc::now();
    let keep = keep.max(1);

    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create '{}'", parent.display()))?;
    }
    fs::create_dir_all(&archive_root)
        .with_context(|| format!("Failed to create '{}'", archive_root.display()))?;

    if !log_path.exists() {
        File::create(&log_path).with_context(|| {
            format!("Failed to create runtime log file '{}'", log_path.display())
        })?;
    }

    let previous_size_bytes = fs::metadata(&log_path)
        .with_context(|| format!("Failed to stat '{}'", log_path.display()))?
        .len();
    let should_rotate = force
        || max_bytes
            .map(|limit| previous_size_bytes >= limit)
            .unwrap_or(previous_size_bytes > 0);

    if !should_rotate {
        let retained_archives = list_runtime_log_archives(&archive_root)?;
        return Ok(RuntimeLogRotationSummary {
            rotated_at: rotated_at.to_rfc3339(),
            log_path: log_path.display().to_string(),
            archive_root: archive_root.display().to_string(),
            rotated: false,
            previous_size_bytes,
            archive_path: None,
            retained_archives,
            removed_archives: Vec::new(),
            reason: Some(format!(
                "runtime log is below the configured rotation threshold{}",
                max_bytes
                    .map(|value| format!(" ({value} bytes)"))
                    .unwrap_or_default()
            )),
        });
    }

    let archive_path = archive_root.join(format!(
        "runtime-{}.log",
        rotated_at.format("%Y%m%d%H%M%S%f")
    ));
    fs::copy(&log_path, &archive_path).with_context(|| {
        format!(
            "Failed to archive '{}' -> '{}'",
            log_path.display(),
            archive_path.display()
        )
    })?;
    OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&log_path)
        .with_context(|| format!("Failed to truncate '{}'", log_path.display()))?;

    let (retained_archives, removed_archives) = prune_runtime_log_archives(&archive_root, keep)?;

    Ok(RuntimeLogRotationSummary {
        rotated_at: rotated_at.to_rfc3339(),
        log_path: log_path.display().to_string(),
        archive_root: archive_root.display().to_string(),
        rotated: true,
        previous_size_bytes,
        archive_path: Some(archive_path.display().to_string()),
        retained_archives,
        removed_archives,
        reason: None,
    })
}

pub fn read_recent_logs(workspace_root: &Path, limit: usize) -> Result<Vec<RuntimeLogEntry>> {
    let path = runtime_log_path_for(workspace_root);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(&path)
        .with_context(|| format!("Failed to open runtime log file '{}'", path.display()))?;
    let mut recent = VecDeque::new();
    for line in BufReader::new(file).lines() {
        let line = match line {
            Ok(value) if !value.trim().is_empty() => value,
            Ok(_) => continue,
            Err(_) => continue,
        };
        let Ok(entry) = serde_json::from_str::<RuntimeLogEntry>(&line) else {
            continue;
        };
        recent.push_back(entry);
        while recent.len() > limit.max(1) {
            recent.pop_front();
        }
    }
    Ok(recent.into_iter().collect())
}

pub fn subscribe_runtime_logs() -> Option<broadcast::Receiver<RuntimeLogEntry>> {
    STORE.get().map(RuntimeLogStore::subscribe)
}

fn list_runtime_log_archives(archive_root: &Path) -> Result<Vec<String>> {
    if !archive_root.exists() {
        return Ok(Vec::new());
    }

    let mut archives = fs::read_dir(archive_root)
        .with_context(|| format!("Failed to read '{}'", archive_root.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    archives.sort();
    Ok(archives
        .into_iter()
        .map(|path| path.display().to_string())
        .collect())
}

fn prune_runtime_log_archives(
    archive_root: &Path,
    keep: usize,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut archives = if archive_root.exists() {
        fs::read_dir(archive_root)
            .with_context(|| format!("Failed to read '{}'", archive_root.display()))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    archives.sort();

    let split_at = archives.len().saturating_sub(keep);
    let stale_archives = archives.drain(..split_at).collect::<Vec<_>>();
    let mut removed = Vec::new();
    for archive in stale_archives {
        fs::remove_file(&archive)
            .with_context(|| format!("Failed to remove '{}'", archive.display()))?;
        removed.push(archive.display().to_string());
    }

    let retained = archives
        .into_iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    Ok((retained, removed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_recent_logs_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = runtime_log_path_for(dir.path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();

        let entries = vec![
            RuntimeLogEntry {
                timestamp: "2026-03-19T00:00:00Z".to_string(),
                level: "INFO".to_string(),
                target: "openrustclaw::test".to_string(),
                module_path: None,
                file: None,
                line: None,
                message: "one".to_string(),
                fields: Map::new(),
            },
            RuntimeLogEntry {
                timestamp: "2026-03-19T00:00:01Z".to_string(),
                level: "WARN".to_string(),
                target: "openrustclaw::test".to_string(),
                module_path: None,
                file: None,
                line: None,
                message: "two".to_string(),
                fields: Map::new(),
            },
        ];

        let mut file = File::create(&path).unwrap();
        for entry in &entries {
            writeln!(file, "{}", serde_json::to_string(entry).unwrap()).unwrap();
        }

        let recent = read_recent_logs(dir.path(), 1).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].message, "two");
    }

    #[test]
    fn rotates_runtime_logs_and_prunes_old_archives() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = runtime_log_path_for(dir.path());
        fs::create_dir_all(log_path.parent().unwrap()).unwrap();
        fs::write(&log_path, "{\"message\":\"one\"}\n").unwrap();

        let first = rotate_runtime_logs(dir.path(), 2, Some(1), false).unwrap();
        assert!(first.rotated);
        assert!(first.archive_path.is_some());
        assert_eq!(fs::metadata(&log_path).unwrap().len(), 0);

        fs::write(&log_path, "{\"message\":\"two\"}\n").unwrap();
        let second = rotate_runtime_logs(dir.path(), 1, Some(1), false).unwrap();
        assert!(second.rotated);
        assert_eq!(second.retained_archives.len(), 1);
        assert_eq!(second.removed_archives.len(), 1);
    }

    #[test]
    fn skips_rotation_when_below_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = runtime_log_path_for(dir.path());
        fs::create_dir_all(log_path.parent().unwrap()).unwrap();
        fs::write(&log_path, "{\"message\":\"small\"}\n").unwrap();

        let summary = rotate_runtime_logs(dir.path(), 2, Some(10_000), false).unwrap();
        assert!(!summary.rotated);
        assert_eq!(summary.archive_path, None);
        assert!(summary.reason.is_some());
        assert!(fs::metadata(&log_path).unwrap().len() > 0);
    }
}

//! Cursor-specific tools for IDE integration.
//!
//! This module provides tools that enable the agent to interact with the Cursor IDE:
//! - Codebase tools: Search, read, edit, create, delete files
//! - Terminal tools: Run commands, read output
//! - Git tools: Status, diff, commit, branch management
//! - Linter tools: Run linters and formatters

use crate::error::{CursorError, Result};
use crate::types::{CursorTool, ToolContext};
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::warn;
use uuid::Uuid;

pub mod codebase;
pub mod git;
pub mod linter;
pub mod terminal;

pub use codebase::{
    CreateFileTool, DeleteFileTool, EditFileTool, ListFilesTool, ReadFileTool, SearchCodeTool,
};
pub use git::{GitBranchTool, GitCommitTool, GitDiffTool, GitStatusTool};
pub use linter::{FormatCodeTool, RunLinterTool};
pub use terminal::{ReadTerminalTool, RunCommandTool, TerminalManager};

const CODING_ARTIFACT_DIR: &str = ".claw/control/cursor-tool-runs";

/// A durable record for one Cursor coding-tool execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CursorToolExecutionArtifact {
    /// Stable artifact identifier.
    pub id: String,
    /// Tool name that produced the artifact.
    pub tool_name: String,
    /// Whether the tool execution succeeded.
    pub success: bool,
    /// RFC3339 timestamp when the artifact was recorded.
    pub created_at: String,
    /// Absolute filesystem path to the persisted artifact JSON.
    pub artifact_path: String,
    /// Target file path when the tool acted on a single file.
    pub target_path: Option<String>,
    /// Optional git diff preview for mutating coding actions.
    pub diff_preview: Option<String>,
    /// Original tool parameters.
    pub params: Value,
    /// Successful tool result payload, if any.
    pub result: Option<Value>,
    /// Error text for failed executions, if any.
    pub error: Option<String>,
}

/// Registry of available Cursor tools.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn CursorTool>>,
    context: Arc<RwLock<ToolContext>>,
}

impl ToolRegistry {
    /// Create a new tool registry with all default tools.
    pub fn new(context: ToolContext) -> Self {
        let terminal_manager = Arc::new(TerminalManager::new(context.config.terminal_timeout));
        let mut registry = Self {
            tools: HashMap::new(),
            context: Arc::new(RwLock::new(context)),
        };

        // Register codebase tools
        registry.register(Arc::new(SearchCodeTool));
        registry.register(Arc::new(ReadFileTool));
        registry.register(Arc::new(EditFileTool));
        registry.register(Arc::new(CreateFileTool));
        registry.register(Arc::new(DeleteFileTool));
        registry.register(Arc::new(ListFilesTool));

        // Register terminal tools
        registry.register(Arc::new(RunCommandTool::new(terminal_manager.clone())));
        registry.register(Arc::new(ReadTerminalTool::new(terminal_manager)));

        // Register git tools
        registry.register(Arc::new(GitStatusTool));
        registry.register(Arc::new(GitDiffTool));
        registry.register(Arc::new(GitCommitTool));
        registry.register(Arc::new(GitBranchTool));

        // Register linter tools
        registry.register(Arc::new(RunLinterTool));
        registry.register(Arc::new(FormatCodeTool));

        registry
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Arc<dyn CursorTool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn CursorTool>> {
        self.tools.get(name).cloned()
    }

    /// List all available tools.
    pub fn list_tools(&self) -> Vec<Arc<dyn CursorTool>> {
        self.tools.values().cloned().collect()
    }

    /// Get the tool context.
    pub fn context(&self) -> Arc<RwLock<ToolContext>> {
        self.context.clone()
    }

    /// Execute a tool by name with parameters.
    pub async fn execute(&self, name: &str, params: Value) -> Result<Value> {
        let tool = self
            .get(name)
            .ok_or_else(|| CursorError::ToolNotFound(name.to_string()))?;
        let context = self.context.read().await.clone();
        match tool.execute(params.clone(), &context).await {
            Ok(mut result) => {
                if let Some(artifact) = persist_execution_artifact(
                    &context.project_root,
                    name,
                    &params,
                    Some(&result),
                    None,
                )
                .await
                {
                    attach_artifact_metadata(&mut result, &artifact);
                }
                Ok(result)
            }
            Err(error) => {
                let _ = persist_execution_artifact(
                    &context.project_root,
                    name,
                    &params,
                    None,
                    Some(error.to_string()),
                )
                .await;
                Err(error)
            }
        }
    }

    /// Get all tool definitions for MCP/ACP registration.
    pub fn get_tool_definitions(&self) -> Vec<Value> {
        self.tools
            .values()
            .map(|tool| {
                serde_json::json!({
                    "name": tool.name(),
                    "description": tool.description(),
                    "inputSchema": tool.parameters_schema(),
                })
            })
            .collect()
    }
}

/// Return the workspace-local root directory for Cursor coding artifacts.
pub fn execution_artifact_root(project_root: &Path) -> PathBuf {
    project_root.join(CODING_ARTIFACT_DIR)
}

/// Load recent Cursor coding execution artifacts from the workspace index.
pub fn load_execution_artifacts(
    project_root: &Path,
    limit: usize,
) -> Result<Vec<CursorToolExecutionArtifact>> {
    let index_path = execution_artifact_root(project_root).join("index.jsonl");
    if !index_path.exists() {
        return Ok(Vec::new());
    }

    let file = OpenOptions::new()
        .read(true)
        .open(index_path)
        .map_err(|e| CursorError::FileOperation(e.to_string()))?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(|e| CursorError::FileOperation(e.to_string()))?;
        if line.trim().is_empty() {
            continue;
        }
        let entry = serde_json::from_str::<CursorToolExecutionArtifact>(&line)
            .map_err(|e| CursorError::Protocol(e.to_string()))?;
        entries.push(entry);
    }

    entries.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    entries.truncate(limit.max(1));
    Ok(entries)
}

fn attach_artifact_metadata(output: &mut Value, artifact: &CursorToolExecutionArtifact) {
    let metadata = serde_json::json!({
        "id": artifact.id,
        "path": artifact.artifact_path,
        "success": artifact.success,
        "target_path": artifact.target_path,
    });
    if let Some(object) = output.as_object_mut() {
        object.insert("_artifact".to_string(), metadata);
    }
}

async fn persist_execution_artifact(
    project_root: &Path,
    tool_name: &str,
    params: &Value,
    result: Option<&Value>,
    error: Option<String>,
) -> Option<CursorToolExecutionArtifact> {
    let artifact_root = execution_artifact_root(project_root);
    if let Err(problem) = fs::create_dir_all(&artifact_root) {
        warn!(path = %artifact_root.display(), error = %problem, "Failed to create cursor artifact root");
        return None;
    }

    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    let artifact_path = artifact_root.join(format!("{created_at}-{id}.json").replace(':', "-"));
    let target_path = result
        .and_then(|value| value.get("path"))
        .and_then(|value| value.as_str())
        .map(ToString::to_string);
    let diff_preview = capture_diff_preview(project_root, tool_name, result).await;
    let artifact = CursorToolExecutionArtifact {
        id,
        tool_name: tool_name.to_string(),
        success: error.is_none(),
        created_at,
        artifact_path: artifact_path.display().to_string(),
        target_path,
        diff_preview,
        params: params.clone(),
        result: result.cloned(),
        error,
    };

    let rendered = match serde_json::to_vec_pretty(&artifact) {
        Ok(rendered) => rendered,
        Err(problem) => {
            warn!(tool = %tool_name, error = %problem, "Failed to serialize cursor execution artifact");
            return None;
        }
    };
    if let Err(problem) = fs::write(&artifact_path, rendered) {
        warn!(tool = %tool_name, path = %artifact_path.display(), error = %problem, "Failed to write cursor execution artifact");
        return None;
    }

    let index_path = artifact_root.join("index.jsonl");
    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&index_path)
    {
        Ok(mut file) => {
            if serde_json::to_writer(&mut file, &artifact).is_err()
                || file.write_all(b"\n").is_err()
            {
                warn!(tool = %tool_name, path = %index_path.display(), "Failed to append cursor execution artifact index");
            }
        }
        Err(problem) => {
            warn!(tool = %tool_name, path = %index_path.display(), error = %problem, "Failed to open cursor execution artifact index");
        }
    }

    Some(artifact)
}

async fn capture_diff_preview(
    project_root: &Path,
    tool_name: &str,
    result: Option<&Value>,
) -> Option<String> {
    if tool_name == "git_diff" {
        return result
            .and_then(|value| value.get("diff"))
            .and_then(|value| value.as_str())
            .map(|value| truncate_preview(value, 2000));
    }

    if !matches!(tool_name, "edit_file" | "create_file" | "delete_file") {
        return None;
    }

    let path = result
        .and_then(|value| value.get("path"))
        .and_then(|value| value.as_str())?;
    let output = Command::new("git")
        .args(["diff", "--", path])
        .current_dir(project_root)
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let diff = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if diff.is_empty() {
        None
    } else {
        Some(truncate_preview(&diff, 2000))
    }
}

fn truncate_preview(value: &str, max_chars: usize) -> String {
    let truncated = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        format!("{truncated}…")
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CursorConfig;

    fn create_test_context() -> ToolContext {
        ToolContext::new(std::path::PathBuf::from("."), CursorConfig::default())
    }

    #[test]
    fn tool_registry_creation() {
        let registry = ToolRegistry::new(create_test_context());
        let tools = registry.list_tools();
        assert!(!tools.is_empty());
    }

    #[test]
    fn tool_lookup() {
        let registry = ToolRegistry::new(create_test_context());
        assert!(registry.get("search_code").is_some());
        assert!(registry.get("read_file").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn tool_definitions() {
        let registry = ToolRegistry::new(create_test_context());
        let definitions = registry.get_tool_definitions();
        assert!(!definitions.is_empty());

        for def in definitions {
            assert!(def.get("name").is_some());
            assert!(def.get("description").is_some());
            assert!(def.get("inputSchema").is_some());
        }
    }
}

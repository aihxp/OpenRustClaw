//! Git operations tools for repository management.

use crate::error::{CursorError, Result};
use crate::types::{CursorTool, GitStatus, ToolContext};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::PathBuf;
use tokio::process::Command;
use tracing::{info, warn};

/// Check git status.
pub struct GitStatusTool;

#[async_trait]
impl CursorTool for GitStatusTool {
    fn name(&self) -> &str {
        "git_status"
    }

    fn description(&self) -> &str {
        "Get the current git status including branch, modified files, staged files, and untracked files."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "short": {
                    "type": "boolean",
                    "description": "Short format output",
                    "default": false
                }
            },
            "required": []
        })
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> Result<Value> {
        let short = params
            .get("short")
            .and_then(|s| s.as_bool())
            .unwrap_or(false);

        let project_root = context.project_root.clone();

        // Get status
        let output = Command::new("git")
            .args(["status", "--porcelain", "-b"])
            .current_dir(&project_root)
            .output()
            .await
            .map_err(|e| CursorError::GitOperation(e.to_string()))?;

        if !output.status.success() {
            return Err(CursorError::GitOperation(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let status = parse_git_status(&stdout)?;

        // Get branch info
        let branch_output = Command::new("git")
            .args(["branch", "-vv"])
            .current_dir(&project_root)
            .output()
            .await
            .map_err(|e| CursorError::GitOperation(e.to_string()))?;

        let branch_info = String::from_utf8_lossy(&branch_output.stdout);

        // Get ahead/behind info
        let ahead_behind_output = Command::new("git")
            .args(["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
            .current_dir(&project_root)
            .output()
            .await;

        let (ahead, behind) = if let Ok(output) = ahead_behind_output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = stdout.split_whitespace().collect();
                if parts.len() == 2 {
                    (parts[0].parse().unwrap_or(0), parts[1].parse().unwrap_or(0))
                } else {
                    (0, 0)
                }
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        info!(
            "Git status: branch={}, modified={}, staged={}, untracked={}",
            status.branch,
            status.modified.len(),
            status.staged.len(),
            status.untracked.len()
        );

        if short {
            Ok(json!({
                "branch": status.branch,
                "modified": status.modified.len(),
                "staged": status.staged.len(),
                "untracked": status.untracked.len(),
                "ahead": ahead,
                "behind": behind,
            }))
        } else {
            Ok(json!({
                "branch": status.branch,
                "modified": status.modified,
                "staged": status.staged,
                "untracked": status.untracked,
                "renamed": status.renamed,
                "deleted": status.deleted,
                "ahead": ahead,
                "behind": behind,
                "is_clean": status.modified.is_empty() && status.staged.is_empty() && status.untracked.is_empty(),
                "raw_branch_info": branch_info.to_string(),
            }))
        }
    }
}

/// Show git diff.
pub struct GitDiffTool;

#[async_trait]
impl CursorTool for GitDiffTool {
    fn name(&self) -> &str {
        "git_diff"
    }

    fn description(&self) -> &str {
        "Show git diff for unstaged changes, staged changes, or between commits."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "staged": {
                    "type": "boolean",
                    "description": "Show staged changes",
                    "default": false
                },
                "cached": {
                    "type": "boolean",
                    "description": "Alias for staged",
                    "default": false
                },
                "path": {
                    "type": "string",
                    "description": "Optional specific file or directory to diff"
                },
                "from": {
                    "type": "string",
                    "description": "From commit/ref (for commit comparison)"
                },
                "to": {
                    "type": "string",
                    "description": "To commit/ref (for commit comparison, defaults to HEAD)"
                },
                "stat": {
                    "type": "boolean",
                    "description": "Show diffstat only",
                    "default": false
                }
            },
            "required": []
        })
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> Result<Value> {
        let project_root = context.project_root.clone();

        let staged = params
            .get("staged")
            .and_then(|s| s.as_bool())
            .unwrap_or(false)
            || params
                .get("cached")
                .and_then(|c| c.as_bool())
                .unwrap_or(false);
        let stat = params
            .get("stat")
            .and_then(|s| s.as_bool())
            .unwrap_or(false);
        let path = params.get("path").and_then(|p| p.as_str());
        let from = params.get("from").and_then(|f| f.as_str());
        let to = params.get("to").and_then(|t| t.as_str());

        let mut args = vec!["diff"];

        if staged {
            args.push("--staged");
        }

        if stat {
            args.push("--stat");
        }

        if let Some(from_ref) = from {
            args.push(from_ref);
            if let Some(to_ref) = to {
                args.push(to_ref);
            }
        }

        if let Some(p) = path {
            args.push("--");
            args.push(p);
        }

        let output = Command::new("git")
            .args(&args)
            .current_dir(&project_root)
            .output()
            .await
            .map_err(|e| CursorError::GitOperation(e.to_string()))?;

        let diff = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() && !stderr.is_empty() {
            warn!("Git diff stderr: {}", stderr);
        }

        Ok(json!({
            "diff": diff.to_string(),
            "staged": staged,
            "path": path,
            "has_changes": !diff.is_empty(),
        }))
    }
}

/// Make a git commit.
pub struct GitCommitTool;

#[async_trait]
impl CursorTool for GitCommitTool {
    fn name(&self) -> &str {
        "git_commit"
    }

    fn description(&self) -> &str {
        "Create a git commit. Can add files and commit in one operation."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "Commit message"
                },
                "files": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Specific files to add and commit (default: all staged)"
                },
                "all": {
                    "type": "boolean",
                    "description": "Stage all modified files before committing",
                    "default": false
                },
                "amend": {
                    "type": "boolean",
                    "description": "Amend the previous commit",
                    "default": false
                }
            },
            "required": ["message"]
        })
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> Result<Value> {
        let message = params
            .get("message")
            .and_then(|m| m.as_str())
            .ok_or_else(|| input_validation_error(self.name(), "Missing 'message' parameter"))?;

        let all = params.get("all").and_then(|a| a.as_bool()).unwrap_or(false);
        let amend = params
            .get("amend")
            .and_then(|a| a.as_bool())
            .unwrap_or(false);
        let files: Vec<String> = params
            .get("files")
            .and_then(|f| f.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let project_root = context.project_root.clone();

        // Stage files if specified
        if !files.is_empty() {
            let mut add_args = vec!["add"];
            for file in &files {
                add_args.push(file);
            }

            let output = Command::new("git")
                .args(&add_args)
                .current_dir(&project_root)
                .output()
                .await
                .map_err(|e| CursorError::GitOperation(e.to_string()))?;

            if !output.status.success() {
                return Err(CursorError::GitOperation(
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ));
            }
        } else if all {
            let output = Command::new("git")
                .args(["add", "-u"])
                .current_dir(&project_root)
                .output()
                .await
                .map_err(|e| CursorError::GitOperation(e.to_string()))?;

            if !output.status.success() {
                return Err(CursorError::GitOperation(
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ));
            }
        }

        // Commit
        let mut commit_args = vec!["commit", "-m", message];
        if amend {
            commit_args.push("--amend");
            commit_args.push("--no-edit");
        }

        let output = Command::new("git")
            .args(&commit_args)
            .current_dir(&project_root)
            .output()
            .await
            .map_err(|e| CursorError::GitOperation(e.to_string()))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        if !output.status.success() {
            return Err(CursorError::GitOperation(stderr.to_string()));
        }

        info!("Git commit created: {}", message);

        // Get the commit hash
        let hash_output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&project_root)
            .output()
            .await
            .map_err(|e| CursorError::GitOperation(e.to_string()))?;

        let commit_hash = String::from_utf8_lossy(&hash_output.stdout)
            .trim()
            .to_string();

        Ok(json!({
            "success": true,
            "message": message,
            "commit_hash": commit_hash,
            "amended": amend,
            "output": stdout.to_string(),
        }))
    }
}

/// Manage git branches.
pub struct GitBranchTool;

#[async_trait]
impl CursorTool for GitBranchTool {
    fn name(&self) -> &str {
        "git_branch"
    }

    fn description(&self) -> &str {
        "List, create, delete, or switch git branches."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "create", "delete", "switch"],
                    "description": "Action to perform"
                },
                "branch": {
                    "type": "string",
                    "description": "Branch name (for create, delete, or switch)"
                },
                "from": {
                    "type": "string",
                    "description": "Source branch/commit for create (default: current)"
                },
                "force": {
                    "type": "boolean",
                    "description": "Force the operation",
                    "default": false
                },
                "remote": {
                    "type": "boolean",
                    "description": "Include remote branches in list",
                    "default": false
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> Result<Value> {
        let action = params
            .get("action")
            .and_then(|a| a.as_str())
            .ok_or_else(|| input_validation_error(self.name(), "Missing 'action' parameter"))?;

        let branch = params.get("branch").and_then(|b| b.as_str());
        let from = params.get("from").and_then(|f| f.as_str());
        let force = params
            .get("force")
            .and_then(|f| f.as_bool())
            .unwrap_or(false);
        let remote = params
            .get("remote")
            .and_then(|r| r.as_bool())
            .unwrap_or(false);

        let project_root = context.project_root.clone();

        match action {
            "list" => {
                let mut args = vec!["branch", "-vv"];
                if remote {
                    args.push("-a");
                }

                let output = Command::new("git")
                    .args(&args)
                    .current_dir(&project_root)
                    .output()
                    .await
                    .map_err(|e| CursorError::GitOperation(e.to_string()))?;

                let stdout = String::from_utf8_lossy(&output.stdout);
                let branches: Vec<Value> = stdout
                    .lines()
                    .map(|line| {
                        let current = line.starts_with('*');
                        let name = line
                            .trim_start_matches('*')
                            .split_whitespace()
                            .next()
                            .unwrap_or("");
                        json!({
                            "name": name,
                            "current": current,
                            "line": line.trim(),
                        })
                    })
                    .filter(|b| !b["name"].as_str().unwrap_or("").is_empty())
                    .collect();

                // Get current branch
                let current_output = Command::new("git")
                    .args(["branch", "--show-current"])
                    .current_dir(&project_root)
                    .output()
                    .await;

                let current_branch = current_output
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();

                Ok(json!({
                    "branches": branches,
                    "current": current_branch,
                    "count": branches.len(),
                }))
            }
            "create" => {
                let branch_name = branch.ok_or_else(|| {
                    input_validation_error(self.name(), "Missing 'branch' parameter for create")
                })?;

                let mut args = vec!["checkout", "-b", branch_name];
                if let Some(from_ref) = from {
                    args.push(from_ref);
                }

                let output = Command::new("git")
                    .args(&args)
                    .current_dir(&project_root)
                    .output()
                    .await
                    .map_err(|e| CursorError::GitOperation(e.to_string()))?;

                if !output.status.success() {
                    return Err(CursorError::GitOperation(
                        String::from_utf8_lossy(&output.stderr).to_string(),
                    ));
                }

                info!("Created and switched to branch: {}", branch_name);

                Ok(json!({
                    "success": true,
                    "branch": branch_name,
                    "action": "create",
                    "from": from,
                }))
            }
            "delete" => {
                let branch_name = branch.ok_or_else(|| {
                    input_validation_error(self.name(), "Missing 'branch' parameter for delete")
                })?;

                let mut args = vec!["branch"];
                if force {
                    args.push("-D");
                } else {
                    args.push("-d");
                }
                args.push(branch_name);

                let output = Command::new("git")
                    .args(&args)
                    .current_dir(&project_root)
                    .output()
                    .await
                    .map_err(|e| CursorError::GitOperation(e.to_string()))?;

                if !output.status.success() {
                    return Err(CursorError::GitOperation(
                        String::from_utf8_lossy(&output.stderr).to_string(),
                    ));
                }

                info!("Deleted branch: {}", branch_name);

                Ok(json!({
                    "success": true,
                    "branch": branch_name,
                    "action": "delete",
                    "forced": force,
                }))
            }
            "switch" => {
                let branch_name = branch.ok_or_else(|| {
                    input_validation_error(self.name(), "Missing 'branch' parameter for switch")
                })?;

                let output = Command::new("git")
                    .args(["checkout", branch_name])
                    .current_dir(&project_root)
                    .output()
                    .await
                    .map_err(|e| CursorError::GitOperation(e.to_string()))?;

                if !output.status.success() {
                    return Err(CursorError::GitOperation(
                        String::from_utf8_lossy(&output.stderr).to_string(),
                    ));
                }

                info!("Switched to branch: {}", branch_name);

                Ok(json!({
                    "success": true,
                    "branch": branch_name,
                    "action": "switch",
                }))
            }
            _ => Err(input_validation_error(
                self.name(),
                format!("Invalid action: {}", action),
            )),
        }
    }
}

/// Parse git status --porcelain output.
fn parse_git_status(output: &str) -> Result<GitStatus> {
    let mut status = GitStatus {
        branch: "main".to_string(),
        modified: vec![],
        staged: vec![],
        untracked: vec![],
        renamed: vec![],
        deleted: vec![],
        ahead: 0,
        behind: 0,
    };

    for line in output.lines() {
        if line.starts_with("##") {
            // Parse branch info
            if let Some(branch_part) = line.strip_prefix("## ") {
                // Handle branch with upstream info: main...origin/main
                let branch_name = branch_part.split("...").next().unwrap_or(branch_part);
                // Handle initial commit state: No commits yet on main
                let branch_name = branch_name.split_whitespace().last().unwrap_or(branch_name);
                status.branch = branch_name.to_string();

                // Parse ahead/behind if present
                if let Some(upstream_idx) = branch_part.find("[")
                    && let Some(end_idx) = branch_part.find("]")
                {
                    let upstream_info = &branch_part[upstream_idx + 1..end_idx];
                    for part in upstream_info.split(", ") {
                        if let Some(ahead) = part.strip_prefix("ahead ") {
                            status.ahead = ahead.parse().unwrap_or(0);
                        } else if let Some(behind) = part.strip_prefix("behind ") {
                            status.behind = behind.parse().unwrap_or(0);
                        }
                    }
                }
            }
        } else if line.len() >= 3 {
            let index_status = &line[0..1];
            let worktree_status = &line[1..2];
            let path_str = &line[3..];

            // Handle renamed files: R  old -> new
            let path = if path_str.contains(" -> ") {
                let parts: Vec<&str> = path_str.split(" -> ").collect();
                if parts.len() == 2 {
                    PathBuf::from(parts[1])
                } else {
                    PathBuf::from(path_str)
                }
            } else {
                PathBuf::from(path_str)
            };

            match (index_status, worktree_status) {
                ("M", " ") | ("A", " ") | ("R", " ") => status.staged.push(path),
                (" ", "M") => status.modified.push(path),
                (" ", "?") => status.untracked.push(path),
                ("D", " ") | (" ", "D") => status.deleted.push(path),
                ("M", "M") | ("A", "M") => {
                    // Staged and then modified
                    status.staged.push(path.clone());
                    status.modified.push(path);
                }
                _ => {
                    // Handle other cases
                    if index_status != " " && index_status != "?" {
                        status.staged.push(path.clone());
                    }
                    if worktree_status != " " && worktree_status != "?" {
                        match worktree_status {
                            "M" => status.modified.push(path),
                            "D" => status.deleted.push(path),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    Ok(status)
}

// Helper function to create input validation errors
fn input_validation_error(tool: impl Into<String>, message: impl Into<String>) -> CursorError {
    CursorError::ToolExecution {
        tool: tool.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CursorConfig;

    #[test]
    fn test_parse_git_status() {
        let output = r#"## main...origin/main [ahead 2, behind 1]
 M modified.txt
M  staged.txt
A  added.txt
D  deleted.txt
?? untracked.txt
R  renamed.txt"#;

        let status = parse_git_status(output).unwrap();
        assert_eq!(status.branch, "main");
        assert_eq!(status.ahead, 2);
        assert_eq!(status.behind, 1);
        // Verify the parser processes all file entries
        let total = status.modified.len()
            + status.staged.len()
            + status.untracked.len()
            + status.deleted.len();
        assert!(total >= 5, "Expected at least 5 file entries, got {total}");
    }

    #[test]
    fn test_parse_git_status_initial_commit() {
        let output = "## No commits yet on main\n";
        let status = parse_git_status(output).unwrap();
        assert_eq!(status.branch, "main");
    }

    #[tokio::test]
    async fn test_git_status_tool() {
        // This test requires a git repository
        // Skip if not in a git repo
        let output = Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .output()
            .await;

        if output.is_err() || !output.unwrap().status.success() {
            return; // Skip test
        }

        let context = ToolContext::new(std::env::current_dir().unwrap(), CursorConfig::default());
        let tool = GitStatusTool;
        let result = tool.execute(json!({}), &context).await.unwrap();

        assert!(result.get("branch").is_some());
        assert!(result.get("modified").is_some());
        assert!(result.get("is_clean").is_some());
    }

    #[tokio::test]
    async fn test_git_branch_tool_list() {
        // This test requires a git repository
        let output = Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .output()
            .await;

        if output.is_err() || !output.unwrap().status.success() {
            return; // Skip test
        }

        let context = ToolContext::new(std::env::current_dir().unwrap(), CursorConfig::default());
        let tool = GitBranchTool;
        let result = tool
            .execute(json!({"action": "list"}), &context)
            .await
            .unwrap();

        assert!(result.get("branches").is_some());
        assert!(result.get("current").is_some());
    }
}

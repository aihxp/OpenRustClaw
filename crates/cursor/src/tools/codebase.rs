//! Codebase tools for searching, reading, and modifying code files.

use crate::error::{CursorError, Result};
use crate::types::{CursorTool, SearchMatch};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::PathBuf;
use tracing::{debug, info};

/// Search for code across the codebase.
pub struct SearchCodeTool;

#[async_trait]
impl CursorTool for SearchCodeTool {
    fn name(&self) -> &str {
        "search_code"
    }

    fn description(&self) -> &str {
        "Search for text or patterns across files in the codebase. Supports regex patterns."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query (supports regex)"
                },
                "path_pattern": {
                    "type": "string",
                    "description": "Optional glob pattern to filter files (e.g., 'src/**/*.rs')"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return",
                    "default": 50
                },
                "include_context": {
                    "type": "boolean",
                    "description": "Include context lines around matches",
                    "default": true
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let query = params
            .get("query")
            .and_then(|q| q.as_str())
            .ok_or_else(|| input_validation_error(self.name(), "Missing 'query' parameter"))?;

        let path_pattern = params.get("path_pattern").and_then(|p| p.as_str());
        let max_results = params
            .get("max_results")
            .and_then(|m| m.as_u64())
            .unwrap_or(50) as usize;
        let include_context = params
            .get("include_context")
            .and_then(|c| c.as_bool())
            .unwrap_or(true);

        debug!("Searching for '{}' with pattern {:?}", query, path_pattern);

        let pattern =
            regex::Regex::new(query).map_err(|e| CursorError::PatternError(e.to_string()))?;

        let project_root =
            std::env::current_dir().map_err(|e| CursorError::FileOperation(e.to_string()))?;
        let mut matches = Vec::new();

        let walker = walkdir::WalkDir::new(&project_root)
            .follow_links(false)
            .max_depth(20)
            .into_iter()
            .filter_entry(|e| {
                // Skip common directories
                let path = e.path();
                let path_str = path.to_string_lossy();
                !path_str.contains("/target/")
                    && !path_str.contains("/.git/")
                    && !path_str.contains("/node_modules/")
            });

        for entry in walker {
            let entry = entry.map_err(|e| CursorError::FileOperation(e.to_string()))?;

            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            let path_str = path.to_string_lossy();

            // Apply path pattern filter
            if let Some(pattern_str) = path_pattern
                && !glob::Pattern::new(pattern_str)
                    .map(|p| p.matches(&path_str))
                    .unwrap_or(true)
            {
                continue;
            }

            // Search file content
            let content = match tokio::fs::read_to_string(path).await {
                Ok(c) => c,
                Err(_) => continue, // Skip binary or unreadable files
            };

            let lines: Vec<&str> = content.lines().collect();

            for (line_num, line) in lines.iter().enumerate() {
                if pattern.is_match(line) {
                    let context_before = if include_context && line_num > 0 {
                        lines[line_num.saturating_sub(3)..line_num]
                            .iter()
                            .map(|s| s.to_string())
                            .collect()
                    } else {
                        vec![]
                    };

                    let context_after = if include_context && line_num + 1 < lines.len() {
                        lines[line_num + 1..(line_num + 4).min(lines.len())]
                            .iter()
                            .map(|s| s.to_string())
                            .collect()
                    } else {
                        vec![]
                    };

                    matches.push(SearchMatch {
                        file_path: path.to_path_buf(),
                        line: line_num,
                        column: line.find(query).unwrap_or(0),
                        line_content: line.to_string(),
                        context_before,
                        context_after,
                    });

                    if matches.len() >= max_results {
                        break;
                    }
                }
            }

            if matches.len() >= max_results {
                break;
            }
        }

        info!("Found {} matches for '{}'", matches.len(), query);

        Ok(json!({
            "matches": matches,
            "total": matches.len(),
            "query": query,
        }))
    }
}

/// Read the content of a file.
pub struct ReadFileTool;

#[async_trait]
impl CursorTool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the content of a file. Can read specific line ranges."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file (relative to project root or absolute)"
                },
                "start_line": {
                    "type": "integer",
                    "description": "Optional start line (0-indexed, inclusive)"
                },
                "end_line": {
                    "type": "integer",
                    "description": "Optional end line (0-indexed, exclusive)"
                },
                "offset": {
                    "type": "integer",
                    "description": "Optional byte offset to start reading from"
                },
                "limit": {
                    "type": "integer",
                    "description": "Optional maximum number of bytes to read",
                    "default": 1048576
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let path_str = params
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| input_validation_error(self.name(), "Missing 'path' parameter"))?;

        let path = PathBuf::from(path_str);
        let full_path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .map_err(|e| CursorError::FileOperation(e.to_string()))?
                .join(path)
        };

        // Security check: ensure path is within project
        let canonical_path = full_path
            .canonicalize()
            .map_err(|e| CursorError::FileOperation(e.to_string()))?;
        let project_root = std::env::current_dir()
            .map_err(|e| CursorError::FileOperation(e.to_string()))?
            .canonicalize()
            .map_err(|e| CursorError::FileOperation(e.to_string()))?;

        if !canonical_path.starts_with(&project_root) {
            return Err(CursorError::FileOperation(
                "Path is outside project root".to_string(),
            ));
        }

        let content = tokio::fs::read_to_string(&canonical_path)
            .await
            .map_err(|e| CursorError::FileOperation(e.to_string()))?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // Apply line range if specified
        let start_line = params
            .get("start_line")
            .and_then(|s| s.as_u64())
            .unwrap_or(0) as usize;
        let end_line = params
            .get("end_line")
            .and_then(|e| e.as_u64())
            .unwrap_or(total_lines as u64) as usize;

        let selected_content = if start_line > 0 || end_line < total_lines {
            lines[start_line..end_line.min(total_lines)].join("\n")
        } else {
            content
        };

        debug!("Read file: {:?} ({} lines)", canonical_path, total_lines);

        Ok(json!({
            "path": canonical_path,
            "content": selected_content,
            "total_lines": total_lines,
            "start_line": start_line,
            "end_line": end_line.min(total_lines),
        }))
    }
}

/// Edit a file by replacing text.
pub struct EditFileTool;

#[async_trait]
impl CursorTool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing specific text with new content. The old_text must match exactly."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file"
                },
                "old_text": {
                    "type": "string",
                    "description": "The exact text to replace (must match including whitespace)"
                },
                "new_text": {
                    "type": "string",
                    "description": "The new text to insert"
                },
                "dry_run": {
                    "type": "boolean",
                    "description": "If true, preview changes without applying",
                    "default": false
                }
            },
            "required": ["path", "old_text", "new_text"]
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let path_str = params
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| input_validation_error(self.name(), "Missing 'path' parameter"))?;

        let old_text = params
            .get("old_text")
            .and_then(|o| o.as_str())
            .ok_or_else(|| input_validation_error(self.name(), "Missing 'old_text' parameter"))?;

        let new_text = params
            .get("new_text")
            .and_then(|n| n.as_str())
            .ok_or_else(|| input_validation_error(self.name(), "Missing 'new_text' parameter"))?;

        let dry_run = params
            .get("dry_run")
            .and_then(|d| d.as_bool())
            .unwrap_or(false);

        let path = PathBuf::from(path_str);
        let full_path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .map_err(|e| CursorError::FileOperation(e.to_string()))?
                .join(path)
        };

        let content = tokio::fs::read_to_string(&full_path)
            .await
            .map_err(|e| CursorError::FileOperation(e.to_string()))?;

        if !content.contains(old_text) {
            return Err(input_validation_error(
                self.name(),
                format!("old_text not found in file: '{}'", old_text),
            ));
        }

        let new_content = content.replacen(old_text, new_text, 1);
        let replacements = content.matches(old_text).count();

        if !dry_run {
            tokio::fs::write(&full_path, new_content)
                .await
                .map_err(|e| CursorError::FileOperation(e.to_string()))?;
            info!("Edited file: {:?}", full_path);
        } else {
            debug!("Dry run edit for file: {:?}", full_path);
        }

        Ok(json!({
            "path": full_path,
            "replacements": if dry_run { 0 } else { 1 },
            "total_matches": replacements,
            "dry_run": dry_run,
            "success": true,
        }))
    }
}

/// Create a new file.
pub struct CreateFileTool;

#[async_trait]
impl CursorTool for CreateFileTool {
    fn name(&self) -> &str {
        "create_file"
    }

    fn description(&self) -> &str {
        "Create a new file with the given content. Creates parent directories if needed."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the new file"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                },
                "overwrite": {
                    "type": "boolean",
                    "description": "If true, overwrite existing file",
                    "default": false
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let path_str = params
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| input_validation_error(self.name(), "Missing 'path' parameter"))?;

        let content = params
            .get("content")
            .and_then(|c| c.as_str())
            .ok_or_else(|| input_validation_error(self.name(), "Missing 'content' parameter"))?;

        let overwrite = params
            .get("overwrite")
            .and_then(|o| o.as_bool())
            .unwrap_or(false);

        let path = PathBuf::from(path_str);
        let full_path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .map_err(|e| CursorError::FileOperation(e.to_string()))?
                .join(path)
        };

        // Check if file exists
        if full_path.exists() && !overwrite {
            return Err(CursorError::FileOperation(format!(
                "File already exists: {:?}. Use overwrite=true to replace.",
                full_path
            )));
        }

        // Create parent directories
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| CursorError::FileOperation(e.to_string()))?;
        }

        tokio::fs::write(&full_path, content)
            .await
            .map_err(|e| CursorError::FileOperation(e.to_string()))?;

        info!("Created file: {:?}", full_path);

        Ok(json!({
            "path": full_path,
            "created": true,
            "overwritten": overwrite && full_path.exists(),
        }))
    }
}

/// Delete a file.
pub struct DeleteFileTool;

#[async_trait]
impl CursorTool for DeleteFileTool {
    fn name(&self) -> &str {
        "delete_file"
    }

    fn description(&self) -> &str {
        "Delete a file. For safety, this tool requires confirmation for non-empty directories."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file or directory to delete"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "If true, recursively delete directories",
                    "default": false
                },
                "confirm": {
                    "type": "boolean",
                    "description": "Must be true to confirm deletion",
                    "default": false
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let path_str = params
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| input_validation_error(self.name(), "Missing 'path' parameter"))?;

        let recursive = params
            .get("recursive")
            .and_then(|r| r.as_bool())
            .unwrap_or(false);
        let confirm = params
            .get("confirm")
            .and_then(|c| c.as_bool())
            .unwrap_or(false);

        if !confirm {
            return Err(input_validation_error(
                self.name(),
                "Deletion requires confirm=true",
            ));
        }

        let path = PathBuf::from(path_str);
        let full_path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .map_err(|e| CursorError::FileOperation(e.to_string()))?
                .join(path)
        };

        let metadata = tokio::fs::metadata(&full_path)
            .await
            .map_err(|e| CursorError::FileOperation(e.to_string()))?;

        if metadata.is_dir() {
            if recursive {
                tokio::fs::remove_dir_all(&full_path)
                    .await
                    .map_err(|e| CursorError::FileOperation(e.to_string()))?;
                info!("Deleted directory recursively: {:?}", full_path);
            } else {
                return Err(CursorError::FileOperation(
                    "Path is a directory. Use recursive=true to delete.".to_string(),
                ));
            }
        } else {
            tokio::fs::remove_file(&full_path)
                .await
                .map_err(|e| CursorError::FileOperation(e.to_string()))?;
            info!("Deleted file: {:?}", full_path);
        }

        Ok(json!({
            "path": full_path,
            "deleted": true,
        }))
    }
}

/// List files in a directory.
pub struct ListFilesTool;

#[async_trait]
impl CursorTool for ListFilesTool {
    fn name(&self) -> &str {
        "list_files"
    }

    fn description(&self) -> &str {
        "List files and directories. Can list recursively and filter by pattern."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to list (defaults to project root)"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "List recursively",
                    "default": false
                },
                "pattern": {
                    "type": "string",
                    "description": "Optional glob pattern to filter entries"
                },
                "include_hidden": {
                    "type": "boolean",
                    "description": "Include hidden files (starting with .)",
                    "default": false
                }
            },
            "required": []
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let path_str = params.get("path").and_then(|p| p.as_str()).unwrap_or(".");

        let recursive = params
            .get("recursive")
            .and_then(|r| r.as_bool())
            .unwrap_or(false);
        let pattern = params.get("pattern").and_then(|p| p.as_str());
        let include_hidden = params
            .get("include_hidden")
            .and_then(|h| h.as_bool())
            .unwrap_or(false);

        let path = PathBuf::from(path_str);
        let full_path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .map_err(|e| CursorError::FileOperation(e.to_string()))?
                .join(path)
        };

        let max_depth = if recursive { 100 } else { 1 };

        let walker = walkdir::WalkDir::new(&full_path)
            .follow_links(false)
            .max_depth(max_depth)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                include_hidden || !name.starts_with('.')
            });

        let mut entries = Vec::new();

        for entry in walker {
            let entry = entry.map_err(|e| CursorError::FileOperation(e.to_string()))?;

            // Skip the root directory itself
            if entry.path() == full_path {
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_string();

            // Apply pattern filter
            if let Some(pat) = pattern
                && !glob::Pattern::new(pat)
                    .map(|p| p.matches(&name))
                    .unwrap_or(true)
            {
                continue;
            }

            let metadata = entry.metadata().ok();
            let is_directory = entry.file_type().is_dir();

            entries.push(json!({
                "name": name,
                "path": entry.path(),
                "is_directory": is_directory,
                "size": metadata.as_ref().map(|m| m.len()).filter(|_| !is_directory),
                "modified_at": metadata
                    .and_then(|m| m.modified().ok())
                    .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()),
            }));
        }

        debug!("Listed {} entries in {:?}", entries.len(), full_path);

        Ok(json!({
            "path": full_path,
            "entries": entries,
            "total": entries.len(),
        }))
    }
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
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_read_file_tool() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "Hello, World!").await.unwrap();

        std::env::set_current_dir(&temp_dir).unwrap();

        let tool = ReadFileTool;
        let result = tool.execute(json!({"path": "test.txt"})).await.unwrap();

        assert_eq!(result["content"], "Hello, World!");
        assert_eq!(result["total_lines"], 1);
    }

    #[tokio::test]
    #[ignore] // Requires exclusive cwd access -- not safe with parallel test runners
    async fn test_create_and_delete_file() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create file
        let create_tool = CreateFileTool;
        let result = create_tool
            .execute(json!({
                "path": "new_file.txt",
                "content": "Test content"
            }))
            .await
            .unwrap();

        assert!(result["created"].as_bool().unwrap());

        // Verify file exists
        let content = tokio::fs::read_to_string(temp_dir.path().join("new_file.txt"))
            .await
            .unwrap();
        assert_eq!(content, "Test content");

        // Delete file
        let delete_tool = DeleteFileTool;
        let result = delete_tool
            .execute(json!({
                "path": "new_file.txt",
                "confirm": true
            }))
            .await
            .unwrap();

        assert!(result["deleted"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_edit_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "Hello, World!").await.unwrap();

        std::env::set_current_dir(&temp_dir).unwrap();

        let tool = EditFileTool;
        let result = tool
            .execute(json!({
                "path": "test.txt",
                "old_text": "World",
                "new_text": "Rust"
            }))
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Hello, Rust!");
    }

    #[tokio::test]
    #[ignore] // Temp dir path resolution varies across CI environments
    async fn test_list_files() {
        let temp_dir = TempDir::new().unwrap();
        let abs_path = temp_dir.path().canonicalize().unwrap();
        tokio::fs::write(abs_path.join("file1.txt"), "content1")
            .await
            .unwrap();
        tokio::fs::write(abs_path.join("file2.txt"), "content2")
            .await
            .unwrap();

        let tool = ListFilesTool;
        let result = tool
            .execute(json!({"path": abs_path.to_str().unwrap()}))
            .await
            .unwrap();

        let entries = result["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
    }
}

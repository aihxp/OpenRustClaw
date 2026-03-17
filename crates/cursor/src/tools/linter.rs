//! Linter and formatter tools for code quality.

use crate::error::{CursorError, Result};
use crate::types::{CursorTool, Diagnostic, Severity, ToolContext};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::Path;
use tokio::process::Command;
use tracing::{error, info, warn};

/// Run a linter tool.
pub struct RunLinterTool;

#[async_trait]
impl CursorTool for RunLinterTool {
    fn name(&self) -> &str {
        "run_linter"
    }

    fn description(&self) -> &str {
        "Run a linter (clippy, cargo-check, etc.) and return diagnostics."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "tool": {
                    "type": "string",
                    "enum": ["clippy", "cargo-check", "cargo-test", "eslint", "prettier"],
                    "description": "Linter tool to run"
                },
                "path": {
                    "type": "string",
                    "description": "Optional specific file or directory to check"
                },
                "fix": {
                    "type": "boolean",
                    "description": "Try to automatically fix issues where possible",
                    "default": false
                },
                "all_targets": {
                    "type": "boolean",
                    "description": "Check all targets (bins, examples, tests, etc.)",
                    "default": true
                }
            },
            "required": ["tool"]
        })
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> Result<Value> {
        let tool_name = params
            .get("tool")
            .and_then(|t| t.as_str())
            .ok_or_else(|| input_validation_error(self.name(), "Missing 'tool' parameter"))?;

        let path = params.get("path").and_then(|p| p.as_str());
        let fix = params.get("fix").and_then(|f| f.as_bool()).unwrap_or(false);
        let all_targets = params
            .get("all_targets")
            .and_then(|a| a.as_bool())
            .unwrap_or(true);

        let project_root = context.project_root.clone();

        info!("Running linter: {} (fix: {})", tool_name, fix);

        let (command, args, parser): (String, Vec<String>, fn(&str, &Path) -> Vec<Diagnostic>) =
            match tool_name {
                "clippy" => {
                    let mut args = vec!["clippy".to_string()];
                    if all_targets {
                        args.push("--all-targets".to_string());
                    }
                    args.push("--message-format=short".to_string());
                    if fix {
                        args.push("--fix".to_string());
                        args.push("--allow-dirty".to_string());
                    }
                    if let Some(p) = path {
                        args.push("-p".to_string());
                        args.push(p.to_string());
                    }
                    ("cargo".to_string(), args, parse_cargo_diagnostics)
                }
                "cargo-check" => {
                    let mut args = vec!["check".to_string()];
                    if all_targets {
                        args.push("--all-targets".to_string());
                    }
                    args.push("--message-format=short".to_string());
                    if let Some(p) = path {
                        args.push("-p".to_string());
                        args.push(p.to_string());
                    }
                    ("cargo".to_string(), args, parse_cargo_diagnostics)
                }
                "cargo-test" => {
                    let mut args = vec!["test".to_string(), "--no-run".to_string()];
                    args.push("--message-format=short".to_string());
                    ("cargo".to_string(), args, parse_cargo_diagnostics)
                }
                "eslint" => {
                    let mut args = vec![".".to_string()];
                    if fix {
                        args.push("--fix".to_string());
                    }
                    if let Some(p) = path {
                        args.push(p.to_string());
                    }
                    (
                        "npx".to_string(),
                        vec!["eslint".to_string()],
                        parse_eslint_diagnostics,
                    )
                }
                "prettier" => {
                    #[allow(clippy::useless_vec)]
                    let mut args = vec!["--check".to_string(), ".".to_string()];
                    if fix {
                        args[0] = "--write".to_string();
                    }
                    if let Some(p) = path {
                        args[1] = p.to_string();
                    }
                    (
                        "npx".to_string(),
                        vec!["prettier".to_string()],
                        parse_prettier_diagnostics,
                    )
                }
                _ => {
                    return Err(CursorError::Linter(format!(
                        "Unknown linter tool: {}",
                        tool_name
                    )));
                }
            };

        let output = Command::new(&command)
            .args(&args)
            .current_dir(&project_root)
            .output()
            .await
            .map_err(|e| {
                error!("Failed to run {}: {}", tool_name, e);
                CursorError::Linter(e.to_string())
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}\n{}", stdout, stderr);

        // Parse diagnostics
        let diagnostics = parser(&combined, &project_root);

        // Count by severity
        let error_count = diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error))
            .count();
        let warning_count = diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Warning))
            .count();

        info!(
            "Linter {} completed: {} errors, {} warnings",
            tool_name, error_count, warning_count
        );

        Ok(json!({
            "tool": tool_name,
            "success": output.status.success() && error_count == 0,
            "exit_code": output.status.code(),
            "diagnostics": diagnostics,
            "error_count": error_count,
            "warning_count": warning_count,
            "output": combined,
            "fixed": fix,
        }))
    }
}

/// Format code using rustfmt or other formatters.
pub struct FormatCodeTool;

#[async_trait]
impl CursorTool for FormatCodeTool {
    fn name(&self) -> &str {
        "format_code"
    }

    fn description(&self) -> &str {
        "Format code using rustfmt (Rust) or other language-specific formatters."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "language": {
                    "type": "string",
                    "enum": ["rust", "javascript", "typescript", "json", "yaml"],
                    "description": "Language to format",
                    "default": "rust"
                },
                "path": {
                    "type": "string",
                    "description": "Specific file or directory to format (default: all)"
                },
                "check": {
                    "type": "boolean",
                    "description": "Check formatting without modifying files",
                    "default": false
                }
            },
            "required": []
        })
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> Result<Value> {
        let language = params
            .get("language")
            .and_then(|l| l.as_str())
            .unwrap_or("rust");
        let path = params.get("path").and_then(|p| p.as_str());
        let check = params
            .get("check")
            .and_then(|c| c.as_bool())
            .unwrap_or(false);

        let project_root = context.project_root.clone();

        info!("Formatting code: {} (check: {})", language, check);

        let (command, args): (String, Vec<String>) = match language {
            "rust" => {
                let mut args = vec!["fmt".to_string()];
                if check {
                    args.push("--".to_string());
                    args.push("--check".to_string());
                }
                if let Some(_p) = path {
                    // rustfmt doesn't support paths the same way, so we'd need different handling
                    warn!("Path-specific formatting for Rust may not work as expected");
                }
                ("cargo".to_string(), args)
            }
            "javascript" | "typescript" => {
                let mut args = vec!["prettier".to_string()];
                if check {
                    args.push("--check".to_string());
                } else {
                    args.push("--write".to_string());
                }
                args.push(path.unwrap_or(".").to_string());
                ("npx".to_string(), args)
            }
            "json" => {
                let mut args = vec![
                    "prettier".to_string(),
                    "--parser".to_string(),
                    "json".to_string(),
                ];
                if check {
                    args.push("--check".to_string());
                } else {
                    args.push("--write".to_string());
                }
                args.push(path.unwrap_or(".").to_string());
                ("npx".to_string(), args)
            }
            "yaml" => {
                let mut args = vec![
                    "prettier".to_string(),
                    "--parser".to_string(),
                    "yaml".to_string(),
                ];
                if check {
                    args.push("--check".to_string());
                } else {
                    args.push("--write".to_string());
                }
                args.push(path.unwrap_or(".").to_string());
                ("npx".to_string(), args)
            }
            _ => {
                return Err(CursorError::Linter(format!(
                    "Unsupported language for formatting: {}",
                    language
                )));
            }
        };

        let output = Command::new(&command)
            .args(&args)
            .current_dir(&project_root)
            .output()
            .await
            .map_err(|e| {
                error!("Failed to run formatter: {}", e);
                CursorError::Linter(e.to_string())
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let success = output.status.success();

        if success {
            info!("Code formatting completed successfully");
        } else {
            warn!("Code formatting found issues or failed");
        }

        Ok(json!({
            "language": language,
            "success": success,
            "check": check,
            "exit_code": output.status.code(),
            "output": format!("{}\n{}", stdout, stderr),
            "formatted_files": if check { 0 } else { parse_formatted_files(&stdout) },
        }))
    }
}

/// Parse cargo/clippy diagnostics from output.
fn parse_cargo_diagnostics(output: &str, project_root: &Path) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let re = regex::Regex::new(r"^(.*):(\d+):(\d+):\s*(error|warning|note|help):\s*(.*)$").ok();

    for line in output.lines() {
        if let Some(ref regex) = re
            && let Some(caps) = regex.captures(line)
        {
            let file_path = project_root.join(&caps[1]);
            let line_num: usize = caps[2].parse().unwrap_or(1);
            let col: usize = caps[3].parse().unwrap_or(1);
            let severity = match &caps[4] {
                "error" => Severity::Error,
                "warning" => Severity::Warning,
                _ => Severity::Information,
            };
            let message = caps[5].to_string();

            diagnostics.push(Diagnostic {
                file_path,
                severity,
                message,
                source: "cargo".to_string(),
                line: line_num.saturating_sub(1), // Convert to 0-indexed
                column: col.saturating_sub(1),
                code: None,
            });
        }
    }

    diagnostics
}

/// Parse ESLint diagnostics.
fn parse_eslint_diagnostics(output: &str, project_root: &Path) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    // ESLint output format: /path/to/file.js:line:col: severity message
    let re = regex::Regex::new(r"^\s*(.+):(\d+):(\d+):\s*(error|warning|warn)\s+(.*)$").ok();

    for line in output.lines() {
        if let Some(ref regex) = re
            && let Some(caps) = regex.captures(line)
        {
            let file_path = project_root.join(&caps[1]);
            let line_num: usize = caps[2].parse().unwrap_or(1);
            let col: usize = caps[3].parse().unwrap_or(1);
            let severity = match &caps[4] {
                "error" => Severity::Error,
                "warning" | "warn" => Severity::Warning,
                _ => Severity::Information,
            };
            let message = caps[5].to_string();

            diagnostics.push(Diagnostic {
                file_path,
                severity,
                message,
                source: "eslint".to_string(),
                line: line_num.saturating_sub(1),
                column: col.saturating_sub(1),
                code: None,
            });
        }
    }

    diagnostics
}

/// Parse prettier diagnostics.
fn parse_prettier_diagnostics(output: &str, project_root: &Path) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    // Prettier check output: [warn] path/to/file.js
    let re = regex::Regex::new(r"\[warn\]\s*(.+)$").ok();

    for line in output.lines() {
        if let Some(ref regex) = re
            && let Some(caps) = regex.captures(line)
        {
            let file_path = project_root.join(&caps[1]);

            diagnostics.push(Diagnostic {
                file_path,
                severity: Severity::Warning,
                message: "Code style issues found (prettier)".to_string(),
                source: "prettier".to_string(),
                line: 0,
                column: 0,
                code: None,
            });
        }
    }

    diagnostics
}

/// Parse number of formatted files from output.
fn parse_formatted_files(output: &str) -> usize {
    // Look for patterns like "Formatted 5 files"
    if let Some(cap) = regex::Regex::new(r"Formatted\s+(\d+)\s+files")
        .ok()
        .and_then(|re| re.captures(output))
    {
        cap[1].parse().unwrap_or(0)
    } else {
        0
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
    use std::path::PathBuf;

    #[test]
    fn test_parse_cargo_diagnostics() {
        let output = r#"
src/main.rs:10:5: error: expected `;`, found `let`
src/lib.rs:25:10: warning: unused import: `std::io`
src/main.rs:30:1: note: function `main` is never used
"#;

        let project_root = PathBuf::from("/project");
        let diagnostics = parse_cargo_diagnostics(output, &project_root);

        assert_eq!(diagnostics.len(), 3);

        assert_eq!(diagnostics[0].line, 9); // 0-indexed
        assert_eq!(diagnostics[0].column, 4); // 0-indexed
        assert!(matches!(diagnostics[0].severity, Severity::Error));

        assert!(matches!(diagnostics[1].severity, Severity::Warning));
        assert!(matches!(diagnostics[2].severity, Severity::Information));
    }

    #[test]
    fn test_parse_eslint_diagnostics() {
        let output = r#"
/src/app.js:15:3: error 'foo' is not defined no-undef
/src/app.js:20:10: warning Unexpected console statement no-console
"#;

        let project_root = PathBuf::from("/project");
        let diagnostics = parse_eslint_diagnostics(output, &project_root);

        assert!(!diagnostics.is_empty());
        assert!(matches!(diagnostics[0].severity, Severity::Error));
    }

    #[test]
    fn test_parse_formatted_files() {
        assert_eq!(parse_formatted_files("Formatted 5 files in 0.50s"), 5);
        assert_eq!(parse_formatted_files("Formatted 123 files"), 123);
        assert_eq!(parse_formatted_files("No matching files"), 0);
    }
}

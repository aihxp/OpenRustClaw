//! Cursor IDE Agent Context Protocol (ACP) integration for OpenRustClaw.
//!
//! This crate provides deep integration with the Cursor IDE, enabling the agent to:
//!
//! - Access context about open files, cursor position, and selections
//! - Execute terminal commands and read output
//! - Search, read, edit, and create files
//! - Run git operations (status, diff, commit, branch management)
//! - Execute linters and formatters
//!
//! # Architecture
//!
//! The crate implements the Agent Context Protocol (ACP), which is similar to MCP
//! but provides richer IDE-specific context:
//!
//! ```text
//! +-------------------------------------------------------------+
//! |                     Cursor IDE                              |
//! |                         |                                   |
//! |            +------------v------------+                      |
//! |            |   Cursor ACP Server     |                      |
//! |            |   (stdio/tcp/ws)        |                      |
//! |            +------------+------------+                      |
//! |                         |                                   |
//! +-------------------------|-----------------------------------+
//!                           | ACP Protocol
//! +-------------------------|-----------------------------------+
//! |                         |                                   |
//! |            +------------v------------+                      |
//! |            |  OpenRustClaw Agent     |                      |
//! |            |   (Cursor Client)       |                      |
//! |            +-------------------------+                      |
//! +-------------------------------------------------------------+
//! ```
//!
//! # Quick Start
//!
//! ```rust
//! use openrustclaw_cursor::{CursorClient, ClientConnection, CursorConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create client
//! let client = CursorClient::with_defaults();
//!
//! // Connect to Cursor IDE
//! let mut conn = client.connect().await?;
//!
//! // Get IDE state
//! let state = conn.get_state().await?;
//! println!("Active file: {:?}", state.active_file);
//!
//! // Search code
//! let matches = conn.search_code("fn main").await?;
//! println!("Found {} matches", matches.len());
//! # Ok(())
//! # }
//! ```
//!
//! # Tools
//!
//! The following tools are available:
//!
//! ## Codebase Tools
//! - `search_code`: Search for text/patterns across files
//! - `read_file`: Read file content
//! - `edit_file`: Replace text in a file
//! - `create_file`: Create new files
//! - `delete_file`: Delete files
//! - `list_files`: List directory contents
//!
//! ## Terminal Tools
//! - `run_command`: Execute shell commands
//! - `read_terminal`: Read terminal output
//!
//! ## Git Tools
//! - `git_status`: Check repository status
//! - `git_diff`: Show changes
//! - `git_commit`: Create commits
//! - `git_branch`: Manage branches
//!
//! ## Linter Tools
//! - `run_linter`: Run clippy, cargo-check, etc.
//! - `format_code`: Format code with rustfmt/prettier

#![warn(missing_docs)]

pub mod acp;
pub mod client;
pub mod error;
pub mod server;
pub mod tools;
pub mod types;

// Re-export commonly used types
pub use acp::{ACP_PROTOCOL_VERSION, AcpCapabilities, AcpMessage, AcpProtocol};
pub use client::{ClientConnection, CursorClient, CursorConnection};
pub use error::{CursorError, Result};
pub use server::{CursorServer, CursorServerConfig, ServerTransport};
pub use tools::ToolRegistry;
pub use types::{
    CursorConfig, CursorPosition, CursorTool, Diagnostic, GitState, GitStatus, IdeState, Selection,
    Severity, ToolContext,
};

use serde_json::Value;
use std::path::{Path, PathBuf};

/// Generate MCP (Model Context Protocol) configuration for Cursor.
///
/// This generates a `.cursor/mcp.json` compatible configuration that
/// can be used to integrate OpenRustClaw with Cursor IDE.
///
/// # Example
///
/// ```
/// use openrustclaw_cursor::generate_mcp_config;
/// use std::path::PathBuf;
///
/// let config = generate_mcp_config(
///     PathBuf::from("/path/to/project"),
///     vec!["clippy", "cargo-check"],
/// );
/// ```
pub fn generate_mcp_config(project_root: PathBuf, tools: Vec<&str>) -> Value {
    serde_json::json!({
        "mcpServers": {
            "openrustclaw": {
                "command": "cargo",
                "args": ["run", "--bin", "openrustclaw", "--", "cursor", "serve"],
                "env": {
                    "RUST_LOG": "info",
                    "OPENRUSTCLAW_PROJECT_ROOT": project_root.to_string_lossy(),
                },
                "description": "OpenRustClaw AI Agent with Cursor ACP",
                "enabled": true,
                "tools": tools,
            }
        }
    })
}

/// Generate `.cursor/settings.json` configuration.
///
/// This creates IDE-specific settings for optimal integration.
pub fn generate_cursor_settings(project_root: PathBuf) -> Value {
    serde_json::json!({
        "cursor": {
            "agent": {
                "enabled": true,
                "autoRun": false,
                "context": {
                    "includeOpenFiles": true,
                    "includeGitStatus": true,
                    "includeDiagnostics": true,
                },
            },
            "tools": {
                "openrustclaw": {
                    "projectRoot": project_root,
                    "includePatterns": ["src/**/*.rs", "*.toml", "*.md"],
                    "excludePatterns": ["target/**", ".git/**", "node_modules/**"],
                    "terminalTimeout": 30,
                }
            }
        }
    })
}

/// Setup Cursor integration for a project.
///
/// This creates the necessary configuration files in `.cursor/`.
///
/// # Arguments
///
/// * `project_root` - The root directory of the project
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if setup fails.
pub async fn setup_cursor_integration(project_root: PathBuf) -> Result<()> {
    use tokio::fs;

    let cursor_dir = project_root.join(".cursor");

    // Create .cursor directory
    fs::create_dir_all(&cursor_dir)
        .await
        .map_err(|e| CursorError::Config(e.to_string()))?;

    // Generate mcp.json
    let mcp_config = generate_mcp_config(project_root.clone(), vec!["*"]);
    let mcp_path = cursor_dir.join("mcp.json");
    fs::write(
        &mcp_path,
        serde_json::to_string_pretty(&mcp_config).unwrap(),
    )
    .await
    .map_err(|e| CursorError::Config(e.to_string()))?;

    // Generate settings.json
    let settings = generate_cursor_settings(project_root.clone());
    let settings_path = cursor_dir.join("settings.json");
    fs::write(
        &settings_path,
        serde_json::to_string_pretty(&settings).unwrap(),
    )
    .await
    .map_err(|e| CursorError::Config(e.to_string()))?;

    // Create agent rules file
    let rules_content = generate_agent_rules();
    let rules_path = cursor_dir.join("agent-rules.md");
    fs::write(&rules_path, rules_content)
        .await
        .map_err(|e| CursorError::Config(e.to_string()))?;

    tracing::info!("Cursor integration setup complete at {:?}", cursor_dir);

    Ok(())
}

/// Generate agent rules for Cursor.
fn generate_agent_rules() -> String {
    r#"# OpenRustClaw Agent Rules

## Context

You are an AI assistant integrated with the OpenRustClaw framework. You have access to:

- **Codebase Tools**: Search, read, edit, create files
- **Terminal Tools**: Execute commands, read output
- **Git Tools**: Status, diff, commit, branch management
- **Linter Tools**: Run clippy, rustfmt, cargo-check

## Guidelines

1. **Before making changes**:
   - Search for existing implementations
   - Check git status to understand current state
   - Read relevant files to understand context

2. **When editing code**:
   - Make minimal, focused changes
   - Prefer exact text replacements over whole-file rewrites
   - Run linters after changes

3. **Git workflow**:
   - Check git status before committing
   - Create meaningful commit messages
   - Review diff before committing

4. **Terminal usage**:
   - Commands timeout after 30 seconds by default
   - Check terminal output for errors
   - Use cargo commands for Rust projects

## Available Commands

- `search_code(query)` - Search across codebase
- `read_file(path)` - Read file content
- `edit_file(path, old_text, new_text)` - Replace text
- `create_file(path, content)` - Create new file
- `run_command(command)` - Execute shell command
- `git_status()` - Check git status
- `git_diff()` - Show changes
- `run_linter(tool)` - Run clippy/cargo-check

## Safety

- Always confirm before deleting files
- Review changes before committing
- Run tests before finalizing changes
"#
    .to_string()
}

/// Check if Cursor integration is properly configured.
///
/// Returns a report of the configuration status.
pub async fn check_cursor_setup(project_root: &Path) -> SetupStatus {
    use tokio::fs;

    let cursor_dir = project_root.join(".cursor");
    let mcp_path = cursor_dir.join("mcp.json");
    let settings_path = cursor_dir.join("settings.json");

    let has_cursor_dir = cursor_dir.exists();
    let has_mcp_config = mcp_path.exists();
    let has_settings = settings_path.exists();

    // Try to read and validate mcp.json
    let mcp_valid = if has_mcp_config {
        fs::read_to_string(&mcp_path)
            .await
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .map(|v| v.get("mcpServers").is_some())
            .unwrap_or(false)
    } else {
        false
    };

    SetupStatus {
        has_cursor_dir,
        has_mcp_config,
        has_settings,
        mcp_valid,
        all_ready: has_cursor_dir && has_mcp_config && has_settings && mcp_valid,
    }
}

/// Status of Cursor integration setup.
#[derive(Debug, Clone)]
pub struct SetupStatus {
    /// `.cursor/` directory exists.
    pub has_cursor_dir: bool,
    /// `mcp.json` file exists.
    pub has_mcp_config: bool,
    /// `settings.json` file exists.
    pub has_settings: bool,
    /// MCP configuration is valid.
    pub mcp_valid: bool,
    /// All components are ready.
    pub all_ready: bool,
}

impl std::fmt::Display for SetupStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Cursor Integration Status:")?;
        writeln!(
            f,
            "  .cursor/ directory: {}",
            if self.has_cursor_dir { "✓" } else { "✗" }
        )?;
        writeln!(
            f,
            "  mcp.json: {}",
            if self.has_mcp_config { "✓" } else { "✗" }
        )?;
        writeln!(
            f,
            "  settings.json: {}",
            if self.has_settings { "✓" } else { "✗" }
        )?;
        writeln!(f, "  MCP valid: {}", if self.mcp_valid { "✓" } else { "✗" })?;
        writeln!(
            f,
            "  Overall: {}",
            if self.all_ready {
                "✓ Ready"
            } else {
                "✗ Not ready"
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_mcp_config() {
        let config = generate_mcp_config(PathBuf::from("/test"), vec!["clippy"]);

        assert!(config.get("mcpServers").is_some());
        let orc = &config["mcpServers"]["openrustclaw"];
        assert_eq!(orc["command"], "cargo");
        assert!(
            orc["args"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("cursor"))
        );
    }

    #[test]
    fn test_generate_cursor_settings() {
        let settings = generate_cursor_settings(PathBuf::from("/test"));

        assert!(settings.get("cursor").is_some());
        assert!(settings["cursor"]["agent"]["enabled"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_setup_and_check() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        // Check before setup
        let before = check_cursor_setup(&path).await;
        assert!(!before.all_ready);

        // Setup
        setup_cursor_integration(path.clone()).await.unwrap();

        // Check after setup
        let after = check_cursor_setup(&path).await;
        assert!(after.all_ready);
        assert!(after.has_cursor_dir);
        assert!(after.has_mcp_config);
        assert!(after.has_settings);
        assert!(after.mcp_valid);

        // Verify files exist
        assert!(path.join(".cursor/mcp.json").exists());
        assert!(path.join(".cursor/settings.json").exists());
        assert!(path.join(".cursor/agent-rules.md").exists());
    }

    #[test]
    fn test_setup_status_display() {
        let status = SetupStatus {
            has_cursor_dir: true,
            has_mcp_config: true,
            has_settings: false,
            mcp_valid: true,
            all_ready: false,
        };

        let display = format!("{}", status);
        assert!(display.contains("Cursor Integration Status"));
        assert!(display.contains("✓"));
        assert!(display.contains("✗"));
    }
}

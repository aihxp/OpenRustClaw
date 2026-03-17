//! Cursor-specific tools for IDE integration.
//!
//! This module provides tools that enable the agent to interact with the Cursor IDE:
//! - Codebase tools: Search, read, edit, create, delete files
//! - Terminal tools: Run commands, read output
//! - Git tools: Status, diff, commit, branch management
//! - Linter tools: Run linters and formatters

use crate::error::{CursorError, Result};
use crate::types::{CursorTool, ToolContext};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod codebase;
pub mod git;
pub mod linter;
pub mod terminal;

pub use codebase::{
    CreateFileTool, DeleteFileTool, EditFileTool, ListFilesTool, ReadFileTool, SearchCodeTool,
};
pub use git::{GitBranchTool, GitCommitTool, GitDiffTool, GitStatusTool};
pub use linter::{FormatCodeTool, RunLinterTool};
pub use terminal::{ReadTerminalTool, RunCommandTool};

/// Registry of available Cursor tools.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn CursorTool>>,
    context: Arc<RwLock<ToolContext>>,
}

impl ToolRegistry {
    /// Create a new tool registry with all default tools.
    pub fn new(context: ToolContext) -> Self {
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
        registry.register(Arc::new(RunCommandTool::default()));
        registry.register(Arc::new(ReadTerminalTool::default()));

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

        tool.execute(params).await
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

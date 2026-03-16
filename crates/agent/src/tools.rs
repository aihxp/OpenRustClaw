//! Tool registry and execution.

use std::collections::HashMap;
use std::sync::Arc;

use openrustclaw_core::error::{Error, Result, ToolError};
use openrustclaw_core::traits::{Tool, ToolContext};
use openrustclaw_core::types::{ToolCall, ToolDefinition, ToolOutput};
use tracing::{info, warn};

/// Registry of available tools.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        info!(tool = %name, "Registered tool");
        self.tools.insert(name, tool);
    }

    /// Get tool definitions for LLM.
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .map(|t| ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.schema(),
                strict: false,
            })
            .collect()
    }

    /// Execute a tool call.
    pub async fn execute(&self, call: &ToolCall, ctx: &ToolContext) -> Result<ToolOutput> {
        let tool = self
            .tools
            .get(&call.name)
            .ok_or_else(|| Error::Tool(ToolError::NotFound(call.name.clone())))?;

        info!(tool = %call.name, "Executing tool");

        match tool.execute(call.arguments.clone(), ctx).await {
            Ok(mut output) => {
                output.tool_call_id = call.id.clone();
                Ok(output)
            }
            Err(e) => {
                warn!(tool = %call.name, error = %e, "Tool execution failed");
                Ok(ToolOutput {
                    tool_call_id: call.id.clone(),
                    content: format!("Error: {}", e),
                    is_error: true,
                })
            }
        }
    }

    /// Get tool count.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

//! Built-in memory tools for the agent.
//!
//! These tools allow the agent to search, store, and update memory on demand.

use async_trait::async_trait;
use openrustclaw_core::error::Result;
use openrustclaw_core::traits::{Tool, ToolContext};
use openrustclaw_core::types::{SkillCapability, ToolOutput};
use serde_json::Value;

/// Tool: Search recall memory for relevant information.
pub struct MemorySearchTool;

#[async_trait]
impl Tool for MemorySearchTool {
    fn name(&self) -> &str {
        "memory_search"
    }

    fn description(&self) -> &str {
        "Search your memory for information relevant to the current conversation. \
         Use this when you need to recall facts, preferences, or past discussions."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results (default: 5)",
                    "default": 5
                }
            },
            "required": ["query"]
        })
    }

    fn capabilities_required(&self) -> Vec<SkillCapability> {
        vec![] // Memory access is built-in, no special capability needed
    }

    async fn execute(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        // TODO: Wire to actual MemoryStore
        Ok(ToolOutput {
            tool_call_id: String::new(),
            content: "No results found.".to_string(),
            is_error: false,
        })
    }
}

/// Tool: Store important information for later recall.
pub struct MemoryStoreTool;

#[async_trait]
impl Tool for MemoryStoreTool {
    fn name(&self) -> &str {
        "memory_store"
    }

    fn description(&self) -> &str {
        "Remember an important fact, preference, or piece of information for future reference."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "The information to remember"
                },
                "importance": {
                    "type": "number",
                    "description": "Importance score from 0.0 to 1.0 (default: 0.7)",
                    "default": 0.7
                }
            },
            "required": ["content"]
        })
    }

    fn capabilities_required(&self) -> Vec<SkillCapability> {
        vec![SkillCapability::MemoryWrite]
    }

    async fn execute(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        // TODO: Wire to actual MemoryStore
        Ok(ToolOutput {
            tool_call_id: String::new(),
            content: "Memory stored successfully.".to_string(),
            is_error: false,
        })
    }
}

/// Tool: Update core memory (the tiny always-loaded context).
pub struct CoreMemoryUpdateTool;

#[async_trait]
impl Tool for CoreMemoryUpdateTool {
    fn name(&self) -> &str {
        "core_memory_update"
    }

    fn description(&self) -> &str {
        "Update your core knowledge about the user or project. \
         Core memory is always loaded, so use this for truly important facts."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "description": "The memory key (e.g., 'user_name', 'timezone', 'project')"
                },
                "value": {
                    "type": "string",
                    "description": "The value to store (keep concise, 1-2 sentences)"
                }
            },
            "required": ["key", "value"]
        })
    }

    fn capabilities_required(&self) -> Vec<SkillCapability> {
        vec![SkillCapability::MemoryWrite]
    }

    async fn execute(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        // TODO: Wire to actual CoreMemoryStore
        Ok(ToolOutput {
            tool_call_id: String::new(),
            content: "Core memory updated.".to_string(),
            is_error: false,
        })
    }
}

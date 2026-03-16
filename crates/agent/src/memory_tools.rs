//! Built-in memory tools for the agent.
//!
//! These tools allow the agent to search, store, and update memory on demand.

use async_trait::async_trait;
use chrono::Utc;
use openrustclaw_core::error::Result;
use openrustclaw_core::traits::{CoreMemoryStore, MemoryStore, Tool, ToolContext};
use openrustclaw_core::types::{CoreEntry, MemoryEntry, MemoryQuery, MemoryType, ToolOutput};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

/// Tool: Search recall memory for relevant information.
pub struct MemorySearchTool {
    memory_store: Arc<dyn MemoryStore>,
}

impl MemorySearchTool {
    /// Create a new MemorySearchTool with the given memory store.
    pub fn new(memory_store: Arc<dyn MemoryStore>) -> Self {
        Self { memory_store }
    }
}

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

    fn capabilities_required(&self) -> Vec<openrustclaw_core::types::SkillCapability> {
        vec![] // Memory access is built-in, no special capability needed
    }

    #[instrument(skip(self, input, ctx), fields(user_id = %ctx.user_id, session_id = %ctx.session_id))]
    async fn execute(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let query_text = input
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                openrustclaw_core::error::Error::Tool(
                    openrustclaw_core::error::ToolError::InputValidation {
                        tool: self.name().to_string(),
                        message: "Missing or invalid 'query' parameter".to_string(),
                    },
                )
            })?;

        let limit = input
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(5);

        debug!(query = %query_text, limit = limit, "Searching memory");

        let query = MemoryQuery {
            text: query_text.to_string(),
            memory_types: vec![],
            source_types: vec![],
            namespace: Some(ctx.user_id.clone()),
            limit,
            min_confidence: 0.0,
            recency_weight: 0.0,
        };

        match self.memory_store.search(&query).await {
            Ok(results) => {
                info!(count = results.len(), "Memory search completed");

                if results.is_empty() {
                    return Ok(ToolOutput {
                        tool_call_id: String::new(),
                        content: "No relevant memories found.".to_string(),
                        is_error: false,
                    });
                }

                let mut content = String::from("Found the following relevant memories:\n\n");
                for (i, scored) in results.iter().enumerate() {
                    content.push_str(&format!(
                        "{}. {} (score: {:.2}, importance: {:.2})\n",
                        i + 1,
                        scored.entry.content,
                        scored.score,
                        scored.entry.importance
                    ));
                }

                Ok(ToolOutput {
                    tool_call_id: String::new(),
                    content,
                    is_error: false,
                })
            }
            Err(e) => {
                warn!(error = %e, "Memory search failed");
                Ok(ToolOutput {
                    tool_call_id: String::new(),
                    content: format!("Error searching memory: {}", e),
                    is_error: true,
                })
            }
        }
    }
}

/// Tool: Store important information for later recall.
pub struct MemoryStoreTool {
    memory_store: Arc<dyn MemoryStore>,
}

impl MemoryStoreTool {
    /// Create a new MemoryStoreTool with the given memory store.
    pub fn new(memory_store: Arc<dyn MemoryStore>) -> Self {
        Self { memory_store }
    }

    /// Compute SHA-256 hash of content for deduplication.
    fn compute_content_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    }
}

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

    fn capabilities_required(&self) -> Vec<openrustclaw_core::types::SkillCapability> {
        vec![openrustclaw_core::types::SkillCapability::MemoryWrite]
    }

    #[instrument(skip(self, input, ctx), fields(user_id = %ctx.user_id, session_id = %ctx.session_id))]
    async fn execute(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let content = input
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                openrustclaw_core::error::Error::Tool(
                    openrustclaw_core::error::ToolError::InputValidation {
                        tool: self.name().to_string(),
                        message: "Missing or invalid 'content' parameter".to_string(),
                    },
                )
            })?;

        let importance = input
            .get("importance")
            .and_then(|v| v.as_f64())
            .map(|v| v.clamp(0.0, 1.0) as f32)
            .unwrap_or(0.7);

        // Check for duplicates
        let content_hash = Self::compute_content_hash(content);
        match self.memory_store.dedupe_check(&content_hash).await {
            Ok(Some(existing_id)) => {
                debug!(existing_id = %existing_id, "Duplicate memory detected");
                return Ok(ToolOutput {
                    tool_call_id: String::new(),
                    content: format!(
                        "This information is already stored (memory id: {}).",
                        existing_id
                    ),
                    is_error: false,
                });
            }
            Ok(None) => {}
            Err(e) => {
                warn!(error = %e, "Deduplication check failed, proceeding with store");
            }
        }

        let entry = MemoryEntry {
            id: uuid::Uuid::new_v4(),
            memory_type: MemoryType::Semantic,
            content: content.to_string(),
            content_hash,
            source: Some("agent_tool".to_string()),
            source_type: Some(openrustclaw_core::types::SourceType::Conversation),
            session_id: uuid::Uuid::parse_str(&ctx.session_id).ok(),
            user_id: Some(ctx.user_id.clone()),
            namespace: ctx.user_id.clone(),
            importance,
            confidence: 1.0,
            access_count: 0,
            last_accessed: None,
            created_at: Utc::now(),
            expires_at: None,
            metadata: serde_json::json!({
                "tool": "memory_store",
                "source": "agent_execution"
            }),
        };

        match self.memory_store.store(entry).await {
            Ok(()) => {
                info!("Memory stored successfully");
                Ok(ToolOutput {
                    tool_call_id: String::new(),
                    content: "Memory stored successfully.".to_string(),
                    is_error: false,
                })
            }
            Err(e) => {
                warn!(error = %e, "Failed to store memory");
                Ok(ToolOutput {
                    tool_call_id: String::new(),
                    content: format!("Error storing memory: {}", e),
                    is_error: true,
                })
            }
        }
    }
}

/// Tool: Update core memory (the tiny always-loaded context).
pub struct CoreMemoryUpdateTool {
    core_memory_store: Arc<dyn CoreMemoryStore>,
}

impl CoreMemoryUpdateTool {
    /// Create a new CoreMemoryUpdateTool with the given core memory store.
    pub fn new(core_memory_store: Arc<dyn CoreMemoryStore>) -> Self {
        Self { core_memory_store }
    }

    /// Estimate token count (rough approximation: 1 token ≈ 4 characters).
    fn estimate_tokens(text: &str) -> usize {
        text.len() / 4
    }
}

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

    fn capabilities_required(&self) -> Vec<openrustclaw_core::types::SkillCapability> {
        vec![openrustclaw_core::types::SkillCapability::MemoryWrite]
    }

    #[instrument(skip(self, input, ctx), fields(user_id = %ctx.user_id, session_id = %ctx.session_id))]
    async fn execute(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let key = input
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                openrustclaw_core::error::Error::Tool(
                    openrustclaw_core::error::ToolError::InputValidation {
                        tool: self.name().to_string(),
                        message: "Missing or invalid 'key' parameter".to_string(),
                    },
                )
            })?;

        let value = input
            .get("value")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                openrustclaw_core::error::Error::Tool(
                    openrustclaw_core::error::ToolError::InputValidation {
                        tool: self.name().to_string(),
                        message: "Missing or invalid 'value' parameter".to_string(),
                    },
                )
            })?;

        debug!(key = %key, "Updating core memory");

        let token_count = Self::estimate_tokens(value);
        let entry = CoreEntry {
            key: key.to_string(),
            value: value.to_string(),
            importance: 1.0, // Core memory is always important
            token_count,
            updated_at: Utc::now(),
        };

        match self.core_memory_store.set(&ctx.user_id, entry).await {
            Ok(()) => {
                info!(key = %key, "Core memory updated successfully");
                Ok(ToolOutput {
                    tool_call_id: String::new(),
                    content: format!("Core memory '{}' updated successfully.", key),
                    is_error: false,
                })
            }
            Err(e) => {
                warn!(key = %key, error = %e, "Failed to update core memory");
                Ok(ToolOutput {
                    tool_call_id: String::new(),
                    content: format!("Error updating core memory: {}", e),
                    is_error: true,
                })
            }
        }
    }
}

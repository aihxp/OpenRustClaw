//! Tool registry and execution.

use std::collections::HashMap;
use std::sync::Arc;

use openrustclaw_core::error::{Error, Result, ToolError};
use openrustclaw_core::traits::{CoreMemoryStore, MemoryStore, Tool, ToolContext};
use openrustclaw_core::types::{ToolCall, ToolDefinition, ToolOutput};
use tracing::{info, warn};

use crate::tool_factory::ToolFactory;

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

    /// Create a new registry with memory tools pre-registered.
    ///
    /// This convenience method creates a ToolRegistry with all memory-related
    /// tools (memory_search, memory_store, core_memory_update) already configured
    /// and ready to use.
    ///
    /// # Arguments
    ///
    /// * `memory_store` - The memory store for recall memory operations
    /// * `core_memory_store` - The core memory store for persistent key-value storage
    ///
    /// # Example
    ///
    /// ```ignore
    /// use std::sync::Arc;
    /// use openrustclaw_agent::ToolRegistry;
    /// use openrustclaw_core::traits::{MemoryStore, CoreMemoryStore};
    ///
    /// let memory_store: Arc<dyn MemoryStore> = // ... initialize store
    /// let core_memory_store: Arc<dyn CoreMemoryStore> = // ... initialize store
    ///
    /// let registry = ToolRegistry::with_memory_tools(memory_store, core_memory_store);
    /// ```
    pub fn with_memory_tools(
        memory_store: Arc<dyn MemoryStore>,
        core_memory_store: Arc<dyn CoreMemoryStore>,
    ) -> Self {
        let mut registry = Self::new();
        let factory = ToolFactory::new(memory_store, core_memory_store);
        factory.register_all(&mut registry);
        registry
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

    /// Check if a tool with the given name is registered.
    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Create a new registry containing only the named tools that exist.
    pub fn filtered(&self, allowed_names: &[&str]) -> Self {
        let mut registry = Self::new();
        for name in allowed_names {
            if let Some(tool) = self.tools.get(*name) {
                registry.tools.insert((*name).to_string(), tool.clone());
            }
        }
        registry
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_factory::ToolFactory;
    use async_trait::async_trait;
    use openrustclaw_core::traits::{CoreMemoryStore, MemoryStore};
    use openrustclaw_core::types::{CoreEntry, MemoryEntry, MemoryQuery, ScoredMemory};
    use std::sync::Mutex;

    struct MockMemoryStore;

    #[async_trait]
    impl MemoryStore for MockMemoryStore {
        async fn store(&self, _entry: MemoryEntry) -> Result<()> {
            Ok(())
        }

        async fn search(&self, _query: &MemoryQuery) -> Result<Vec<ScoredMemory>> {
            Ok(vec![])
        }

        async fn get(&self, _id: &str) -> Result<Option<MemoryEntry>> {
            Ok(None)
        }

        async fn delete(&self, _id: &str) -> Result<()> {
            Ok(())
        }

        async fn dedupe_check(&self, _content_hash: &str) -> Result<Option<String>> {
            Ok(None)
        }

        async fn expire_stale(&self) -> Result<u64> {
            Ok(0)
        }
    }

    struct MockCoreMemoryStore {
        entries: Mutex<Vec<(String, CoreEntry)>>,
    }

    impl MockCoreMemoryStore {
        fn new() -> Self {
            Self {
                entries: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl CoreMemoryStore for MockCoreMemoryStore {
        async fn get_all(&self, _user_id: &str) -> Result<Vec<CoreEntry>> {
            Ok(self
                .entries
                .lock()
                .unwrap()
                .iter()
                .map(|(_, e)| e.clone())
                .collect())
        }

        async fn set(&self, user_id: &str, entry: CoreEntry) -> Result<()> {
            let mut entries = self.entries.lock().unwrap();
            entries.retain(|(uid, e)| uid != user_id || e.key != entry.key);
            entries.push((user_id.to_string(), entry));
            Ok(())
        }

        async fn remove(&self, user_id: &str, key: &str) -> Result<()> {
            let mut entries = self.entries.lock().unwrap();
            entries.retain(|(uid, e)| uid != user_id || e.key != key);
            Ok(())
        }

        async fn render(&self, _user_id: &str) -> Result<String> {
            Ok(String::new())
        }

        async fn total_tokens(&self, _user_id: &str) -> Result<usize> {
            Ok(0)
        }
    }

    #[test]
    fn filtered_registry_keeps_only_requested_tools() {
        let memory_store: Arc<dyn MemoryStore> = Arc::new(MockMemoryStore);
        let core_memory_store: Arc<dyn CoreMemoryStore> = Arc::new(MockCoreMemoryStore::new());
        let factory = ToolFactory::new(memory_store, core_memory_store);
        let mut registry = ToolRegistry::new();
        factory.register_all(&mut registry);

        let filtered = registry.filtered(&["memory_search", "memory_store"]);

        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains("memory_search"));
        assert!(filtered.contains("memory_store"));
        assert!(!filtered.contains("core_memory_update"));
    }
}

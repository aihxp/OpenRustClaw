//! Tool factory for creating tools with their dependencies.
//!
//! This module provides a factory pattern for creating tool instances
//! that are pre-configured with the necessary storage backends.

use crate::memory_tools::{CoreMemoryUpdateTool, MemorySearchTool, MemoryStoreTool};
use crate::tools::ToolRegistry;
use openrustclaw_core::traits::{CoreMemoryStore, MemoryStore};
use openrustclaw_memory::embeddings::EmbeddingService;
use std::sync::Arc;

/// Factory for creating tools with their dependencies injected.
///
/// This factory holds references to the storage backends and uses them
/// to create tool instances that are ready to use.
pub struct ToolFactory {
    memory_store: Arc<dyn MemoryStore>,
    core_memory_store: Arc<dyn CoreMemoryStore>,
    embedding_service: Option<Arc<EmbeddingService>>,
}

impl ToolFactory {
    /// Create a new ToolFactory with the given storage backends.
    ///
    /// # Arguments
    ///
    /// * `memory_store` - The memory store for recall memory operations
    /// * `core_memory_store` - The core memory store for persistent key-value storage
    pub fn new(
        memory_store: Arc<dyn MemoryStore>,
        core_memory_store: Arc<dyn CoreMemoryStore>,
    ) -> Self {
        Self {
            memory_store,
            core_memory_store,
            embedding_service: None,
        }
    }

    pub fn with_embedding_service(mut self, embedding_service: Arc<EmbeddingService>) -> Self {
        self.embedding_service = Some(embedding_service);
        self
    }

    /// Create a MemorySearchTool instance.
    ///
    /// The returned tool will use the factory's memory store to search
    /// for relevant memories based on user queries.
    pub fn create_memory_search_tool(&self) -> MemorySearchTool {
        MemorySearchTool::new(self.memory_store.clone(), self.embedding_service.clone())
    }

    /// Create a MemoryStoreTool instance.
    ///
    /// The returned tool will use the factory's memory store to persist
    /// new memories for future recall.
    pub fn create_memory_store_tool(&self) -> MemoryStoreTool {
        MemoryStoreTool::new(self.memory_store.clone())
    }

    /// Create a CoreMemoryUpdateTool instance.
    ///
    /// The returned tool will use the factory's core memory store to
    /// update the agent's core knowledge about the user.
    pub fn create_core_memory_update_tool(&self) -> CoreMemoryUpdateTool {
        CoreMemoryUpdateTool::new(self.core_memory_store.clone())
    }

    /// Register all memory tools with the given registry.
    ///
    /// This convenience method creates all three memory tools and registers
    /// them in the provided registry.
    ///
    /// # Arguments
    ///
    /// * `registry` - The ToolRegistry to register the tools with
    pub fn register_all(&self, registry: &mut ToolRegistry) {
        registry.register(Arc::new(self.create_memory_search_tool()));
        registry.register(Arc::new(self.create_memory_store_tool()));
        registry.register(Arc::new(self.create_core_memory_update_tool()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use openrustclaw_core::error::Result;
    use openrustclaw_core::traits::{CoreMemoryStore, MemoryStore};
    use openrustclaw_core::types::{CoreEntry, MemoryEntry, MemoryQuery, ScoredMemory};
    use std::sync::Mutex;

    // Mock implementations for testing
    struct MockMemoryStore {
        entries: Mutex<Vec<MemoryEntry>>,
    }

    impl MockMemoryStore {
        fn new() -> Self {
            Self {
                entries: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl MemoryStore for MockMemoryStore {
        async fn store(&self, entry: MemoryEntry) -> Result<()> {
            self.entries.lock().unwrap().push(entry);
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
    fn test_tool_factory_new() {
        let memory_store: Arc<dyn MemoryStore> = Arc::new(MockMemoryStore::new());
        let core_memory_store: Arc<dyn CoreMemoryStore> = Arc::new(MockCoreMemoryStore::new());

        let factory = ToolFactory::new(memory_store, core_memory_store);

        // Just verify it compiles and creates without panicking
        let _ = factory.create_memory_search_tool();
        let _ = factory.create_memory_store_tool();
        let _ = factory.create_core_memory_update_tool();
    }

    #[test]
    fn test_tool_factory_register_all() {
        let memory_store: Arc<dyn MemoryStore> = Arc::new(MockMemoryStore::new());
        let core_memory_store: Arc<dyn CoreMemoryStore> = Arc::new(MockCoreMemoryStore::new());

        let factory = ToolFactory::new(memory_store, core_memory_store);
        let mut registry = ToolRegistry::new();

        factory.register_all(&mut registry);

        assert_eq!(registry.len(), 3);
    }
}

//! Built-in memory tools for the agent.
//!
//! These tools allow the agent to search, store, and update memory on demand.

use async_trait::async_trait;
use chrono::Utc;
use openrustclaw_core::error::Result;
use openrustclaw_core::traits::{CoreMemoryStore, MemoryStore, Tool, ToolContext};
use openrustclaw_core::types::{
    CoreEntry, MemoryQuery, MemorySource, MemoryType, RecallPack, ToolOutput,
};
use openrustclaw_memory::context::build_recall_pack;
use openrustclaw_memory::embeddings::EmbeddingService;
use openrustclaw_memory::{AssistantMemoryWriteBasis, MemoryPolicies, RecallMemory};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

/// Tool: Search recall memory for relevant information.
pub struct MemorySearchTool {
    memory_store: Arc<dyn MemoryStore>,
    embedding_service: Option<Arc<EmbeddingService>>,
}

impl MemorySearchTool {
    /// Create a new MemorySearchTool with the given memory store.
    pub fn new(
        memory_store: Arc<dyn MemoryStore>,
        embedding_service: Option<Arc<EmbeddingService>>,
    ) -> Self {
        Self {
            memory_store,
            embedding_service,
        }
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
        let query_text = input.get("query").and_then(|v| v.as_str()).ok_or_else(|| {
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

        let results = if let Some(embedding_service) = &self.embedding_service {
            match embedding_service.embed_query(query_text).await {
                Ok(query_embedding) => match self
                    .memory_store
                    .search_with_embedding(&query, &query_embedding)
                    .await
                {
                    Ok(results) => results,
                    Err(error) => {
                        warn!(error = %error, "Hybrid memory search unavailable, falling back to lexical retrieval");
                        self.memory_store.search(&query).await?
                    }
                },
                Err(error) => {
                    warn!(error = %error, "Query embedding unavailable, falling back to lexical retrieval");
                    self.memory_store.search(&query).await?
                }
            }
        } else {
            self.memory_store.search(&query).await?
        };

        info!(count = results.len(), "Memory search completed");

        if results.is_empty() {
            return Ok(ToolOutput {
                tool_call_id: String::new(),
                content: "No relevant memories found.".to_string(),
                is_error: false,
            });
        }

        let pack = build_recall_pack(&results, limit, 220);
        let content = render_recall_pack(&pack);

        Ok(ToolOutput {
            tool_call_id: String::new(),
            content,
            is_error: false,
        })
    }
}

fn render_recall_pack(pack: &RecallPack) -> String {
    let mut content = String::from("Found the following relevant memories:\n\n");
    if pack.degraded {
        let reasons = pack
            .items
            .iter()
            .filter_map(|item| item.explanation.degraded_state.as_ref())
            .map(|state| state.message.as_str())
            .collect::<Vec<_>>();
        if !reasons.is_empty() {
            content.push_str("Retrieval status: degraded vector lane.\n");
            content.push_str(&format!("Reason: {}\n\n", reasons.join(" | ")));
        }
    }

    for (index, item) in pack.items.iter().enumerate() {
        content.push_str(&format!("{}. {}\n", index + 1, item.content));
        content.push_str(&format!(
            "   score={:.2} importance={:.2} confidence={:.2}\n",
            item.score, item.importance, item.confidence
        ));
        content.push_str(&format!(
            "   artifact={:?} namespace={} vector_lane={:?}\n",
            item.explanation.primary_artifact.artifact_kind,
            item.namespace,
            item.explanation.factors.vector_lane
        ));
        if let Some(freshness) = &item.explanation.freshness {
            content.push_str(&format!(
                "   freshness: age_seconds={} created_at={}\n",
                freshness.age_seconds, freshness.created_at
            ));
        }
        if let Some(source_label) = &item.explanation.primary_artifact.source_label {
            content.push_str(&format!("   source={}\n", source_label));
        }
        content.push('\n');
    }

    content
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
        "Remember something only when the user explicitly asked you to remember it or when the content is an obviously durable user or project fact worth future recall."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "The information to remember"
                },
                "basis": {
                    "type": "string",
                    "enum": ["explicit_user_request", "durable_user_fact", "ephemeral_context", "agent_inference"],
                    "description": "Why this write is allowed under the assistant memory policy"
                },
                "reason": {
                    "type": "string",
                    "description": "A short explanation of why the memory should persist"
                },
                "memory_type": {
                    "type": "string",
                    "enum": ["semantic", "episodic", "procedural"],
                    "description": "The recall memory type to store (default: semantic)",
                    "default": "semantic"
                },
                "importance": {
                    "type": "number",
                    "description": "Importance score from 0.0 to 1.0 (default: 0.7)",
                    "default": 0.7
                }
            },
            "required": ["content", "basis", "reason"]
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

        let basis = parse_write_basis(&input, self.name())?;
        let reason = input
            .get("reason")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                openrustclaw_core::error::Error::Tool(
                    openrustclaw_core::error::ToolError::InputValidation {
                        tool: self.name().to_string(),
                        message: "Missing or invalid 'reason' parameter".to_string(),
                    },
                )
            })?;
        let memory_type = parse_memory_type(&input, self.name())?;
        let importance = input
            .get("importance")
            .and_then(|v| v.as_f64())
            .map(|v| v.clamp(0.0, 1.0) as f32)
            .unwrap_or(0.7);
        let policies = MemoryPolicies::default();
        let decision = policies.evaluate_assistant_write(content, basis, reason);

        if !decision.allowed {
            return Ok(ToolOutput {
                tool_call_id: String::new(),
                content: format!("Memory not stored: {}", decision.reason),
                is_error: false,
            });
        }

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

        let recall = RecallMemory::new(policies);
        let source = match basis {
            AssistantMemoryWriteBasis::ExplicitUserRequest
            | AssistantMemoryWriteBasis::DurableUserFact => MemorySource::ExplicitUserStatement,
            AssistantMemoryWriteBasis::EphemeralContext => MemorySource::ConversationSummary,
            AssistantMemoryWriteBasis::AgentInference => MemorySource::AgentInference,
        };
        let mut entry = recall.prepare_entry(
            content,
            memory_type,
            source,
            Some(&ctx.user_id),
            uuid::Uuid::parse_str(&ctx.session_id).ok(),
            Some(&ctx.user_id),
        );
        entry.importance = importance.max(entry.importance).clamp(0.0, 1.0);
        entry.metadata = serde_json::json!({
            "tool": "memory_store",
            "source": "agent_execution",
            "assistant_write_policy": {
                "basis": basis,
                "declared_reason": reason,
                "decision_reason": decision.reason,
                "version": "v1"
            }
        });

        match self.memory_store.store(entry).await {
            Ok(()) => {
                info!("Memory stored successfully");
                Ok(ToolOutput {
                    tool_call_id: String::new(),
                    content: format!("Memory stored successfully under {:?} policy.", basis),
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

fn parse_write_basis(input: &Value, tool_name: &str) -> Result<AssistantMemoryWriteBasis> {
    let raw = input.get("basis").cloned().ok_or_else(|| {
        openrustclaw_core::error::Error::Tool(
            openrustclaw_core::error::ToolError::InputValidation {
                tool: tool_name.to_string(),
                message: "Missing 'basis' parameter".to_string(),
            },
        )
    })?;
    serde_json::from_value(raw).map_err(|_| {
        openrustclaw_core::error::Error::Tool(
            openrustclaw_core::error::ToolError::InputValidation {
                tool: tool_name.to_string(),
                message: "Invalid 'basis' parameter".to_string(),
            },
        )
    })
}

fn parse_memory_type(input: &Value, tool_name: &str) -> Result<MemoryType> {
    match input.get("memory_type").and_then(|value| value.as_str()) {
        None => Ok(MemoryType::Semantic),
        Some("semantic") => Ok(MemoryType::Semantic),
        Some("episodic") => Ok(MemoryType::Episodic),
        Some("procedural") => Ok(MemoryType::Procedural),
        Some(_) => Err(openrustclaw_core::error::Error::Tool(
            openrustclaw_core::error::ToolError::InputValidation {
                tool: tool_name.to_string(),
                message: "Invalid 'memory_type' parameter".to_string(),
            },
        )),
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
        let key = input.get("key").and_then(|v| v.as_str()).ok_or_else(|| {
            openrustclaw_core::error::Error::Tool(
                openrustclaw_core::error::ToolError::InputValidation {
                    tool: self.name().to_string(),
                    message: "Missing or invalid 'key' parameter".to_string(),
                },
            )
        })?;

        let value = input.get("value").and_then(|v| v.as_str()).ok_or_else(|| {
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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use openrustclaw_core::traits::{EmbeddingProvider, MemoryStore, ToolContext};
    use openrustclaw_core::types::{
        MemoryEntry, RetrievalArtifactKind, RetrievalExplanation, ScoredMemory,
    };
    use openrustclaw_memory::embeddings::EmbeddingService;
    use serde_json::json;
    use std::sync::Mutex;
    use uuid::Uuid;

    struct RecordingMemoryStore {
        search_calls: Mutex<usize>,
        hybrid_calls: Mutex<usize>,
    }

    impl RecordingMemoryStore {
        fn new() -> Self {
            Self {
                search_calls: Mutex::new(0),
                hybrid_calls: Mutex::new(0),
            }
        }

        fn scored(content: &str, degraded: bool) -> ScoredMemory {
            let mut explanation = RetrievalExplanation::empty(
                RetrievalArtifactKind::RecallMemory,
                Uuid::new_v4().to_string(),
                "user-1".to_string(),
            );
            if !degraded {
                explanation.degraded_state = None;
            }
            ScoredMemory {
                entry: MemoryEntry {
                    id: Uuid::new_v4(),
                    memory_type: MemoryType::Semantic,
                    content: content.to_string(),
                    content_hash: MemoryPolicies::content_hash(content),
                    source: Some("test".to_string()),
                    source_type: None,
                    session_id: None,
                    user_id: Some("user-1".to_string()),
                    namespace: "user-1".to_string(),
                    importance: 0.8,
                    confidence: 0.9,
                    access_count: 0,
                    last_accessed: None,
                    created_at: Utc::now(),
                    expires_at: None,
                    metadata: json!({}),
                },
                score: 0.8,
                explanation,
            }
        }
    }

    #[async_trait]
    impl MemoryStore for RecordingMemoryStore {
        async fn store(&self, _entry: MemoryEntry) -> Result<()> {
            Ok(())
        }

        async fn search(&self, _query: &MemoryQuery) -> Result<Vec<ScoredMemory>> {
            *self.search_calls.lock().unwrap() += 1;
            Ok(vec![Self::scored("lexical fallback", true)])
        }

        async fn search_with_embedding(
            &self,
            _query: &MemoryQuery,
            _query_embedding: &[f32],
        ) -> Result<Vec<ScoredMemory>> {
            *self.hybrid_calls.lock().unwrap() += 1;
            Ok(vec![Self::scored("hybrid search", false)])
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

    struct StaticEmbeddingProvider;

    #[async_trait]
    impl EmbeddingProvider for StaticEmbeddingProvider {
        async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
            Ok(texts.iter().map(|_| vec![1.0, 0.0, 0.0]).collect())
        }

        fn dimensions(&self) -> usize {
            3
        }

        fn model_id(&self) -> &str {
            "static-test"
        }
    }

    fn tool_ctx() -> ToolContext {
        ToolContext {
            session_id: Uuid::new_v4().to_string(),
            user_id: "user-1".to_string(),
            workspace_path: None,
        }
    }

    #[tokio::test]
    async fn memory_search_uses_query_embeddings_when_service_is_available() {
        let store = Arc::new(RecordingMemoryStore::new());
        let embedding_service =
            Arc::new(EmbeddingService::new(Arc::new(StaticEmbeddingProvider), 1));
        let tool = MemorySearchTool::new(store.clone(), Some(embedding_service));

        let output = tool
            .execute(json!({"query":"ownership","limit":2}), &tool_ctx())
            .await
            .expect("tool execution should succeed");

        assert!(!output.is_error);
        assert_eq!(*store.hybrid_calls.lock().unwrap(), 1);
        assert_eq!(*store.search_calls.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn memory_search_reports_degraded_retrieval_when_embeddings_are_missing() {
        let store = Arc::new(RecordingMemoryStore::new());
        let tool = MemorySearchTool::new(store.clone(), None);

        let output = tool
            .execute(json!({"query":"ownership","limit":2}), &tool_ctx())
            .await
            .expect("tool execution should succeed");

        assert!(!output.is_error);
        assert!(output.content.contains("degraded"));
        assert!(output.content.contains("vector"));
        assert_eq!(*store.search_calls.lock().unwrap(), 1);
    }
}

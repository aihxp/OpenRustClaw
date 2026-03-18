//! Agent execution loop.
//!
//! Receives a message, builds context, calls LLM, processes tool calls,
//! and returns the response.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use openrustclaw_core::error::Result;
use openrustclaw_core::traits::{CoreMemoryStore, LlmProvider, MemoryStore, ToolContext};
use openrustclaw_core::types::{CompletionRequest, CoreEntry, FinishReason, Message};
use openrustclaw_memory::WorkspaceArtifactRegistry;
use tracing::{debug, info};

use crate::prompt::build_system_prompt;
use crate::tools::ToolRegistry;

/// The agent runtime that orchestrates message handling.
pub struct AgentRuntime {
    provider: Arc<dyn LlmProvider>,
    tool_registry: Arc<ToolRegistry>,
    agent_name: String,
    max_tool_iterations: usize,
    memory_store: Option<Arc<dyn MemoryStore>>,
    core_memory_store: Option<Arc<dyn CoreMemoryStore>>,
    workspace_path: Option<PathBuf>,
}

impl AgentRuntime {
    /// Create a new AgentRuntime with the given components.
    ///
    /// # Arguments
    ///
    /// * `provider` - The LLM provider to use for completions
    /// * `tool_registry` - The registry of available tools
    /// * `agent_name` - The name of the agent (used in system prompt)
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        tool_registry: Arc<ToolRegistry>,
        agent_name: String,
    ) -> Self {
        Self {
            provider,
            tool_registry,
            agent_name,
            max_tool_iterations: 10,
            memory_store: None,
            core_memory_store: None,
            workspace_path: None,
        }
    }

    /// Create a new AgentRuntime with memory stores.
    ///
    /// This constructor creates a runtime with memory tools pre-configured
    /// in the tool registry.
    ///
    /// # Arguments
    ///
    /// * `provider` - The LLM provider to use for completions
    /// * `agent_name` - The name of the agent (used in system prompt)
    /// * `memory_store` - The memory store for recall memory operations
    /// * `core_memory_store` - The core memory store for persistent key-value storage
    pub fn with_memory_stores(
        provider: Arc<dyn LlmProvider>,
        agent_name: String,
        memory_store: Arc<dyn MemoryStore>,
        core_memory_store: Arc<dyn CoreMemoryStore>,
    ) -> Self {
        let tool_registry = Arc::new(ToolRegistry::with_memory_tools(
            memory_store.clone(),
            core_memory_store.clone(),
        ));

        Self {
            provider,
            tool_registry,
            agent_name,
            max_tool_iterations: 10,
            memory_store: Some(memory_store),
            core_memory_store: Some(core_memory_store),
            workspace_path: None,
        }
    }

    pub fn with_workspace_path(mut self, workspace_path: impl Into<PathBuf>) -> Self {
        self.workspace_path = Some(workspace_path.into());
        self
    }

    /// Set the maximum number of tool iterations allowed per request.
    ///
    /// Default is 10. Set to 0 to disable tool use entirely.
    pub fn with_max_tool_iterations(mut self, max: usize) -> Self {
        self.max_tool_iterations = max;
        self
    }

    /// Get a reference to the memory store, if configured.
    pub fn memory_store(&self) -> Option<&Arc<dyn MemoryStore>> {
        self.memory_store.as_ref()
    }

    /// Get a reference to the core memory store, if configured.
    pub fn core_memory_store(&self) -> Option<&Arc<dyn CoreMemoryStore>> {
        self.core_memory_store.as_ref()
    }

    /// Get a reference to the tool registry.
    pub fn tool_registry(&self) -> &Arc<ToolRegistry> {
        &self.tool_registry
    }

    /// Process a conversation and return the agent's response.
    ///
    /// # Arguments
    ///
    /// * `messages` - The conversation history
    /// * `core_memory` - Core memory entries to include in the system prompt
    /// * `session_id` - The unique session identifier
    /// * `user_id` - The user identifier
    pub async fn process(
        &self,
        messages: &[Message],
        core_memory: &[CoreEntry],
        session_id: &str,
        user_id: &str,
    ) -> Result<AgentResponse> {
        let tools = self.tool_registry.definitions();
        let supplemental_instructions = self
            .workspace_path
            .as_ref()
            .and_then(|path| {
                WorkspaceArtifactRegistry::resolve(path, self.provider.model_id()).ok()
            })
            .map(|bundle| bundle.merged_instructions);
        let system_prompt = build_system_prompt(
            &self.agent_name,
            core_memory,
            &tools,
            supplemental_instructions.as_deref(),
        );

        let ctx = ToolContext {
            session_id: session_id.to_string(),
            user_id: user_id.to_string(),
            workspace_path: self
                .workspace_path
                .as_ref()
                .map(|path| path.display().to_string()),
        };

        let mut conversation: Vec<Message> = messages.to_vec();
        let mut total_tool_calls = 0;
        let mut executed_tool_call_ids = HashSet::new();

        loop {
            let request = CompletionRequest {
                messages: conversation.clone(),
                model: Some(self.provider.model_id().to_string()),
                max_tokens: Some(4096),
                temperature: Some(0.7),
                tools: if tools.is_empty() {
                    None
                } else {
                    Some(tools.clone())
                },
                system_prompt: Some(system_prompt.clone()),
                stream: false,
            };

            let response = self.provider.complete(request).await?;

            match response.finish_reason {
                FinishReason::Stop | FinishReason::MaxTokens | FinishReason::ContentFilter => {
                    info!(
                        provider = %response.provider,
                        model = %response.model,
                        tokens = response.usage.total_tokens,
                        "Agent response complete"
                    );
                    return Ok(AgentResponse {
                        message: response.message,
                        usage: response.usage,
                        tool_calls_made: total_tool_calls,
                    });
                }
                FinishReason::ToolUse => {
                    let tool_calls = response.message.tool_calls.clone().unwrap_or_default();
                    let new_tool_calls: Vec<_> = tool_calls
                        .into_iter()
                        .filter(|call| executed_tool_call_ids.insert(call.id.clone()))
                        .collect();

                    if new_tool_calls.is_empty() {
                        info!("Provider reported tool use but no new tool calls were available");
                        return Ok(AgentResponse {
                            message: response.message,
                            usage: response.usage,
                            tool_calls_made: total_tool_calls,
                        });
                    }

                    total_tool_calls += new_tool_calls.len();
                    if total_tool_calls > self.max_tool_iterations {
                        info!("Max tool iterations reached");
                        return Ok(AgentResponse {
                            message: response.message,
                            usage: response.usage,
                            tool_calls_made: total_tool_calls,
                        });
                    }

                    // Add assistant message with tool calls
                    conversation.push(response.message.clone());

                    // Execute tool calls
                    for call in &new_tool_calls {
                        debug!(tool = %call.name, "Executing tool call");
                        let output = match self.tool_registry.execute(call, &ctx).await {
                            Ok(output) => output,
                            Err(err) => openrustclaw_core::types::ToolOutput {
                                tool_call_id: call.id.clone(),
                                content: format!("Error: {}", err),
                                is_error: true,
                            },
                        };
                        conversation.push(Message::tool(&output.tool_call_id, &output.content));
                    }
                }
            }
        }
    }
}

/// The result of agent processing.
#[derive(Debug)]
pub struct AgentResponse {
    pub message: Message,
    pub usage: openrustclaw_core::types::TokenUsage,
    pub tool_calls_made: usize,
}

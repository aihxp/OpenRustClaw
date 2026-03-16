//! Agent execution loop.
//!
//! Receives a message, builds context, calls LLM, processes tool calls,
//! and returns the response.

use std::sync::Arc;

use openrustclaw_core::error::Result;
use openrustclaw_core::traits::{LlmProvider, ToolContext};
use openrustclaw_core::types::{CompletionRequest, CoreEntry, FinishReason, Message};
use tracing::{debug, info};

use crate::prompt::build_system_prompt;
use crate::tools::ToolRegistry;

/// The agent runtime that orchestrates message handling.
pub struct AgentRuntime {
    provider: Arc<dyn LlmProvider>,
    tool_registry: Arc<ToolRegistry>,
    agent_name: String,
    max_tool_iterations: usize,
}

impl AgentRuntime {
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
        }
    }

    /// Process a conversation and return the agent's response.
    pub async fn process(
        &self,
        messages: &[Message],
        core_memory: &[CoreEntry],
        session_id: &str,
        user_id: &str,
    ) -> Result<AgentResponse> {
        let tools = self.tool_registry.definitions();
        let system_prompt = build_system_prompt(&self.agent_name, core_memory, &tools);

        let ctx = ToolContext {
            session_id: session_id.to_string(),
            user_id: user_id.to_string(),
            workspace_path: None,
        };

        let mut conversation: Vec<Message> = messages.to_vec();
        let mut total_tool_calls = 0;

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
                    total_tool_calls += 1;
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
                    if let Some(ref tool_calls) = response.message.tool_calls {
                        for call in tool_calls {
                            debug!(tool = %call.name, "Executing tool call");
                            let output = self.tool_registry.execute(call, &ctx).await?;
                            conversation.push(Message::tool(&output.tool_call_id, &output.content));
                        }
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

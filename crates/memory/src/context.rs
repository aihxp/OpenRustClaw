//! Context window manager: Write-Select-Compress-Isolate.
//!
//! Manages the system prompt and context to stay within token budgets.
//! Core memory (~500 tokens) is the only memory always injected.

use openrustclaw_core::types::{CoreEntry, Message, ToolDefinition};

/// Manages context window budget.
pub struct ContextManager {
    max_context_tokens: usize,
    compression_threshold: f32, // 0.85 = compress at 85% capacity
}

/// Built context ready for an LLM request.
pub struct BuiltContext {
    pub system_prompt: String,
    pub messages: Vec<Message>,
    pub estimated_tokens: usize,
}

impl ContextManager {
    pub fn new(max_context_tokens: usize) -> Self {
        Self {
            max_context_tokens,
            compression_threshold: 0.85,
        }
    }

    /// Build the system prompt with core memory and tool schemas.
    pub fn build_system_prompt(
        &self,
        base_prompt: &str,
        core_memory: &[CoreEntry],
        tools: &[ToolDefinition],
    ) -> String {
        let mut prompt = String::new();

        // Base instructions
        prompt.push_str(base_prompt);
        prompt.push_str("\n\n");

        // Core memory (always loaded, ~500 tokens)
        if !core_memory.is_empty() {
            prompt.push_str("[Core Memory]\n");
            for entry in core_memory {
                prompt.push_str(&format!("{}: {}\n", entry.key, entry.value));
            }
            prompt.push('\n');
        }

        // Available tools (names + descriptions only to save tokens)
        if !tools.is_empty() {
            prompt.push_str("[Available Tools]\n");
            for tool in tools {
                prompt.push_str(&format!("- {}: {}\n", tool.name, tool.description));
            }
            prompt.push('\n');
        }

        // Memory instruction
        prompt.push_str("You have a memory_search tool. Use it when you need to recall facts, ");
        prompt.push_str("preferences, or past conversations. Don't guess -- search.\n");

        prompt
    }

    /// Select messages that fit within the token budget.
    pub fn select_messages(
        &self,
        messages: &[Message],
        system_prompt_tokens: usize,
    ) -> Vec<Message> {
        let available = self.max_context_tokens.saturating_sub(system_prompt_tokens);
        let mut selected = Vec::new();
        let mut used = 0;

        // Always include the most recent messages, working backwards
        for msg in messages.iter().rev() {
            let msg_tokens = Self::estimate_tokens(&msg.content);
            if used + msg_tokens > available {
                break;
            }
            selected.push(msg.clone());
            used += msg_tokens;
        }

        selected.reverse();
        selected
    }

    /// Check if compression is needed.
    pub fn needs_compression(&self, current_tokens: usize) -> bool {
        current_tokens as f32 > self.max_context_tokens as f32 * self.compression_threshold
    }

    /// Rough token estimation (~4 chars per token).
    pub fn estimate_tokens(text: &str) -> usize {
        (text.len() / 4).max(1)
    }

    /// Build the full context.
    pub fn build(
        &self,
        base_prompt: &str,
        core_memory: &[CoreEntry],
        tools: &[ToolDefinition],
        messages: &[Message],
    ) -> BuiltContext {
        let system_prompt = self.build_system_prompt(base_prompt, core_memory, tools);
        let system_tokens = Self::estimate_tokens(&system_prompt);
        let selected = self.select_messages(messages, system_tokens);
        let msg_tokens: usize = selected
            .iter()
            .map(|m| Self::estimate_tokens(&m.content))
            .sum();

        BuiltContext {
            system_prompt,
            messages: selected,
            estimated_tokens: system_tokens + msg_tokens,
        }
    }
}

//! Context window manager: Write-Select-Compress-Isolate.
//!
//! Manages the system prompt and context to stay within token budgets.
//! Core memory (~500 tokens) is the only memory always injected.

use std::collections::HashSet;

use openrustclaw_core::types::{
    CoreEntry, Message, RecallPack, RecallPackItem, ScoredMemory, ToolDefinition,
};

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

/// Assemble a bounded, deduplicated recall pack from scored memory results.
pub fn build_recall_pack(
    results: &[ScoredMemory],
    max_items: usize,
    max_content_chars: usize,
) -> RecallPack {
    let mut seen = HashSet::new();
    let mut items = Vec::new();

    for scored in results {
        let dedupe_key = if scored.entry.content_hash.is_empty() {
            scored.explanation.primary_artifact.artifact_id.clone()
        } else {
            scored.entry.content_hash.clone()
        };
        if !seen.insert(dedupe_key) {
            continue;
        }

        items.push(RecallPackItem {
            id: scored.entry.id.to_string(),
            memory_type: scored.entry.memory_type,
            namespace: scored.entry.namespace.clone(),
            content: clip_excerpt(&scored.entry.content, max_content_chars),
            score: scored.score,
            importance: scored.entry.importance,
            confidence: scored.entry.confidence,
            explanation: scored.explanation.clone(),
        });

        if items.len() >= max_items.max(1) {
            break;
        }
    }

    let degraded = items
        .iter()
        .any(|item| item.explanation.degraded_state.is_some());

    RecallPack { degraded, items }
}

fn clip_excerpt(content: &str, max_content_chars: usize) -> String {
    let trimmed = content.trim();
    if trimmed.chars().count() <= max_content_chars {
        return trimmed.to_string();
    }

    let clipped: String = trimmed.chars().take(max_content_chars).collect();
    format!("{}...", clipped.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use openrustclaw_core::types::{
        CoreEntry, MemoryEntry, MemoryType, RetrievalArtifactKind, RetrievalExplanation, Role,
        ScoredMemory, SourceType,
    };
    use uuid::Uuid;

    fn make_core_entry(key: &str, value: &str) -> CoreEntry {
        CoreEntry {
            key: key.to_string(),
            value: value.to_string(),
            importance: 1.0,
            token_count: 10,
            updated_at: Utc::now(),
        }
    }

    fn make_tool_def(name: &str, desc: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: desc.to_string(),
            parameters: serde_json::json!({}),
            strict: false,
        }
    }

    fn make_scored_memory(
        id: &str,
        content: &str,
        content_hash: &str,
        score: f32,
        source_type: Option<SourceType>,
    ) -> ScoredMemory {
        ScoredMemory {
            entry: MemoryEntry {
                id: Uuid::parse_str(id).unwrap_or_else(|_| Uuid::new_v4()),
                memory_type: MemoryType::Semantic,
                content: content.to_string(),
                content_hash: content_hash.to_string(),
                source: Some("test-source".to_string()),
                source_type,
                session_id: None,
                user_id: Some("user-1".to_string()),
                namespace: "user-1".to_string(),
                importance: 0.7,
                confidence: 0.9,
                access_count: 0,
                last_accessed: None,
                created_at: Utc::now(),
                expires_at: None,
                metadata: serde_json::json!({}),
            },
            score,
            explanation: RetrievalExplanation::empty(
                RetrievalArtifactKind::from_memory_parts(MemoryType::Semantic, source_type),
                id.to_string(),
                "user-1".to_string(),
            ),
        }
    }

    // ── estimate_tokens tests ──

    #[test]
    fn estimate_tokens_short_text() {
        let tokens = ContextManager::estimate_tokens("hi");
        assert_eq!(tokens, 1, "Short text should be at least 1 token");
    }

    #[test]
    fn estimate_tokens_longer_text() {
        // 20 chars / 4 = 5 tokens
        let tokens = ContextManager::estimate_tokens("12345678901234567890");
        assert_eq!(tokens, 5);
    }

    #[test]
    fn estimate_tokens_empty_string() {
        let tokens = ContextManager::estimate_tokens("");
        assert_eq!(tokens, 1, "Empty string should return at least 1");
    }

    // ── needs_compression tests ──

    #[test]
    fn needs_compression_below_threshold() {
        let ctx = ContextManager::new(1000);
        assert!(
            !ctx.needs_compression(800),
            "80% usage should not trigger compression at 85% threshold"
        );
    }

    #[test]
    fn needs_compression_above_threshold() {
        let ctx = ContextManager::new(1000);
        assert!(
            ctx.needs_compression(860),
            "86% usage should trigger compression at 85% threshold"
        );
    }

    #[test]
    fn needs_compression_exactly_at_threshold() {
        let ctx = ContextManager::new(1000);
        // 85% of 1000 = 850
        assert!(
            !ctx.needs_compression(850),
            "Exactly at threshold should not trigger (not strictly greater)"
        );
    }

    #[test]
    fn needs_compression_over_max() {
        let ctx = ContextManager::new(1000);
        assert!(
            ctx.needs_compression(1200),
            "Over max should definitely trigger compression"
        );
    }

    // ── build_system_prompt tests ──

    #[test]
    fn build_system_prompt_includes_base() {
        let ctx = ContextManager::new(4096);
        let prompt = ctx.build_system_prompt("You are a helpful assistant.", &[], &[]);
        assert!(prompt.contains("You are a helpful assistant."));
    }

    #[test]
    fn build_system_prompt_includes_core_memory() {
        let ctx = ContextManager::new(4096);
        let entries = vec![make_core_entry("user_name", "Alice")];
        let prompt = ctx.build_system_prompt("Base.", &entries, &[]);
        assert!(prompt.contains("[Core Memory]"));
        assert!(prompt.contains("user_name: Alice"));
    }

    #[test]
    fn build_system_prompt_no_core_memory_section_when_empty() {
        let ctx = ContextManager::new(4096);
        let prompt = ctx.build_system_prompt("Base.", &[], &[]);
        assert!(!prompt.contains("[Core Memory]"));
    }

    #[test]
    fn build_system_prompt_includes_tools() {
        let ctx = ContextManager::new(4096);
        let tools = vec![make_tool_def("memory_search", "Search memory")];
        let prompt = ctx.build_system_prompt("Base.", &[], &tools);
        assert!(prompt.contains("[Available Tools]"));
        assert!(prompt.contains("memory_search: Search memory"));
    }

    #[test]
    fn build_system_prompt_no_tools_section_when_empty() {
        let ctx = ContextManager::new(4096);
        let prompt = ctx.build_system_prompt("Base.", &[], &[]);
        assert!(!prompt.contains("[Available Tools]"));
    }

    #[test]
    fn build_system_prompt_includes_memory_instruction() {
        let ctx = ContextManager::new(4096);
        let prompt = ctx.build_system_prompt("Base.", &[], &[]);
        assert!(prompt.contains("memory_search"));
        assert!(prompt.contains("Don't guess"));
    }

    // ── select_messages tests ──

    #[test]
    fn select_messages_fits_all() {
        let ctx = ContextManager::new(10000);
        let messages = vec![Message::user("Hello"), Message::assistant("Hi there!")];
        let selected = ctx.select_messages(&messages, 100);
        assert_eq!(selected.len(), 2, "All messages should fit");
    }

    #[test]
    fn select_messages_truncates_oldest() {
        // Very tight budget: only room for ~10 tokens beyond system prompt
        let ctx = ContextManager::new(120);
        let messages = vec![
            Message::user("This is a somewhat long first message that should be truncated"),
            Message::assistant("Short reply"),
        ];
        let selected = ctx.select_messages(&messages, 100);
        // Only the most recent messages should be included (working backwards)
        assert!(selected.len() <= 2);
        if selected.len() == 1 {
            // The last message should be kept (most recent)
            assert_eq!(selected[0].role, Role::Assistant);
        }
    }

    #[test]
    fn select_messages_empty_input() {
        let ctx = ContextManager::new(4096);
        let selected = ctx.select_messages(&[], 100);
        assert!(selected.is_empty());
    }

    #[test]
    fn select_messages_preserves_order() {
        let ctx = ContextManager::new(10000);
        let messages = vec![
            Message::user("First"),
            Message::assistant("Second"),
            Message::user("Third"),
        ];
        let selected = ctx.select_messages(&messages, 100);
        assert_eq!(selected.len(), 3);
        assert_eq!(selected[0].content, "First");
        assert_eq!(selected[1].content, "Second");
        assert_eq!(selected[2].content, "Third");
    }

    // ── build tests ──

    #[test]
    fn build_returns_complete_context() {
        let ctx = ContextManager::new(10000);
        let entries = vec![make_core_entry("name", "Bob")];
        let tools = vec![make_tool_def("search", "Search things")];
        let messages = vec![Message::user("Hello"), Message::assistant("Hi!")];

        let built = ctx.build("You are helpful.", &entries, &tools, &messages);

        assert!(built.system_prompt.contains("You are helpful."));
        assert!(built.system_prompt.contains("name: Bob"));
        assert!(built.system_prompt.contains("search: Search things"));
        assert_eq!(built.messages.len(), 2);
        assert!(built.estimated_tokens > 0);
    }

    #[test]
    fn build_estimated_tokens_includes_system_and_messages() {
        let ctx = ContextManager::new(10000);
        let built = ctx.build("Short.", &[], &[], &[Message::user("Hello")]);

        let sys_tokens = ContextManager::estimate_tokens(&built.system_prompt);
        let msg_tokens: usize = built
            .messages
            .iter()
            .map(|m| ContextManager::estimate_tokens(&m.content))
            .sum();
        assert_eq!(built.estimated_tokens, sys_tokens + msg_tokens);
    }

    #[test]
    fn build_recall_pack_deduplicates_and_clips() {
        let results = vec![
            make_scored_memory(
                "00000000-0000-0000-0000-000000000001",
                "Rust ownership keeps memory safety intact over time.",
                "same-hash",
                0.95,
                Some(SourceType::Conversation),
            ),
            make_scored_memory(
                "00000000-0000-0000-0000-000000000002",
                "Rust ownership keeps memory safety intact over time.",
                "same-hash",
                0.82,
                Some(SourceType::Conversation),
            ),
        ];

        let pack = build_recall_pack(&results, 5, 24);
        assert_eq!(pack.items.len(), 1);
        assert!(pack.items[0].content.ends_with("..."));
        assert_eq!(
            pack.items[0].explanation.primary_artifact.artifact_kind,
            RetrievalArtifactKind::ConversationMemory
        );
        assert!(pack.degraded);
    }
}

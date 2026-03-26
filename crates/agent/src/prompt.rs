//! System prompt builder with token budgeting.

use chrono::Utc;
use openrustclaw_core::types::{CoreEntry, ToolDefinition};

/// Build the system prompt with core memory and tool metadata.
pub fn build_system_prompt(
    agent_name: &str,
    core_memory: &[CoreEntry],
    tools: &[ToolDefinition],
    supplemental_instructions: Option<&str>,
) -> String {
    let mut prompt = String::new();

    // Agent identity
    prompt.push_str(&format!(
        "You are {}, an AI assistant powered by OpenRustClaw.\n\n",
        agent_name
    ));

    // Runtime metadata
    prompt.push_str(&format!(
        "Current time: {}\n\n",
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    if let Some(supplemental_instructions) = supplemental_instructions {
        let trimmed = supplemental_instructions.trim();
        if !trimmed.is_empty() {
            prompt.push_str("[Workspace Guidance]\n");
            prompt.push_str(trimmed);
            prompt.push_str("\n\n");
        }
    }

    // Core memory (always loaded, ~500 tokens max)
    if !core_memory.is_empty() {
        prompt.push_str("[Core Memory]\n");
        for entry in core_memory {
            prompt.push_str(&format!("{}: {}\n", entry.key, entry.value));
        }
        prompt.push('\n');
    }

    // Tool descriptions
    if !tools.is_empty() {
        prompt.push_str("[Available Tools]\n");
        for tool in tools {
            prompt.push_str(&format!("- {}: {}\n", tool.name, tool.description));
        }
        prompt.push('\n');
    }

    let tool_names: std::collections::HashSet<&str> =
        tools.iter().map(|tool| tool.name.as_str()).collect();
    if tool_names.contains("memory_search") {
        prompt.push_str("You have a memory_search tool. Use it when you need to recall facts, ");
        prompt.push_str("preferences, or past conversations. Don't guess, search.\n");
    }
    if tool_names.contains("memory_store") {
        prompt.push_str("You have a memory_store tool. Use it to remember important information ");
        prompt.push_str("for future conversations.\n");
    }
    if tool_names.contains("core_memory_update") {
        prompt
            .push_str("You have a core_memory_update tool. Use it sparingly to maintain durable ");
        prompt.push_str("profile-style memory when the user clearly wants it updated.\n");
    }

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prompt_only_mentions_tools_that_exist() {
        let prompt = build_system_prompt(
            "Assistant",
            &[],
            &[ToolDefinition {
                name: "memory_search".to_string(),
                description: "search".to_string(),
                parameters: json!({}),
                strict: false,
            }],
            None,
        );

        assert!(prompt.contains("memory_search"));
        assert!(!prompt.contains("memory_store tool"));
        assert!(!prompt.contains("core_memory_update tool"));
    }
}

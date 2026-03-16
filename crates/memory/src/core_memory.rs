//! Core memory: tiny, curated, always-loaded context (~500 tokens).
//!
//! The ONLY memory tier that is injected into the system prompt.
//! Hard capped at ~500 tokens and ~20 entries.

use chrono::Utc;
use openrustclaw_core::types::CoreEntry;

/// Manages core memory entries for system prompt injection.
pub struct CoreMemoryManager {
    max_tokens: usize,
    max_entries: usize,
}

impl CoreMemoryManager {
    pub fn new(max_tokens: usize, max_entries: usize) -> Self {
        Self {
            max_tokens,
            max_entries,
        }
    }

    /// Render core memory entries as a formatted string for system prompt.
    pub fn render(entries: &[CoreEntry]) -> String {
        if entries.is_empty() {
            return String::new();
        }
        let mut output = String::from("[Core Memory]\n");
        for entry in entries {
            output.push_str(&format!("{}: {}\n", entry.key, entry.value));
        }
        output
    }

    /// Check if adding a new entry would exceed the token budget.
    pub fn would_exceed_budget(
        &self,
        current_entries: &[CoreEntry],
        new_entry: &CoreEntry,
    ) -> bool {
        let current_tokens: usize = current_entries.iter().map(|e| e.token_count).sum();
        current_tokens + new_entry.token_count > self.max_tokens
            || current_entries.len() >= self.max_entries
    }

    /// Find the lowest-importance entry that could be evicted.
    pub fn eviction_candidate(entries: &[CoreEntry]) -> Option<usize> {
        entries
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.importance
                    .partial_cmp(&b.importance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
    }

    /// Create a new core entry.
    pub fn new_entry(key: &str, value: &str, importance: f32) -> CoreEntry {
        // Rough token estimate: ~1 token per 4 chars
        let token_count = (key.len() + value.len() + 2) / 4 + 1;
        CoreEntry {
            key: key.to_string(),
            value: value.to_string(),
            importance,
            token_count,
            updated_at: Utc::now(),
        }
    }

    /// Get max tokens.
    pub fn max_tokens(&self) -> usize {
        self.max_tokens
    }

    /// Get max entries.
    pub fn max_entries(&self) -> usize {
        self.max_entries
    }
}

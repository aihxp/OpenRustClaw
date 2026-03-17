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

#[cfg(test)]
mod tests {
    use super::*;
    use openrustclaw_core::types::CoreEntry;

    fn make_entry(key: &str, value: &str, importance: f32, token_count: usize) -> CoreEntry {
        CoreEntry {
            key: key.to_string(),
            value: value.to_string(),
            importance,
            token_count,
            updated_at: Utc::now(),
        }
    }

    // ── render tests ──

    #[test]
    fn render_empty_returns_empty_string() {
        let result = CoreMemoryManager::render(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn render_single_entry() {
        let entries = vec![make_entry("name", "Alice", 1.0, 5)];
        let result = CoreMemoryManager::render(&entries);
        assert!(result.starts_with("[Core Memory]\n"));
        assert!(result.contains("name: Alice"));
    }

    #[test]
    fn render_multiple_entries_preserves_order() {
        let entries = vec![
            make_entry("name", "Alice", 1.0, 5),
            make_entry("lang", "Rust", 0.8, 4),
            make_entry("role", "Engineer", 0.9, 6),
        ];
        let result = CoreMemoryManager::render(&entries);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[0], "[Core Memory]");
        assert_eq!(lines[1], "name: Alice");
        assert_eq!(lines[2], "lang: Rust");
        assert_eq!(lines[3], "role: Engineer");
    }

    // ── would_exceed_budget tests ──

    #[test]
    fn budget_not_exceeded_when_under_limits() {
        let mgr = CoreMemoryManager::new(100, 10);
        let current = vec![make_entry("k", "v", 1.0, 20)];
        let new = make_entry("k2", "v2", 1.0, 30);
        assert!(!mgr.would_exceed_budget(&current, &new));
    }

    #[test]
    fn budget_exceeded_by_tokens() {
        let mgr = CoreMemoryManager::new(50, 100);
        let current = vec![make_entry("k", "v", 1.0, 30)];
        let new = make_entry("k2", "v2", 1.0, 25);
        // 30 + 25 = 55 > 50
        assert!(mgr.would_exceed_budget(&current, &new));
    }

    #[test]
    fn budget_exceeded_by_entry_count() {
        let mgr = CoreMemoryManager::new(10000, 2);
        let current = vec![
            make_entry("k1", "v1", 1.0, 5),
            make_entry("k2", "v2", 1.0, 5),
        ];
        let new = make_entry("k3", "v3", 1.0, 5);
        // current.len() == 2 == max_entries
        assert!(mgr.would_exceed_budget(&current, &new));
    }

    #[test]
    fn budget_exactly_at_token_limit() {
        let mgr = CoreMemoryManager::new(50, 100);
        let current = vec![make_entry("k", "v", 1.0, 25)];
        let new = make_entry("k2", "v2", 1.0, 25);
        // 25 + 25 = 50 == max_tokens, not exceeded
        assert!(!mgr.would_exceed_budget(&current, &new));
    }

    #[test]
    fn budget_one_token_over() {
        let mgr = CoreMemoryManager::new(50, 100);
        let current = vec![make_entry("k", "v", 1.0, 25)];
        let new = make_entry("k2", "v2", 1.0, 26);
        // 25 + 26 = 51 > 50
        assert!(mgr.would_exceed_budget(&current, &new));
    }

    #[test]
    fn budget_empty_current_entries() {
        let mgr = CoreMemoryManager::new(100, 10);
        let new = make_entry("k", "v", 1.0, 50);
        assert!(!mgr.would_exceed_budget(&[], &new));
    }

    // ── eviction_candidate tests ──

    #[test]
    fn eviction_candidate_returns_lowest_importance() {
        let entries = vec![
            make_entry("high", "v", 0.9, 5),
            make_entry("low", "v", 0.1, 5),
            make_entry("mid", "v", 0.5, 5),
        ];
        let idx = CoreMemoryManager::eviction_candidate(&entries);
        assert_eq!(idx, Some(1), "Should evict the entry with importance 0.1");
    }

    #[test]
    fn eviction_candidate_empty_returns_none() {
        let idx = CoreMemoryManager::eviction_candidate(&[]);
        assert_eq!(idx, None);
    }

    #[test]
    fn eviction_candidate_single_entry() {
        let entries = vec![make_entry("only", "v", 0.5, 5)];
        let idx = CoreMemoryManager::eviction_candidate(&entries);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn eviction_candidate_equal_importance_returns_first() {
        let entries = vec![
            make_entry("a", "v", 0.5, 5),
            make_entry("b", "v", 0.5, 5),
            make_entry("c", "v", 0.5, 5),
        ];
        let idx = CoreMemoryManager::eviction_candidate(&entries);
        // min_by returns the first minimum found
        assert_eq!(idx, Some(0));
    }

    // ── new_entry tests ──

    #[test]
    fn new_entry_basic_fields() {
        let entry = CoreMemoryManager::new_entry("name", "Alice", 0.9);
        assert_eq!(entry.key, "name");
        assert_eq!(entry.value, "Alice");
        assert_eq!(entry.importance, 0.9);
        assert!(entry.token_count > 0, "Token count should be positive");
    }

    #[test]
    fn new_entry_token_count_estimation() {
        // Token count = (key.len() + value.len() + 2) / 4 + 1
        let entry = CoreMemoryManager::new_entry("k", "val", 1.0);
        // (1 + 3 + 2) / 4 + 1 = 6/4 + 1 = 1 + 1 = 2
        assert_eq!(entry.token_count, 2);
    }

    #[test]
    fn new_entry_long_value_has_larger_token_count() {
        let short = CoreMemoryManager::new_entry("k", "hi", 1.0);
        let long = CoreMemoryManager::new_entry("k", "this is a much longer value with many words", 1.0);
        assert!(
            long.token_count > short.token_count,
            "Longer values should produce higher token counts"
        );
    }

    // ── Constructor tests ──

    #[test]
    fn constructor_stores_limits() {
        let mgr = CoreMemoryManager::new(500, 20);
        assert_eq!(mgr.max_tokens(), 500);
        assert_eq!(mgr.max_entries(), 20);
    }
}

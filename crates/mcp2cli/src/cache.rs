//! Caching with TTL for tool discovery
//!
//! This module provides a thread-safe cache with configurable TTL
//! for storing tool lists and help information.

use crate::discovery::{ToolHelp, ToolSummary};
use crate::error::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Cache entry with expiration
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    value: T,
    expires_at: DateTime<Utc>,
}

impl<T> CacheEntry<T> {
    fn new(value: T, ttl: Duration) -> Self {
        Self {
            value,
            expires_at: Utc::now() + chrono::Duration::from_std(ttl).unwrap_or(chrono::Duration::hours(1)),
        }
    }

    fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Cached tool list
#[derive(Debug, Clone)]
pub struct CachedToolList {
    pub tools: Vec<ToolSummary>,
    pub cached_at: DateTime<Utc>,
}

/// Cached tool help
#[derive(Debug, Clone)]
pub struct CachedToolHelp {
    pub help: ToolHelp,
    pub cached_at: DateTime<Utc>,
}

/// Thread-safe cache with TTL
pub struct ToolCache {
    ttl: Duration,
    tool_lists: DashMap<String, CacheEntry<CachedToolList>>,
    tool_helps: DashMap<String, CacheEntry<CachedToolHelp>>,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl ToolCache {
    /// Create a new cache with the specified TTL
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            tool_lists: DashMap::new(),
            tool_helps: DashMap::new(),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Get or insert a tool list
    pub async fn get_or_insert<F, Fut>(&self, key: &str, f: F) -> Result<CachedToolList>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Vec<ToolSummary>>>,
    {
        // Check cache first
        if let Some(entry) = self.tool_lists.get(key) {
            if !entry.is_expired() {
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Ok(entry.value.clone());
            }
            // Expired entry, remove it
            drop(entry);
            self.tool_lists.remove(key);
        }

        // Cache miss - fetch the data
        self.misses.fetch_add(1, Ordering::Relaxed);
        let tools = f().await?;
        let cached = CachedToolList {
            tools,
            cached_at: Utc::now(),
        };

        // Store in cache
        let entry = CacheEntry::new(cached.clone(), self.ttl);
        self.tool_lists.insert(key.to_string(), entry);

        Ok(cached)
    }

    /// Get or insert a tool help
    pub async fn get_or_insert_tool_help<F, Fut>(&self, key: &str, f: F) -> Result<ToolHelp>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<ToolHelp>>,
    {
        // Check cache first
        if let Some(entry) = self.tool_helps.get(key) {
            if !entry.is_expired() {
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Ok(entry.value.help.clone());
            }
            // Expired entry, remove it
            drop(entry);
            self.tool_helps.remove(key);
        }

        // Cache miss - fetch the data
        self.misses.fetch_add(1, Ordering::Relaxed);
        let help = f().await?;
        let cached = CachedToolHelp {
            help: help.clone(),
            cached_at: Utc::now(),
        };

        // Store in cache
        let entry = CacheEntry::new(cached, self.ttl);
        self.tool_helps.insert(key.to_string(), entry);

        Ok(help)
    }

    /// Invalidate a specific cache entry
    pub fn invalidate(&self, key: &str) {
        self.tool_lists.remove(key);
        self.tool_helps.remove(key);
    }

    /// Invalidate all entries with the given prefix
    pub fn invalidate_prefix(&self, prefix: &str) {
        // Remove matching tool lists
        let keys_to_remove: Vec<_> = self
            .tool_lists
            .iter()
            .filter(|e| e.key().starts_with(prefix))
            .map(|e| e.key().clone())
            .collect();
        for key in keys_to_remove {
            self.tool_lists.remove(&key);
        }

        // Remove matching tool helps
        let keys_to_remove: Vec<_> = self
            .tool_helps
            .iter()
            .filter(|e| e.key().starts_with(prefix))
            .map(|e| e.key().clone())
            .collect();
        for key in keys_to_remove {
            self.tool_helps.remove(&key);
        }
    }

    /// Clear all cached data
    pub fn clear(&self) {
        self.tool_lists.clear();
        self.tool_helps.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entry_count: self.tool_lists.len() + self.tool_helps.len(),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
        }
    }

    /// Get the configured TTL
    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// Clean up expired entries (call periodically)
    pub fn cleanup_expired(&self) {
        let now = Utc::now();

        // Clean up expired tool lists
        let expired_keys: Vec<_> = self
            .tool_lists
            .iter()
            .filter(|e| e.expires_at < now)
            .map(|e| e.key().clone())
            .collect();
        for key in expired_keys {
            self.tool_lists.remove(&key);
        }

        // Clean up expired tool helps
        let expired_keys: Vec<_> = self
            .tool_helps
            .iter()
            .filter(|e| e.expires_at < now)
            .map(|e| e.key().clone())
            .collect();
        for key in expired_keys {
            self.tool_helps.remove(&key);
        }
    }
}

impl Default for ToolCache {
    fn default() -> Self {
        Self::new(Duration::from_secs(3600))
    }
}

/// Cache statistics
#[derive(Debug, Clone, Copy)]
pub struct CacheStats {
    /// Number of cached entries
    pub entry_count: usize,
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
}

impl CacheStats {
    /// Calculate hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Calculate miss rate (0.0 to 1.0)
    pub fn miss_rate(&self) -> f64 {
        1.0 - self.hit_rate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_get_or_insert() {
        let cache = ToolCache::new(Duration::from_secs(60));
        
        let result = cache
            .get_or_insert("test", || async {
                Ok(vec![ToolSummary::new("tool1", "Description")])
            })
            .await
            .unwrap();
        
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0].name, "tool1");
        
        // Second call should hit cache
        let result2 = cache
            .get_or_insert("test", || async {
                // This should not be called
                panic!("Should not be called");
            })
            .await
            .unwrap();
        
        assert_eq!(result2.tools.len(), 1);
        
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_cache_invalidate() {
        let cache = ToolCache::new(Duration::from_secs(60));
        
        cache
            .get_or_insert("key1", || async {
                Ok(vec![ToolSummary::new("tool1", "Desc")])
            })
            .await
            .unwrap();
        
        cache.invalidate("key1");
        
        let stats = cache.stats();
        assert_eq!(stats.entry_count, 0);
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = ToolCache::new(Duration::from_secs(60));
        
        cache
            .get_or_insert("key1", || async {
                Ok(vec![ToolSummary::new("tool1", "Desc")])
            })
            .await
            .unwrap();
        
        cache.clear();
        
        let stats = cache.stats();
        assert_eq!(stats.entry_count, 0);
    }

    #[test]
    fn test_cache_stats() {
        let stats = CacheStats {
            entry_count: 10,
            hits: 90,
            misses: 10,
        };
        
        assert!((stats.hit_rate() - 0.9).abs() < 0.001);
        assert!((stats.miss_rate() - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_cache_entry_expiration() {
        let entry = CacheEntry::new("value", Duration::from_secs(0));
        // Should be expired immediately (or very quickly)
        std::thread::sleep(Duration::from_millis(10));
        assert!(entry.is_expired());
    }
}

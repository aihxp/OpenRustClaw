//! Per-session rate limiting.

use openrustclaw_core::error::{Error, GatewayError, Result};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Per-session rate limiter using a sliding window.
pub struct RateLimiter {
    max_requests: u32,
    window: Duration,
    counters: HashMap<String, Vec<Instant>>,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(window_secs),
            counters: HashMap::new(),
        }
    }

    /// Check if a request is allowed for the given session.
    pub fn check(&mut self, session_id: &str) -> Result<()> {
        let now = Instant::now();
        let timestamps = self.counters.entry(session_id.to_string()).or_default();

        // Remove expired entries
        timestamps.retain(|t| now.duration_since(*t) < self.window);

        if timestamps.len() >= self.max_requests as usize {
            return Err(Error::Gateway(GatewayError::RateLimitExceeded {
                session_id: session_id.to_string(),
            }));
        }

        timestamps.push(now);
        Ok(())
    }

    /// Clear rate limit data for a session.
    pub fn clear(&mut self, session_id: &str) {
        self.counters.remove(session_id);
    }
}

//! Exponential backoff, max retries, dead-letter handling.

use std::time::Duration;

/// Calculate exponential backoff delay.
pub fn backoff_delay(retry_count: u32, base_delay_secs: u64, max_delay_secs: u64) -> Duration {
    let delay = base_delay_secs.saturating_mul(2u64.saturating_pow(retry_count));
    Duration::from_secs(delay.min(max_delay_secs))
}

/// Determine if a job should be sent to dead-letter queue.
pub fn should_dead_letter(consecutive_failures: u32, max_retries: u32) -> bool {
    consecutive_failures >= max_retries
}

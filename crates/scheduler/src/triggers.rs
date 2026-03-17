//! Trigger types for scheduled jobs.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Trigger configuration (stored as JSON in the DB).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerConfig {
    Interval { interval_secs: u64 },
    Event { event_name: String },
    Webhook { webhook_url: String },
    Dependency { depends_on: Vec<String> },
    Absolute { run_at: DateTime<Utc> },
}

/// Calculate the next run time based on trigger configuration.
pub fn calculate_next_run(
    trigger: &TriggerConfig,
    last_run: Option<DateTime<Utc>>,
) -> Option<DateTime<Utc>> {
    match trigger {
        TriggerConfig::Interval { interval_secs } => {
            let base = last_run.unwrap_or_else(Utc::now);
            Some(base + Duration::seconds(*interval_secs as i64))
        }
        TriggerConfig::Absolute { run_at } => {
            if last_run.is_some() {
                None // Absolute triggers fire once
            } else {
                Some(*run_at)
            }
        }
        TriggerConfig::Event { .. } => None, // Event-triggered, no scheduled time
        TriggerConfig::Webhook { .. } => None, // Webhook-triggered, no scheduled time
        TriggerConfig::Dependency { .. } => None, // Dependency-triggered
    }
}

use chrono::Utc;
use openrustclaw_core::config::AppConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelProbeStatus {
    Ready,
    Warning,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelProbeEntry {
    pub platform: String,
    pub enabled: bool,
    pub status: ChannelProbeStatus,
    pub probe_kind: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelHealthMonitorStatus {
    pub generated_at: String,
    pub health_monitor_enabled: bool,
    pub auto_restart_on_failure: bool,
    pub auto_restart_ready: bool,
    pub consecutive_failure_threshold: usize,
    pub current_consecutive_failures: usize,
    pub degraded: bool,
    pub failing_platforms: Vec<String>,
    pub last_failure_at: Option<String>,
    pub last_healthy_at: Option<String>,
    pub restart_requested: bool,
    pub restart_reason: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ChannelHealthMonitorService;

impl ChannelHealthMonitorService {
    pub fn new() -> Self {
        Self
    }

    pub fn enabled_channels(&self, config: &AppConfig) -> Vec<String> {
        let mut channels = Vec::new();
        if config.channels.telegram.enabled {
            channels.push("telegram".to_string());
        }
        if config.channels.discord.enabled {
            channels.push("discord".to_string());
        }
        if config.channels.slack.enabled {
            channels.push("slack".to_string());
        }
        if config.channels.whatsapp.enabled {
            channels.push("whatsapp".to_string());
        }
        if config.channels.teams.enabled {
            channels.push("teams".to_string());
        }
        if config.channels.mattermost.enabled {
            channels.push("mattermost".to_string());
        }
        if config.channels.google_chat.enabled {
            channels.push("google_chat".to_string());
        }
        if config.channels.google_meet.enabled {
            channels.push("google_meet".to_string());
        }
        if config.channels.gmail_pubsub.enabled {
            channels.push("gmail_pubsub".to_string());
        }
        if config.channels.signal.enabled {
            channels.push("signal".to_string());
        }
        if config.channels.matrix.enabled {
            channels.push("matrix".to_string());
        }
        if config.channels.imessage.enabled {
            channels.push("imessage".to_string());
        }
        channels
    }

    pub fn compose_status(
        &self,
        config: &AppConfig,
        auto_restart_ready: bool,
        previous: Option<&ChannelHealthMonitorStatus>,
        entries: &[ChannelProbeEntry],
    ) -> ChannelHealthMonitorStatus {
        self.compose_status_at(
            config,
            auto_restart_ready,
            previous,
            entries,
            Utc::now().to_rfc3339(),
        )
    }

    pub fn compose_status_at(
        &self,
        config: &AppConfig,
        auto_restart_ready: bool,
        previous: Option<&ChannelHealthMonitorStatus>,
        entries: &[ChannelProbeEntry],
        generated_at: String,
    ) -> ChannelHealthMonitorStatus {
        let failing_platforms = entries
            .iter()
            .filter(|entry| entry.enabled && entry.status == ChannelProbeStatus::Failed)
            .map(|entry| entry.platform.clone())
            .collect::<Vec<_>>();
        let degraded = !failing_platforms.is_empty();
        let current_consecutive_failures = if degraded {
            previous
                .map(|status| status.current_consecutive_failures + 1)
                .unwrap_or(1)
        } else {
            0
        };
        let restart_requested = config.channels.runtime.health_monitor_enabled
            && config.channels.runtime.auto_restart_on_failure
            && auto_restart_ready
            && current_consecutive_failures >= config.channels.runtime.failure_threshold;
        let restart_reason = restart_requested.then(|| {
            format!(
                "Channel health monitor requested a managed restart after {} consecutive failing scans: {}",
                current_consecutive_failures,
                failing_platforms.join(", ")
            )
        });

        ChannelHealthMonitorStatus {
            generated_at: generated_at.clone(),
            health_monitor_enabled: config.channels.runtime.health_monitor_enabled,
            auto_restart_on_failure: config.channels.runtime.auto_restart_on_failure,
            auto_restart_ready,
            consecutive_failure_threshold: config.channels.runtime.failure_threshold,
            current_consecutive_failures,
            degraded,
            failing_platforms,
            last_failure_at: if degraded {
                Some(generated_at.clone())
            } else {
                previous.and_then(|status| status.last_failure_at.clone())
            },
            last_healthy_at: if degraded {
                previous.and_then(|status| status.last_healthy_at.clone())
            } else {
                Some(generated_at)
            },
            restart_requested,
            restart_reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabled_channels_only_returns_enabled_entries() {
        let mut config = AppConfig::default();
        config.channels.telegram.enabled = true;
        config.channels.discord.enabled = false;
        config.channels.matrix.enabled = true;
        let channels = ChannelHealthMonitorService::new().enabled_channels(&config);
        assert!(channels.contains(&"telegram".to_string()));
        assert!(channels.contains(&"matrix".to_string()));
        assert!(!channels.contains(&"discord".to_string()));
    }

    #[test]
    fn channel_health_monitor_requests_restart_after_threshold() {
        let mut config = AppConfig::default();
        config.channels.runtime.auto_restart_on_failure = true;
        config.channels.runtime.failure_threshold = 2;

        let previous = ChannelHealthMonitorStatus {
            generated_at: "2026-03-27T00:00:00Z".to_string(),
            health_monitor_enabled: true,
            auto_restart_on_failure: true,
            auto_restart_ready: true,
            consecutive_failure_threshold: 2,
            current_consecutive_failures: 1,
            degraded: true,
            failing_platforms: vec!["slack".to_string()],
            last_failure_at: Some("2026-03-27T00:00:00Z".to_string()),
            last_healthy_at: None,
            restart_requested: false,
            restart_reason: None,
        };
        let entries = vec![ChannelProbeEntry {
            platform: "slack".to_string(),
            enabled: true,
            status: ChannelProbeStatus::Failed,
            probe_kind: "remote_auth".to_string(),
            detail: "token rejected".to_string(),
        }];

        let monitor = ChannelHealthMonitorService::new().compose_status_at(
            &config,
            true,
            Some(&previous),
            &entries,
            "2026-03-28T00:00:00Z".to_string(),
        );
        assert!(monitor.restart_requested);
        assert_eq!(monitor.current_consecutive_failures, 2);
    }
}

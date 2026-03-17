//! Push notifications for mobile nodes

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Notification priority
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationPriority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

/// Notification type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    #[default]
    Message,
    Alert,
    Update,
    System,
    Custom,
}

/// Push notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub priority: NotificationPriority,
    #[serde(default)]
    pub notification_type: NotificationType,
    #[serde(default)]
    pub data: HashMap<String, String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Notification {
    /// Create a new notification
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.into(),
            body: body.into(),
            priority: NotificationPriority::default(),
            notification_type: NotificationType::default(),
            data: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Set the priority
    pub fn with_priority(mut self, priority: NotificationPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the notification type
    pub fn with_type(mut self, notification_type: NotificationType) -> Self {
        self.notification_type = notification_type;
        self
    }

    /// Add custom data
    pub fn with_data(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.data.insert(key.into(), value.into());
        self
    }

    /// Check if notification is high priority
    pub fn is_high_priority(&self) -> bool {
        matches!(
            self.priority,
            NotificationPriority::High | NotificationPriority::Critical
        )
    }
}

/// Notification configuration
#[derive(Debug, Clone)]
pub struct NotificationConfig {
    pub enabled: bool,
    pub apns_enabled: bool, // Apple Push Notification Service
    pub fcm_enabled: bool,  // Firebase Cloud Messaging
    pub show_badge: bool,
    pub play_sound: bool,
    pub sound_name: Option<String>,
    pub vibration: bool,
    pub batch_interval_secs: u64,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            apns_enabled: cfg!(target_os = "ios"),
            fcm_enabled: cfg!(target_os = "android"),
            show_badge: true,
            play_sound: true,
            sound_name: None,
            vibration: true,
            batch_interval_secs: 5,
        }
    }
}

/// Notification manager
pub struct NotificationManager {
    config: NotificationConfig,
    pending: Vec<Notification>,
    token: Option<String>, // Device push token
}

impl NotificationManager {
    /// Create a new notification manager
    pub fn new(config: NotificationConfig) -> Self {
        Self {
            config,
            pending: Vec::new(),
            token: None,
        }
    }

    /// Set the device push token
    pub fn set_token(&mut self, token: impl Into<String>) {
        self.token = Some(token.into());
    }

    /// Get the device push token
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// Check if notifications are enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Show a notification immediately
    pub fn show(&mut self, notification: Notification) {
        if !self.config.enabled {
            return;
        }

        // Platform-specific implementation would go here
        // For iOS: Use UNUserNotificationCenter
        // For Android: Use NotificationManager

        #[cfg(target_os = "ios")]
        {
            if self.config.apns_enabled {
                // iOS notification display
            }
        }

        #[cfg(target_os = "android")]
        {
            if self.config.fcm_enabled {
                // Android notification display
            }
        }

        // For now, just add to pending list
        self.pending.push(notification);
    }

    /// Queue a notification for batching
    pub fn queue(&mut self, notification: Notification) {
        if !self.config.enabled {
            return;
        }
        self.pending.push(notification);
    }

    /// Get pending notifications count
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Clear pending notifications
    pub fn clear_pending(&mut self) {
        self.pending.clear();
    }

    /// Process all pending notifications
    pub fn flush(&mut self) {
        if self.pending.is_empty() {
            return;
        }

        // Group notifications by priority
        let high_priority: Vec<_> = self
            .pending
            .iter()
            .filter(|n| n.is_high_priority())
            .cloned()
            .collect();

        // Show high priority immediately
        for notification in high_priority {
            self.show_immediate(&notification);
        }

        // Batch remaining notifications
        let regular: Vec<_> = self
            .pending
            .iter()
            .filter(|n| !n.is_high_priority())
            .cloned()
            .collect();

        if regular.len() == 1 {
            self.show_immediate(&regular[0]);
        } else if regular.len() > 1 {
            // Create a summary notification
            let summary = Notification::new(
                format!("{} new notifications", regular.len()),
                "Tap to view",
            )
            .with_type(NotificationType::System);
            self.show_immediate(&summary);
        }

        self.pending.clear();
    }

    fn show_immediate(&self, notification: &Notification) {
        // Platform-specific immediate display
        #[cfg(target_os = "ios")]
        {
            // iOS: Post to UNUserNotificationCenter
            let _ = notification; // Use the notification
        }

        #[cfg(target_os = "android")]
        {
            // Android: Post via NotificationManager
            let _ = notification; // Use the notification
        }

        tracing::info!(
            "Showing notification: {} - {}",
            notification.title,
            notification.body
        );
    }

    /// Handle a received push notification from APNS/FCM
    pub fn handle_push(&mut self, payload: &str) -> Result<Notification, NotificationError> {
        let notification: Notification =
            serde_json::from_str(payload).map_err(|e| NotificationError::Parse(e.to_string()))?;

        // Validate notification
        if notification.id.is_empty() {
            return Err(NotificationError::Invalid(
                "Missing notification ID".to_string(),
            ));
        }

        Ok(notification)
    }
}

/// Notification errors
#[derive(Debug, thiserror::Error)]
pub enum NotificationError {
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Invalid notification: {0}")]
    Invalid(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Platform error: {0}")]
    Platform(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_creation() {
        let notification =
            Notification::new("Test Title", "Test Body").with_priority(NotificationPriority::High);

        assert_eq!(notification.title, "Test Title");
        assert_eq!(notification.body, "Test Body");
        assert_eq!(notification.priority, NotificationPriority::High);
        assert!(notification.is_high_priority());
        assert!(!notification.id.is_empty());
    }

    #[test]
    fn test_notification_data() {
        let notification = Notification::new("Title", "Body")
            .with_data("key1", "value1")
            .with_data("key2", "value2");

        assert_eq!(notification.data.get("key1"), Some(&"value1".to_string()));
        assert_eq!(notification.data.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_notification_priority() {
        let low = Notification::new("Low", "Body").with_priority(NotificationPriority::Low);
        let critical =
            Notification::new("Critical", "Body").with_priority(NotificationPriority::Critical);

        assert!(!low.is_high_priority());
        assert!(critical.is_high_priority());
    }

    #[test]
    fn test_notification_config() {
        let config = NotificationConfig::default();
        assert!(config.enabled);
        assert!(config.show_badge);
        assert!(config.play_sound);
        assert!(config.vibration);
    }

    #[test]
    fn test_notification_manager() {
        let config = NotificationConfig::default();
        let mut manager = NotificationManager::new(config);

        assert!(manager.is_enabled());
        assert_eq!(manager.pending_count(), 0);

        // Set token
        manager.set_token("test-token-123");
        assert_eq!(manager.token(), Some("test-token-123"));

        // Queue notification
        let notification = Notification::new("Test", "Body");
        manager.queue(notification);
        assert_eq!(manager.pending_count(), 1);

        // Clear pending
        manager.clear_pending();
        assert_eq!(manager.pending_count(), 0);
    }

    #[test]
    fn test_handle_push() {
        let config = NotificationConfig::default();
        let mut manager = NotificationManager::new(config);

        let payload = r#"{
            "id": "notif-123",
            "title": "Test",
            "body": "Test body",
            "priority": "high",
            "timestamp": "2024-01-01T00:00:00Z"
        }"#;

        let result = manager.handle_push(payload);
        assert!(result.is_ok());

        let notification = result.unwrap();
        assert_eq!(notification.id, "notif-123");
        assert_eq!(notification.priority, NotificationPriority::High);
    }

    #[test]
    fn test_handle_push_invalid() {
        let config = NotificationConfig::default();
        let mut manager = NotificationManager::new(config);

        // Invalid JSON
        let result = manager.handle_push("not valid json");
        assert!(result.is_err());
    }
}

//! Security audit logging.

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use tracing::info;

/// Severity level for audit events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverity {
    Info,
    Warn,
    Error,
    Critical,
}

/// An audit event to be logged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_type: String,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub details: Option<String>,
    pub severity: AuditSeverity,
    pub timestamp: DateTime<Utc>,
}

impl AuditEvent {
    pub fn new(event_type: &str, severity: AuditSeverity) -> Self {
        Self {
            event_type: event_type.to_string(),
            session_id: None,
            user_id: None,
            details: None,
            severity,
            timestamp: Utc::now(),
        }
    }

    pub fn with_session(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }

    pub fn with_user(mut self, user_id: &str) -> Self {
        self.user_id = Some(user_id.to_string());
        self
    }

    pub fn with_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }

    /// Log this event via tracing (DB persistence handled by db crate).
    pub fn log(&self) {
        match self.severity {
            AuditSeverity::Info => {
                info!(event = %self.event_type, "Audit: {}", self.event_type)
            }
            AuditSeverity::Warn => {
                tracing::warn!(event = %self.event_type, "Audit: {}", self.event_type)
            }
            AuditSeverity::Error => {
                tracing::error!(event = %self.event_type, "Audit: {}", self.event_type)
            }
            AuditSeverity::Critical => {
                tracing::error!(event = %self.event_type, "CRITICAL Audit: {}", self.event_type)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_audit_event() {
        let event = AuditEvent::new("login_attempt", AuditSeverity::Info);
        assert_eq!(event.event_type, "login_attempt");
        assert!(event.session_id.is_none());
        assert!(event.user_id.is_none());
        assert!(event.details.is_none());
    }

    #[test]
    fn builder_pattern() {
        let event = AuditEvent::new("origin_rejected", AuditSeverity::Warn)
            .with_session("sess_123")
            .with_user("user_42")
            .with_details("Origin https://evil.com was rejected");

        assert_eq!(event.event_type, "origin_rejected");
        assert_eq!(event.session_id.as_deref(), Some("sess_123"));
        assert_eq!(event.user_id.as_deref(), Some("user_42"));
        assert!(event.details.as_ref().unwrap().contains("evil.com"));
    }

    #[test]
    fn serialization_roundtrip() {
        let event = AuditEvent::new("test_event", AuditSeverity::Critical)
            .with_session("s1")
            .with_user("u1");

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: AuditEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.event_type, "test_event");
        assert_eq!(deserialized.session_id.as_deref(), Some("s1"));
    }
}

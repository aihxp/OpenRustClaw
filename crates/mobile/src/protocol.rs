//! Typed mobile node command protocol shared across CLI, control surfaces, and runtimes.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceCommandKind {
    SendMessage,
    PushNotification,
    SyncNow,
}

impl DeviceCommandKind {
    pub fn required_capability(&self) -> &'static str {
        match self {
            DeviceCommandKind::SendMessage => "mobile",
            DeviceCommandKind::PushNotification => "notifications",
            DeviceCommandKind::SyncNow => "mobile",
        }
    }

    pub fn default_requires_approval(&self) -> bool {
        !matches!(self, DeviceCommandKind::SyncNow)
    }
}

impl fmt::Display for DeviceCommandKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            DeviceCommandKind::SendMessage => "send_message",
            DeviceCommandKind::PushNotification => "push_notification",
            DeviceCommandKind::SyncNow => "sync_now",
        };
        f.write_str(value)
    }
}

impl FromStr for DeviceCommandKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "send_message" => Ok(DeviceCommandKind::SendMessage),
            "push_notification" => Ok(DeviceCommandKind::PushNotification),
            "sync_now" => Ok(DeviceCommandKind::SyncNow),
            other => Err(format!("unsupported mobile command '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileCommandDispatchRequest {
    pub node_id: String,
    pub command: DeviceCommandKind,
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub approved_by: Option<String>,
    #[serde(default)]
    pub require_approval: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileCommandDecisionRequest {
    pub decided_by: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileCommandRecord {
    pub id: String,
    pub node_id: String,
    pub command: DeviceCommandKind,
    pub required_capability: String,
    pub approval_required: bool,
    pub status: String,
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub result: Value,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub approved_by: Option<String>,
    #[serde(default)]
    pub decided_reason: Option<String>,
    #[serde(default)]
    pub approved_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub executed_at: Option<DateTime<Utc>>,
}

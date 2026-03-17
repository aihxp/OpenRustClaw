//! WebSocket protocol for Canvas communication
//!
//! Defines the message types for bidirectional communication between
//! the server and connected clients.

use crate::canvas::{CanvasAction, CanvasUpdate};
use crate::elements::CanvasElement;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Messages exchanged between client and server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CanvasMessage {
    // Client -> Server
    /// Connect to a canvas (creates new if canvas_id is None)
    Connect { canvas_id: Option<Uuid> },
    /// User interaction with an element
    Interact {
        element_id: Uuid,
        interaction: Interaction,
    },
    /// Subscribe to canvas updates
    Subscribe { canvas_id: Uuid },
    /// Unsubscribe from canvas updates
    Unsubscribe { canvas_id: Uuid },
    /// Set canvas state
    SetState {
        key: String,
        value: serde_json::Value,
    },
    /// Execute command on canvas
    Command { command: CanvasCommand },
    /// Ping to keep connection alive
    Ping,

    // Server -> Client
    /// Canvas update notification
    Update { update: CanvasUpdate },
    /// Full canvas snapshot
    Snapshot { canvas: CanvasSnapshot },
    /// Interaction acknowledgment
    Ack { message_id: Uuid },
    /// Error response
    Error {
        message: String,
        code: Option<String>,
    },
    /// Pong response
    Pong,
}

/// User interaction types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Interaction {
    /// Simple click
    Click,
    /// Form submission with data
    Submit { data: serde_json::Value },
    /// Text input
    Input { value: String },
    /// Option selection
    Select { option: String },
    /// Checkbox toggle
    Toggle { checked: bool },
    /// Custom interaction
    Custom { payload: serde_json::Value },
}

/// Commands that can be sent to the canvas
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum CanvasCommand {
    /// Push new elements
    Push { elements: Vec<CanvasElement> },
    /// Reset/clear canvas
    Reset,
    /// Update specific element
    Update {
        element_id: Uuid,
        element: CanvasElement,
    },
    /// Remove specific element
    Remove { element_id: Uuid },
    /// Execute JavaScript
    Eval { script: String },
    /// Get canvas snapshot
    GetSnapshot,
    /// Set canvas title
    SetTitle { title: String },
}

/// Snapshot of canvas state for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasSnapshot {
    pub id: Uuid,
    pub title: String,
    pub elements: Vec<CanvasElement>,
    pub state: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl CanvasSnapshot {
    /// Create a new snapshot
    pub fn new(id: Uuid, title: impl Into<String>, elements: Vec<CanvasElement>) -> Self {
        Self {
            id,
            title: title.into(),
            elements,
            state: serde_json::Value::Object(serde_json::Map::new()),
            created_at: chrono::Utc::now(),
        }
    }

    /// Create a snapshot with state
    pub fn with_state(
        id: Uuid,
        title: impl Into<String>,
        elements: Vec<CanvasElement>,
        state: impl Serialize,
    ) -> Result<Self, serde_json::Error> {
        Ok(Self {
            id,
            title: title.into(),
            elements,
            state: serde_json::to_value(state)?,
            created_at: chrono::Utc::now(),
        })
    }
}

/// Helper for serializing CanvasUpdate
impl Serialize for CanvasUpdate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("CanvasUpdate", 2)?;
        state.serialize_field("canvas_id", &self.canvas_id)?;
        state.serialize_field("action", &self.action)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for CanvasUpdate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct CanvasUpdateHelper {
            canvas_id: Uuid,
            action: CanvasAction,
        }

        let helper = CanvasUpdateHelper::deserialize(deserializer)?;
        Ok(CanvasUpdate {
            canvas_id: helper.canvas_id,
            action: helper.action,
        })
    }
}

/// Helper for serializing CanvasAction
impl Serialize for CanvasAction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        match self {
            CanvasAction::Push { elements } => {
                let mut state = serializer.serialize_struct("CanvasActionPush", 2)?;
                state.serialize_field("type", "push")?;
                state.serialize_field("elements", elements)?;
                state.end()
            }
            CanvasAction::Reset => {
                let mut state = serializer.serialize_struct("CanvasActionReset", 1)?;
                state.serialize_field("type", "reset")?;
                state.end()
            }
            CanvasAction::Update {
                element_id,
                element,
            } => {
                let mut state = serializer.serialize_struct("CanvasActionUpdate", 3)?;
                state.serialize_field("type", "update")?;
                state.serialize_field("element_id", element_id)?;
                state.serialize_field("element", element)?;
                state.end()
            }
            CanvasAction::Remove { element_id } => {
                let mut state = serializer.serialize_struct("CanvasActionRemove", 2)?;
                state.serialize_field("type", "remove")?;
                state.serialize_field("element_id", element_id)?;
                state.end()
            }
            CanvasAction::Eval { script } => {
                let mut state = serializer.serialize_struct("CanvasActionEval", 2)?;
                state.serialize_field("type", "eval")?;
                state.serialize_field("script", script)?;
                state.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for CanvasAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct CanvasActionHelper {
            #[serde(rename = "type")]
            action_type: String,
            elements: Option<Vec<CanvasElement>>,
            element_id: Option<Uuid>,
            element: Option<CanvasElement>,
            script: Option<String>,
        }

        let helper = CanvasActionHelper::deserialize(deserializer)?;

        match helper.action_type.as_str() {
            "push" => {
                let elements = helper
                    .elements
                    .ok_or_else(|| serde::de::Error::custom("Missing elements for push action"))?;
                Ok(CanvasAction::Push { elements })
            }
            "reset" => Ok(CanvasAction::Reset),
            "update" => {
                let element_id = helper.element_id.ok_or_else(|| {
                    serde::de::Error::custom("Missing element_id for update action")
                })?;
                let element = helper
                    .element
                    .ok_or_else(|| serde::de::Error::custom("Missing element for update action"))?;
                Ok(CanvasAction::Update {
                    element_id,
                    element,
                })
            }
            "remove" => {
                let element_id = helper.element_id.ok_or_else(|| {
                    serde::de::Error::custom("Missing element_id for remove action")
                })?;
                Ok(CanvasAction::Remove { element_id })
            }
            "eval" => {
                let script = helper
                    .script
                    .ok_or_else(|| serde::de::Error::custom("Missing script for eval action"))?;
                Ok(CanvasAction::Eval { script })
            }
            _ => Err(serde::de::Error::custom(format!(
                "Unknown action type: {}",
                helper.action_type
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::TextStyle;

    #[test]
    fn test_serialize_canvas_message() {
        let msg = CanvasMessage::Connect {
            canvas_id: Some(Uuid::new_v4()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("connect"));
    }

    #[test]
    fn test_serialize_interaction() {
        let interaction = Interaction::Submit {
            data: serde_json::json!({"key": "value"}),
        };

        let json = serde_json::to_string(&interaction).unwrap();
        assert!(json.contains("submit"));
        assert!(json.contains("key"));
    }

    #[test]
    fn test_serialize_canvas_command() {
        let cmd = CanvasCommand::Reset;

        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("reset"));
    }

    #[test]
    fn test_canvas_action_serialization() {
        let action = CanvasAction::Push {
            elements: vec![CanvasElement::Text {
                id: Uuid::new_v4(),
                content: "Test".to_string(),
                style: TextStyle::new(),
            }],
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("push"));
        assert!(json.contains("Test"));

        let deserialized: CanvasAction = serde_json::from_str(&json).unwrap();
        match deserialized {
            CanvasAction::Push { elements } => assert_eq!(elements.len(), 1),
            _ => panic!("Wrong action type"),
        }
    }

    #[test]
    fn test_canvas_snapshot_creation() {
        let id = Uuid::new_v4();
        let snapshot = CanvasSnapshot::new(id, "Test Canvas", vec![]);

        assert_eq!(snapshot.id, id);
        assert_eq!(snapshot.title, "Test Canvas");
    }
}

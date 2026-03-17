//! Streaming response handling for the Anthropic API.

use serde::{Deserialize, Serialize};

use crate::types::{ContentBlockDelta, Message, MessageRole, StopReason, Usage};

/// A stream event from the Anthropic API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// The beginning of the streaming response.
    MessageStart {
        /// The message information.
        message: StreamMessage,
    },

    /// A content block is starting.
    ContentBlockStart {
        /// The index of this content block.
        index: usize,
        /// The content block.
        content_block: ContentBlockStart,
    },

    /// A delta in a content block.
    ContentBlockDelta {
        /// The index of the content block.
        index: usize,
        /// The delta.
        delta: ContentBlockDelta,
    },

    /// A content block has finished.
    ContentBlockStop {
        /// The index of the content block.
        index: usize,
    },

    /// The message is being updated.
    MessageDelta {
        /// The delta information.
        delta: MessageDelta,
        /// Usage statistics (if available).
        usage: Option<Usage>,
    },

    /// The message has finished.
    MessageStop,

    /// A ping event (can be ignored).
    Ping,
}

/// A message in a stream start event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMessage {
    /// Unique identifier.
    pub id: String,
    /// Message type.
    #[serde(rename = "type")]
    pub message_type: String,
    /// The role.
    pub role: MessageRole,
    /// The model used.
    pub model: String,
    /// Stop reason (will be null during streaming).
    pub stop_reason: Option<StopReason>,
    /// Stop sequence (will be null during streaming).
    pub stop_sequence: Option<String>,
    /// Usage (will be null during streaming).
    pub usage: Option<Usage>,
}

/// The start of a content block in streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockStart {
    /// A text block is starting.
    Text {
        /// The text (usually empty at start).
        text: String,
    },
    /// A tool use block is starting.
    ToolUse {
        /// The tool use ID.
        id: String,
        /// The tool name.
        name: String,
        /// The input (usually empty at start).
        input: serde_json::Value,
    },
}

/// A delta in a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDelta {
    /// The stop reason, if the message is ending.
    pub stop_reason: Option<StopReason>,
    /// The stop sequence, if applicable.
    pub stop_sequence: Option<String>,
}

/// Result type for streaming operations.
pub type StreamResult<T> = std::result::Result<T, crate::error::AnthropicError>;

/// A collector that accumulates streaming events into a complete message.
#[derive(Debug, Default)]
pub struct StreamCollector {
    message_id: Option<String>,
    model: Option<String>,
    content_blocks: Vec<PartialBlock>,
    usage: Option<Usage>,
    stop_reason: Option<StopReason>,
}

#[derive(Debug)]
enum PartialBlock {
    Text(String),
    ToolUse {
        id: String,
        name: String,
        input: String, // Accumulated as JSON string
    },
}

impl StreamCollector {
    /// Create a new stream collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a stream event.
    pub fn process_event(&mut self, event: &StreamEvent) {
        match event {
            StreamEvent::MessageStart { message } => {
                self.message_id = Some(message.id.clone());
                self.model = Some(message.model.clone());
            }
            StreamEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                // Ensure we have enough slots
                while self.content_blocks.len() <= *index {
                    self.content_blocks.push(PartialBlock::Text(String::new()));
                }

                match content_block {
                    ContentBlockStart::Text { text } => {
                        self.content_blocks[*index] = PartialBlock::Text(text.clone());
                    }
                    ContentBlockStart::ToolUse { id, name, .. } => {
                        self.content_blocks[*index] = PartialBlock::ToolUse {
                            id: id.clone(),
                            name: name.clone(),
                            input: String::new(),
                        };
                    }
                }
            }
            StreamEvent::ContentBlockDelta { index, delta } => {
                if let Some(block) = self.content_blocks.get_mut(*index) {
                    match block {
                        PartialBlock::Text(text) => {
                            if let ContentBlockDelta::TextDelta(t) = delta {
                                text.push_str(&t.text);
                            }
                        }
                        PartialBlock::ToolUse { input, .. } => {
                            if let ContentBlockDelta::PartialJson { partial_json } = delta {
                                input.push_str(partial_json);
                            }
                        }
                    }
                }
            }
            StreamEvent::MessageDelta { delta, usage } => {
                self.stop_reason = delta.stop_reason;
                if let Some(u) = usage {
                    self.usage = Some(*u);
                }
            }
            _ => {}
        }
    }

    /// Build the final message from collected events.
    pub fn build_message(&self) -> Option<Message> {
        let content = self
            .content_blocks
            .iter()
            .filter_map(|block| match block {
                PartialBlock::Text(text) => {
                    if text.is_empty() {
                        None
                    } else {
                        Some(crate::types::ContentBlock::text(text.clone()))
                    }
                }
                PartialBlock::ToolUse { id, name, input } => {
                    let parsed_input = serde_json::from_str(input).unwrap_or_default();
                    Some(crate::types::ContentBlock::tool_use(
                        id.clone(),
                        name.clone(),
                        parsed_input,
                    ))
                }
            })
            .collect();

        Some(Message::with_content(MessageRole::Assistant, content))
    }

    /// Get the collected text so far.
    pub fn current_text(&self) -> String {
        self.content_blocks
            .iter()
            .filter_map(|block| match block {
                PartialBlock::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Get the message ID if available.
    pub fn message_id(&self) -> Option<&str> {
        self.message_id.as_deref()
    }

    /// Get the model if available.
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// Get the usage if available.
    pub fn usage(&self) -> Option<Usage> {
        self.usage
    }

    /// Check if the stream has finished.
    pub fn is_complete(&self) -> bool {
        self.stop_reason.is_some()
    }
}

/// Helper function to collect all events from a stream.
pub async fn collect_stream<S>(mut stream: S) -> (Vec<StreamEvent>, StreamCollector)
where
    S: futures::Stream<Item = StreamResult<StreamEvent>> + Unpin,
{
    use futures::StreamExt;

    let mut events = Vec::new();
    let mut collector = StreamCollector::new();

    while let Some(result) = stream.next().await {
        if let Ok(event) = result {
            collector.process_event(&event);
            events.push(event);
        }
    }

    (events, collector)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_collector() {
        let mut collector = StreamCollector::new();

        // Simulate streaming events
        let start = StreamEvent::MessageStart {
            message: StreamMessage {
                id: "msg_123".to_string(),
                message_type: "message".to_string(),
                role: MessageRole::Assistant,
                model: "claude-3".to_string(),
                stop_reason: None,
                stop_sequence: None,
                usage: None,
            },
        };
        collector.process_event(&start);

        let block_start = StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlockStart::Text {
                text: String::new(),
            },
        };
        collector.process_event(&block_start);

        let delta1 = StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentBlockDelta::TextDelta(crate::types::TextDelta {
                text: "Hello".to_string(),
            }),
        };
        collector.process_event(&delta1);

        let delta2 = StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentBlockDelta::TextDelta(crate::types::TextDelta {
                text: " world!".to_string(),
            }),
        };
        collector.process_event(&delta2);

        assert_eq!(collector.current_text(), "Hello world!");
    }

    #[test]
    fn test_message_delta() {
        let delta = MessageDelta {
            stop_reason: Some(StopReason::EndTurn),
            stop_sequence: None,
        };
        assert_eq!(delta.stop_reason, Some(StopReason::EndTurn));
    }
}

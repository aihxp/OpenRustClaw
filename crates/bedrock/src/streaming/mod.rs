//! Streaming support for AWS Bedrock.
//!
//! This module provides types and utilities for handling streaming responses
//! from the InvokeModelWithResponseStream and ConverseStream APIs.

use serde::{Deserialize, Serialize};

use crate::types::{ContentBlockDelta, ConversationRole, StopReason, TokenUsage};

/// A stream event from Bedrock.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// The start of a message.
    MessageStart {
        /// The message start details.
        message: StreamMessage,
    },
    /// The start of a content block.
    ContentBlockStart {
        /// The content block index.
        content_block_index: i32,
        /// The start details.
        #[serde(skip_serializing_if = "Option::is_none")]
        start: Option<ContentBlockStart>,
    },
    /// A delta in a content block.
    ContentBlockDelta {
        /// The content block index.
        content_block_index: i32,
        /// The delta.
        delta: ContentBlockDelta,
    },
    /// The end of a content block.
    ContentBlockStop {
        /// The content block index.
        content_block_index: i32,
    },
    /// The end of a message.
    MessageStop {
        /// The stop reason.
        stop_reason: StopReason,
    },
    /// Metadata about the request.
    Metadata {
        /// Token usage.
        usage: TokenUsage,
        /// Metrics.
        metrics: StreamMetrics,
    },
    /// Internal server error (for streaming errors).
    InternalServerError {
        /// The error message.
        message: String,
    },
    /// Model stream error.
    ModelStreamError {
        /// The error message.
        message: String,
    },
    /// Validation exception.
    ValidationException {
        /// The error message.
        message: String,
    },
    /// Throttling exception.
    ThrottlingException {
        /// The error message.
        message: String,
    },
}

/// A message in a stream start event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamMessage {
    /// The role.
    pub role: ConversationRole,
}

/// The start of a content block in streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentBlockStart {
    /// Tool use start.
    #[serde(rename = "toolUse")]
    pub tool_use: Option<ToolUseStart>,
}

/// Tool use start details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolUseStart {
    /// The tool use ID.
    pub tool_use_id: String,
    /// The tool name.
    pub name: String,
}

/// Metrics for streaming.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamMetrics {
    /// The latency in milliseconds.
    pub latency_ms: i64,
}

/// Result type for streaming operations.
pub type StreamResult<T> = std::result::Result<T, crate::error::BedrockError>;

/// A collector that accumulates streaming events into a complete message.
#[derive(Debug, Default)]
pub struct StreamCollector {
    role: Option<ConversationRole>,
    content_blocks: Vec<PartialBlock>,
    usage: Option<TokenUsage>,
    stop_reason: Option<StopReason>,
}

#[derive(Debug)]
#[allow(dead_code)]
enum PartialBlock {
    Text(String),
    ToolUse {
        id: String,
        name: String,
        input: String,
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
                self.role = Some(message.role);
            }
            StreamEvent::ContentBlockStart {
                content_block_index,
                start,
            } => {
                while self.content_blocks.len() <= *content_block_index as usize {
                    self.content_blocks.push(PartialBlock::Text(String::new()));
                }

                if let Some(start) = start {
                    if let Some(tool_use) = &start.tool_use {
                        self.content_blocks[*content_block_index as usize] =
                            PartialBlock::ToolUse {
                                id: tool_use.tool_use_id.clone(),
                                name: tool_use.name.clone(),
                                input: String::new(),
                            };
                    }
                }
            }
            StreamEvent::ContentBlockDelta {
                content_block_index,
                delta,
            } => {
                if let Some(block) = self.content_blocks.get_mut(*content_block_index as usize) {
                    match block {
                        PartialBlock::Text(text) => {
                            if let Some(delta_text) = delta.text() {
                                text.push_str(delta_text);
                            }
                        }
                        PartialBlock::ToolUse { input, .. } => {
                            if let ContentBlockDelta::ToolUse { tool_use } = delta {
                                input.push_str(&tool_use.input);
                            }
                        }
                    }
                }
            }
            StreamEvent::MessageStop { stop_reason } => {
                self.stop_reason = Some(*stop_reason);
            }
            StreamEvent::Metadata { usage, .. } => {
                self.usage = Some(*usage);
            }
            _ => {}
        }
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

    /// Check if the stream has finished.
    pub fn is_complete(&self) -> bool {
        self.stop_reason.is_some()
    }

    /// Get the stop reason if available.
    pub fn stop_reason(&self) -> Option<StopReason> {
        self.stop_reason
    }

    /// Get token usage if available.
    pub fn usage(&self) -> Option<TokenUsage> {
        self.usage
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

/// Stream chunk for InvokeModelWithResponseStream.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseStreamChunk {
    /// The bytes of the chunk.
    pub bytes: String,
}

/// Payload part for response streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadPart {
    /// The chunk of the response.
    pub chunk: ResponseStreamChunk,
}

/// Internal server exception for streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalServerException {
    /// The error message.
    pub message: String,
}

/// Model stream error exception.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStreamErrorException {
    /// The original message ID.
    pub original_message: String,
    /// The error message.
    pub message: String,
}

/// Validation exception for streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationException {
    /// The error message.
    pub message: String,
}

/// Throttling exception for streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThrottlingException {
    /// The error message.
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ConversationRole;

    #[test]
    fn test_stream_collector() {
        let mut collector = StreamCollector::new();

        // Start message
        collector.process_event(&StreamEvent::MessageStart {
            message: StreamMessage {
                role: ConversationRole::Assistant,
            },
        });

        // ContentBlockStart must precede ContentBlockDelta to initialize the block
        collector.process_event(&StreamEvent::ContentBlockStart {
            content_block_index: 0,
            start: Some(ContentBlockStart { tool_use: None }),
        });

        // Add text
        collector.process_event(&StreamEvent::ContentBlockDelta {
            content_block_index: 0,
            delta: ContentBlockDelta::Text {
                text: "Hello".to_string(),
            },
        });

        collector.process_event(&StreamEvent::ContentBlockDelta {
            content_block_index: 0,
            delta: ContentBlockDelta::Text {
                text: " world!".to_string(),
            },
        });

        assert_eq!(collector.current_text(), "Hello world!");

        // Stop
        collector.process_event(&StreamEvent::MessageStop {
            stop_reason: StopReason::EndTurn,
        });

        assert!(collector.is_complete());
        assert_eq!(collector.stop_reason(), Some(StopReason::EndTurn));
    }

    #[test]
    fn test_stream_metrics() {
        let metrics = StreamMetrics { latency_ms: 1000 };
        assert_eq!(metrics.latency_ms, 1000);
    }
}

//! Converse API response types.

use serde::{Deserialize, Serialize};

use crate::types::{
    AdditionalModelResultFields, ContentBlock, ConversationRole, GuardrailAssessment, StopReason,
    TokenUsage,
};

/// Response from the Converse API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConverseResponse {
    /// The output from the model.
    pub output: Output,
    /// Token usage statistics.
    pub usage: TokenUsage,
    /// The stop reason.
    pub stop_reason: StopReason,
    /// Additional model-specific response fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_model_result_fields: Option<AdditionalModelResultFields>,
    /// Metrics for the request.
    pub metrics: Metrics,
    /// The trace from guardrails.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<ConverseTrace>,
}

impl ConverseResponse {
    /// Get the message content from the output.
    pub fn message(&self) -> Option<&Message> {
        match &self.output {
            Output::Message(msg) => Some(msg),
        }
    }

    /// Get the text content from the output.
    pub fn text(&self) -> String {
        self.output.text()
    }

    /// Check if the response has tool uses.
    pub fn has_tool_use(&self) -> bool {
        self.output.has_tool_use()
    }

    /// Get the stop reason.
    pub fn stop_reason(&self) -> StopReason {
        self.stop_reason
    }

    /// Get token usage.
    pub fn usage(&self) -> &TokenUsage {
        &self.usage
    }

    /// Get the latency in milliseconds.
    pub fn latency_ms(&self) -> i64 {
        self.metrics.latency_ms
    }
}

/// Output from the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "message")]
pub enum Output {
    /// A message response.
    Message(Message),
}

impl Output {
    /// Get the text content.
    pub fn text(&self) -> String {
        match self {
            Output::Message(msg) => msg.text_content(),
        }
    }

    /// Check if the output has tool uses.
    pub fn has_tool_use(&self) -> bool {
        match self {
            Output::Message(msg) => msg.has_tool_use(),
        }
    }

    /// Get tool uses from the output.
    pub fn tool_uses(&self) -> Vec<&ContentBlock> {
        match self {
            Output::Message(msg) => msg
                .content
                .iter()
                .filter(|b| b.is_tool_use())
                .collect(),
        }
    }
}

/// A message in the response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// The role of the message author.
    pub role: ConversationRole,
    /// The content of the message.
    pub content: Vec<ContentBlock>,
}

impl Message {
    /// Get text content (concatenates all text blocks).
    pub fn text_content(&self) -> String {
        self.content
            .iter()
            .filter_map(|block| block.as_text())
            .collect::<Vec<_>>()
            .join("")
    }

    /// Check if this message has tool uses.
    pub fn has_tool_use(&self) -> bool {
        self.content.iter().any(|block| block.is_tool_use())
    }
}

/// Metrics for a Converse request.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    /// The latency in milliseconds.
    pub latency_ms: i64,
    /// Input tokens per minute (for streaming).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens_per_minute: Option<i64>,
    /// Output tokens per minute (for streaming).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens_per_minute: Option<i64>,
}

/// Trace from guardrails.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConverseTrace {
    /// Guardrail trace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guardrail: Option<GuardrailTrace>,
}

/// Guardrail trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailTrace {
    /// The model output before guardrails.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_output: Option<Vec<String>>,
    /// The input assessments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_assessments: Option<Vec<Vec<GuardrailAssessment>>>,
    /// The output assessments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_assessments: Option<Vec<Vec<GuardrailAssessment>>>,
}

/// Response from the ConverseStream API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConverseStreamResponse {
    /// The stream of events.
    pub stream: Vec<StreamEvent>,
}

/// Stream event from ConverseStream.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StreamEvent {
    /// The start of a message.
    MessageStart {
        /// The message start details.
        message: StreamMessageStart,
    },
    /// The start of a content block.
    ContentBlockStart {
        /// The content block index.
        content_block_index: i32,
        /// The start details.
        start: ContentBlockStart,
    },
    /// A delta in a content block.
    ContentBlockDelta {
        /// The content block index.
        content_block_index: i32,
        /// The delta.
        delta: crate::types::ContentBlockDelta,
    },
    /// The end of a content block.
    ContentBlockStop {
        /// The content block index.
        content_block_index: i32,
    },
    /// The end of a message.
    MessageStop {
        /// The stop details.
        stop_reason: StopReason,
    },
    /// Metadata about the request.
    Metadata {
        /// The metadata.
        metadata: StreamMetadata,
    },
}

/// Message start in a stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamMessageStart {
    /// The role.
    pub role: ConversationRole,
}

/// Content block start in a stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentBlockStart {
    /// Tool use start.
    #[serde(rename = "toolUse")]
    pub tool_use: Option<crate::types::ToolUseBlock>,
}

/// Stream metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamMetadata {
    /// Token usage.
    pub usage: TokenUsage,
    /// Metrics.
    pub metrics: Metrics,
    /// The trace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<ConverseTrace>,
}

/// Collector for accumulating stream events.
#[derive(Debug, Default)]
pub struct StreamCollector {
    role: Option<ConversationRole>,
    content_blocks: Vec<PartialContentBlock>,
    usage: Option<TokenUsage>,
    stop_reason: Option<StopReason>,
}

#[derive(Debug)]
enum PartialContentBlock {
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
            StreamEvent::ContentBlockStart { content_block_index, start } => {
                // Ensure we have enough slots
                while self.content_blocks.len() <= *content_block_index as usize {
                    self.content_blocks.push(PartialContentBlock::Text(String::new()));
                }

                if let Some(tool_use) = &start.tool_use {
                    self.content_blocks[*content_block_index as usize] = PartialContentBlock::ToolUse {
                        id: tool_use.tool_use_id.clone(),
                        name: tool_use.name.clone(),
                        input: String::new(),
                    };
                }
            }
            StreamEvent::ContentBlockDelta { content_block_index, delta } => {
                if let Some(block) = self.content_blocks.get_mut(*content_block_index as usize) {
                    match block {
                        PartialContentBlock::Text(text) => {
                            if let Some(delta_text) = delta.text() {
                                text.push_str(delta_text);
                            }
                        }
                        PartialContentBlock::ToolUse { input, .. } => {
                            if let crate::types::ContentBlockDelta::ToolUse { tool_use } = delta {
                                input.push_str(&tool_use.input);
                            }
                        }
                    }
                }
            }
            StreamEvent::MessageStop { stop_reason } => {
                self.stop_reason = Some(*stop_reason);
            }
            StreamEvent::Metadata { metadata } => {
                self.usage = Some(metadata.usage);
            }
            _ => {}
        }
    }

    /// Build the final message from collected events.
    pub fn build_message(&self) -> Option<Message> {
        let role = self.role?;

        let content: Vec<ContentBlock> = self
            .content_blocks
            .iter()
            .filter_map(|block| match block {
                PartialContentBlock::Text(text) => {
                    if text.is_empty() {
                        None
                    } else {
                        Some(ContentBlock::text(text.clone()))
                    }
                }
                PartialContentBlock::ToolUse { id, name, input } => {
                    let parsed_input = serde_json::from_str(input).unwrap_or_default();
                    Some(ContentBlock::tool_use(id.clone(), name.clone(), parsed_input))
                }
            })
            .collect();

        Some(Message { role, content })
    }

    /// Get the collected text so far.
    pub fn current_text(&self) -> String {
        self.content_blocks
            .iter()
            .filter_map(|block| match block {
                PartialContentBlock::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Get token usage if available.
    pub fn usage(&self) -> Option<TokenUsage> {
        self.usage
    }

    /// Get the stop reason if available.
    pub fn stop_reason(&self) -> Option<StopReason> {
        self.stop_reason
    }

    /// Check if the stream has finished.
    pub fn is_complete(&self) -> bool {
        self.stop_reason.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ContentBlock, ConversationRole};

    #[test]
    fn test_converse_response() {
        let response = ConverseResponse {
            output: Output::Message(Message {
                role: ConversationRole::Assistant,
                content: vec![ContentBlock::text("Hello!")],
            }),
            usage: TokenUsage::new(10, 20),
            stop_reason: StopReason::EndTurn,
            additional_model_result_fields: None,
            metrics: Metrics {
                latency_ms: 1000,
                input_tokens_per_minute: None,
                output_tokens_per_minute: None,
            },
            trace: None,
        };

        assert_eq!(response.text(), "Hello!");
        assert_eq!(response.usage().input_tokens, 10);
        assert_eq!(response.latency_ms(), 1000);
        assert!(!response.has_tool_use());
    }

    #[test]
    fn test_stream_collector() {
        let mut collector = StreamCollector::new();

        // Simulate streaming events
        collector.process_event(&StreamEvent::MessageStart {
            message: StreamMessageStart {
                role: ConversationRole::Assistant,
            },
        });

        collector.process_event(&StreamEvent::ContentBlockDelta {
            content_block_index: 0,
            delta: crate::types::ContentBlockDelta::Text {
                text: "Hello".to_string(),
            },
        });

        collector.process_event(&StreamEvent::ContentBlockDelta {
            content_block_index: 0,
            delta: crate::types::ContentBlockDelta::Text {
                text: " world!".to_string(),
            },
        });

        assert_eq!(collector.current_text(), "Hello world!");

        collector.process_event(&StreamEvent::MessageStop {
            stop_reason: StopReason::EndTurn,
        });

        assert!(collector.is_complete());
        assert!(collector.build_message().is_some());
    }

    #[test]
    fn test_output_text() {
        let output = Output::Message(Message {
            role: ConversationRole::Assistant,
            content: vec![
                ContentBlock::text("Hello "),
                ContentBlock::text("world!"),
            ],
        });

        assert_eq!(output.text(), "Hello world!");
    }
}

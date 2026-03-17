//! Streaming response handling for the AI21 API.

use serde::{Deserialize, Serialize};

use crate::types::{FinishReason, ToolCall, Usage};

/// A stream event from the AI21 API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamEvent {
    /// The streaming has started.
    #[serde(rename = "start")]
    Start {
        /// The ID of the chat completion.
        #[serde(rename = "chat_completion_id")]
        chat_completion_id: String,
    },

    /// A content delta (token) in the stream.
    #[serde(rename = "content")]
    ContentDelta {
        /// The content delta.
        delta: String,
    },

    /// Tool calls delta in the stream.
    #[serde(rename = "tool_calls")]
    ToolCallsDelta {
        /// The tool calls delta.
        #[serde(rename = "tool_calls")]
        tool_calls: Vec<ToolCall>,
    },

    /// The stream has ended.
    #[serde(rename = "end")]
    End {
        /// The finish reason.
        #[serde(rename = "finish_reason")]
        finish_reason: Option<FinishReason>,
    },

    /// Usage information at the end of the stream.
    #[serde(rename = "usage")]
    Usage {
        /// The usage information.
        usage: Usage,
    },
}

/// Result type for streaming operations.
pub type StreamResult<T> = std::result::Result<T, crate::error::Ai21Error>;

/// A collector that accumulates streaming events into a complete response.
#[derive(Debug, Default)]
pub struct StreamCollector {
    chat_completion_id: Option<String>,
    content: String,
    tool_calls: Vec<ToolCall>,
    finish_reason: Option<FinishReason>,
    usage: Option<Usage>,
}

impl StreamCollector {
    /// Create a new stream collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a stream event.
    pub fn process_event(&mut self, event: &StreamEvent) {
        match event {
            StreamEvent::Start { chat_completion_id } => {
                self.chat_completion_id = Some(chat_completion_id.clone());
            }
            StreamEvent::ContentDelta { delta } => {
                self.content.push_str(delta);
            }
            StreamEvent::ToolCallsDelta { tool_calls } => {
                self.tool_calls.extend(tool_calls.clone());
            }
            StreamEvent::End { finish_reason } => {
                self.finish_reason = *finish_reason;
            }
            StreamEvent::Usage { usage } => {
                self.usage = Some(usage.clone());
            }
        }
    }

    /// Get the collected content so far.
    pub fn current_content(&self) -> &str {
        &self.content
    }

    /// Get the chat completion ID if available.
    pub fn chat_completion_id(&self) -> Option<&str> {
        self.chat_completion_id.as_deref()
    }

    /// Get the finish reason if available.
    pub fn finish_reason(&self) -> Option<FinishReason> {
        self.finish_reason
    }

    /// Get the collected tool calls.
    pub fn tool_calls(&self) -> &[ToolCall] {
        &self.tool_calls
    }

    /// Get the usage if available.
    pub fn usage(&self) -> Option<&Usage> {
        self.usage.as_ref()
    }

    /// Check if the stream has finished.
    pub fn is_complete(&self) -> bool {
        self.finish_reason.is_some()
    }

    /// Build the final content.
    pub fn build_content(&self) -> Option<String> {
        if self.content.is_empty() {
            None
        } else {
            Some(self.content.clone())
        }
    }

    /// Get the total tokens if usage is available.
    pub fn total_tokens(&self) -> Option<usize> {
        self.usage.as_ref().map(|u| u.total_tokens)
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
        let start = StreamEvent::Start {
            chat_completion_id: "chat_123".to_string(),
        };
        collector.process_event(&start);

        let delta1 = StreamEvent::ContentDelta {
            delta: "Hello".to_string(),
        };
        collector.process_event(&delta1);

        let delta2 = StreamEvent::ContentDelta {
            delta: " world!".to_string(),
        };
        collector.process_event(&delta2);

        assert_eq!(collector.current_content(), "Hello world!");
        assert_eq!(collector.chat_completion_id(), Some("chat_123"));
        assert!(!collector.is_complete());

        let end = StreamEvent::End {
            finish_reason: Some(FinishReason::Stop),
        };
        collector.process_event(&end);

        assert!(collector.is_complete());
        assert_eq!(collector.finish_reason(), Some(FinishReason::Stop));
    }

    #[test]
    fn test_stream_collector_with_usage() {
        let mut collector = StreamCollector::new();

        let usage = StreamEvent::Usage {
            usage: Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
        };
        collector.process_event(&usage);

        assert_eq!(collector.total_tokens(), Some(15));
        assert!(collector.usage().is_some());
    }
}

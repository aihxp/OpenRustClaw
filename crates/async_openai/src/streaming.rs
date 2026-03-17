//! Streaming response handling for the OpenAI API.

use serde::{Deserialize, Serialize};

/// A chat completion chunk in a stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    /// Unique identifier for the chunk.
    pub id: String,
    /// The object type (always "chat.completion.chunk").
    pub object: String,
    /// The Unix timestamp.
    pub created: i64,
    /// The model used.
    pub model: String,
    /// The list of choices.
    pub choices: Vec<StreamChoice>,
    /// System fingerprint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

impl ChatCompletionChunk {
    /// Check if this is the final chunk.
    pub fn is_done(&self) -> bool {
        self.choices
            .iter()
            .all(|c| c.delta.is_empty() && c.finish_reason.is_some())
    }

    /// Create a done chunk.
    pub fn done() -> Self {
        Self {
            id: String::new(),
            object: "chat.completion.chunk".to_string(),
            created: 0,
            model: String::new(),
            choices: vec![],
            system_fingerprint: None,
        }
    }

    /// Get the content delta from the first choice.
    pub fn content(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|c| c.delta.content.as_deref())
    }

    /// Get tool call deltas from the first choice.
    pub fn tool_calls(&self) -> Option<&Vec<ToolCallDelta>> {
        self.choices
            .first()
            .and_then(|c| c.delta.tool_calls.as_ref())
    }

    /// Check if the first choice has finished.
    pub fn finish_reason(&self) -> Option<crate::types::FinishReason> {
        self.choices.first().and_then(|c| c.finish_reason)
    }
}

/// A choice in a streaming chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    /// The index of this choice.
    pub index: usize,
    /// The delta.
    pub delta: StreamDelta,
    /// The reason the completion finished.
    pub finish_reason: Option<crate::types::FinishReason>,
    /// Log probabilities (if requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

/// A delta in a streaming response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamDelta {
    /// The role (only present in the first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<crate::types::Role>,
    /// The content delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Tool call deltas.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallDelta>>,
    /// Function call (deprecated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCallDelta>,
}

impl StreamDelta {
    /// Check if this delta is empty.
    pub fn is_empty(&self) -> bool {
        self.role.is_none()
            && self.content.is_none()
            && self.tool_calls.is_none()
            && self.function_call.is_none()
    }
}

/// A tool call delta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallDelta {
    /// The index of this tool call.
    pub index: usize,
    /// The ID of the tool call (present in first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The type of tool call (present in first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub call_type: Option<String>,
    /// The function delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<FunctionDelta>,
}

/// A function delta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDelta {
    /// The name (present in first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The arguments delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

/// A function call delta (deprecated).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallDelta {
    /// The name (present in first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The arguments delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

/// A collector that accumulates streaming chunks into a complete response.
#[derive(Debug, Default)]
pub struct StreamCollector {
    message_id: Option<String>,
    model: Option<String>,
    content: String,
    tool_calls: Vec<PartialToolCall>,
    finish_reason: Option<crate::types::FinishReason>,
}

#[derive(Debug)]
struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
}

impl StreamCollector {
    /// Create a new stream collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a chunk.
    pub fn process_chunk(&mut self, chunk: &ChatCompletionChunk) {
        if !chunk.id.is_empty() {
            self.message_id = Some(chunk.id.clone());
        }
        if !chunk.model.is_empty() {
            self.model = Some(chunk.model.clone());
        }

        if let Some(choice) = chunk.choices.first() {
            if let Some(content) = &choice.delta.content {
                self.content.push_str(content);
            }

            if let Some(tool_call_deltas) = &choice.delta.tool_calls {
                for delta in tool_call_deltas {
                    // Ensure we have enough slots
                    while self.tool_calls.len() <= delta.index {
                        self.tool_calls.push(PartialToolCall {
                            id: String::new(),
                            name: String::new(),
                            arguments: String::new(),
                        });
                    }

                    let tool_call = &mut self.tool_calls[delta.index];

                    if let Some(id) = &delta.id {
                        tool_call.id = id.clone();
                    }
                    if let Some(function) = &delta.function {
                        if let Some(name) = &function.name {
                            tool_call.name = name.clone();
                        }
                        if let Some(args) = &function.arguments {
                            tool_call.arguments.push_str(args);
                        }
                    }
                }
            }

            if let Some(reason) = choice.finish_reason {
                self.finish_reason = Some(reason);
            }
        }
    }

    /// Get the collected content so far.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the collected tool calls.
    pub fn tool_calls(&self) -> Vec<crate::types::ToolCall> {
        self.tool_calls
            .iter()
            .map(|tc| crate::types::ToolCall {
                id: tc.id.clone(),
                call_type: "function".to_string(),
                function: crate::types::FunctionCall {
                    name: tc.name.clone(),
                    arguments: tc.arguments.clone(),
                },
            })
            .collect()
    }

    /// Check if the stream has finished.
    pub fn is_complete(&self) -> bool {
        self.finish_reason.is_some()
    }

    /// Get the finish reason.
    pub fn finish_reason(&self) -> Option<crate::types::FinishReason> {
        self.finish_reason
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_collector() {
        let mut collector = StreamCollector::new();

        let chunk1 = ChatCompletionChunk {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: Some(crate::types::Role::Assistant),
                    content: Some("Hello".to_string()),
                    tool_calls: None,
                    function_call: None,
                },
                finish_reason: None,
                logprobs: None,
            }],
            system_fingerprint: None,
        };
        collector.process_chunk(&chunk1);

        let chunk2 = ChatCompletionChunk {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: None,
                    content: Some(" world!".to_string()),
                    tool_calls: None,
                    function_call: None,
                },
                finish_reason: Some(crate::types::FinishReason::Stop),
                logprobs: None,
            }],
            system_fingerprint: None,
        };
        collector.process_chunk(&chunk2);

        assert_eq!(collector.content(), "Hello world!");
        assert!(collector.is_complete());
    }

    #[test]
    fn test_stream_delta_empty() {
        let delta = StreamDelta::default();
        assert!(delta.is_empty());
    }
}

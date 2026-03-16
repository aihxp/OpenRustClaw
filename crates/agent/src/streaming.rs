//! Unified streaming with tool call delta buffering.
//!
//! Fixes Ollama's broken tool call streaming by buffering deltas
//! until a complete tool call is assembled.

use openrustclaw_core::types::{StreamChunk, ToolCall};
use serde_json::Value;

/// Buffer for accumulating streaming tool call deltas.
pub struct ToolCallBuffer {
    calls: Vec<BufferedToolCall>,
}

struct BufferedToolCall {
    id: String,
    name: String,
    arguments: String,
}

impl ToolCallBuffer {
    pub fn new() -> Self {
        Self { calls: Vec::new() }
    }

    /// Process a stream chunk. Returns completed tool calls if any.
    pub fn process(&mut self, chunk: &StreamChunk) -> Vec<ToolCall> {
        match chunk {
            StreamChunk::ToolCallDelta {
                id,
                name,
                arguments_delta,
            } => {
                // Find or create the buffered call
                let existing = self.calls.iter_mut().find(|c| c.id == *id);
                if let Some(call) = existing {
                    call.arguments.push_str(arguments_delta);
                } else {
                    self.calls.push(BufferedToolCall {
                        id: id.clone(),
                        name: name.clone().unwrap_or_default(),
                        arguments: arguments_delta.clone(),
                    });
                }
                vec![] // Not complete yet
            }
            StreamChunk::Done { .. } => {
                // Emit all buffered calls
                self.flush()
            }
            _ => vec![],
        }
    }

    /// Flush all buffered tool calls.
    pub fn flush(&mut self) -> Vec<ToolCall> {
        self.calls
            .drain(..)
            .filter_map(|call| {
                let arguments: Value =
                    serde_json::from_str(&call.arguments).unwrap_or(Value::Null);
                Some(ToolCall {
                    id: call.id,
                    name: call.name,
                    arguments,
                })
            })
            .collect()
    }

    /// Check if there are pending deltas.
    pub fn has_pending(&self) -> bool {
        !self.calls.is_empty()
    }
}

impl Default for ToolCallBuffer {
    fn default() -> Self {
        Self::new()
    }
}

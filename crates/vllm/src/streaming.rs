//! Streaming response handling for vLLM.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{Stream, StreamExt};
use pin_project_lite::pin_project;
use serde::{Deserialize, Serialize};

use crate::error::{Result, VllmError};
use crate::types::{Role, TokenUsage, ToolCall};

/// A chat completion chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    /// Unique identifier.
    pub id: String,
    /// Object type.
    pub object: String,
    /// Unix timestamp.
    pub created: i64,
    /// Model used.
    pub model: String,
    /// Choices.
    pub choices: Vec<StreamChoice>,
    /// Usage statistics (only present in the final chunk when `stream_options` is enabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
}

impl ChatCompletionChunk {
    /// Get the content delta from the first choice.
    pub fn content(&self) -> &str {
        self.choices
            .first()
            .and_then(|c| c.delta.content.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// Check if this is the final chunk (indicated by finish_reason).
    pub fn is_final(&self) -> bool {
        self.choices
            .first()
            .and_then(|c| c.finish_reason.as_ref())
            .is_some()
    }

    /// Get the finish reason if present.
    pub fn finish_reason(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|c| c.finish_reason.as_deref())
    }

    /// Get tool calls from the delta.
    pub fn tool_calls(&self) -> Option<&Vec<ToolCall>> {
        self.choices
            .first()
            .and_then(|c| c.delta.tool_calls.as_ref())
    }
}

/// A completion chunk (legacy).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionChunk {
    /// Unique identifier.
    pub id: String,
    /// Object type.
    pub object: String,
    /// Unix timestamp.
    pub created: i64,
    /// Model used.
    pub model: String,
    /// Choices.
    pub choices: Vec<CompletionStreamChoice>,
}

impl CompletionChunk {
    /// Get the text from the first choice.
    pub fn text(&self) -> &str {
        self.choices
            .first()
            .map(|c| c.text.as_str())
            .unwrap_or("")
    }

    /// Check if this is the final chunk.
    pub fn is_final(&self) -> bool {
        self.choices
            .first()
            .and_then(|c| c.finish_reason.as_ref())
            .is_some()
    }

    /// Get the finish reason if present.
    pub fn finish_reason(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|c| c.finish_reason.as_deref())
    }
}

/// A choice in a streaming chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    /// Index.
    pub index: usize,
    /// Delta.
    pub delta: StreamDelta,
    /// Finish reason.
    pub finish_reason: Option<String>,
    /// Logprobs (if requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

/// A choice in a completion stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionStreamChoice {
    /// Index.
    pub index: usize,
    /// Text delta.
    pub text: String,
    /// Logprobs (if requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
    /// Finish reason.
    pub finish_reason: Option<String>,
}

/// A delta in a stream.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamDelta {
    /// Role (only present in first chunk).
    pub role: Option<Role>,
    /// Content.
    pub content: Option<String>,
    /// Tool calls.
    pub tool_calls: Option<Vec<ToolCall>>,
}

pin_project! {
    /// A stream of chat completion chunks.
    pub struct ChatStream {
        #[pin]
        inner: Pin<Box<dyn Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>> + Send>>,
        buffer: String,
    }
}

impl ChatStream {
    /// Create a new chat completion stream from an HTTP response.
    pub fn new(response: reqwest::Response) -> Self {
        Self {
            inner: Box::pin(response.bytes_stream()),
            buffer: String::new(),
        }
    }
}

impl Stream for ChatStream {
    type Item = Result<ChatCompletionChunk>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this.inner.poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                // Append to buffer
                this.buffer.push_str(&String::from_utf8_lossy(&bytes));

                // Process complete lines
                if let Some(pos) = this.buffer.find('\n') {
                    let line = this.buffer[..pos].trim().to_string();
                    *this.buffer = this.buffer[pos + 1..].to_string();

                    // Handle SSE format: "data: {...}"
                    if line.starts_with("data: ") {
                        let data = &line[6..]; // Skip "data: "

                        if data == "[DONE]" {
                            return Poll::Ready(None); // End of stream
                        }

                        match serde_json::from_str::<ChatCompletionChunk>(data) {
                            Ok(chunk) => return Poll::Ready(Some(Ok(chunk))),
                            Err(e) => {
                                return Poll::Ready(Some(Err(VllmError::Stream {
                                    message: format!("Failed to parse chunk: {e}"),
                                })));
                            }
                        }
                    }
                }

                // If we got bytes but no complete line, continue polling
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Some(Err(VllmError::Http { source: e })))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

pin_project! {
    /// A stream of completion chunks.
    pub struct CompletionStream {
        #[pin]
        inner: Pin<Box<dyn Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>> + Send>>,
        buffer: String,
    }
}

impl CompletionStream {
    /// Create a new completion stream from an HTTP response.
    pub fn new(response: reqwest::Response) -> Self {
        Self {
            inner: Box::pin(response.bytes_stream()),
            buffer: String::new(),
        }
    }
}

impl Stream for CompletionStream {
    type Item = Result<CompletionChunk>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this.inner.poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                // Append to buffer
                this.buffer.push_str(&String::from_utf8_lossy(&bytes));

                // Process complete lines
                if let Some(pos) = this.buffer.find('\n') {
                    let line = this.buffer[..pos].trim().to_string();
                    *this.buffer = this.buffer[pos + 1..].to_string();

                    // Handle SSE format: "data: {...}"
                    if line.starts_with("data: ") {
                        let data = &line[6..]; // Skip "data: "

                        if data == "[DONE]" {
                            return Poll::Ready(None); // End of stream
                        }

                        match serde_json::from_str::<CompletionChunk>(data) {
                            Ok(chunk) => return Poll::Ready(Some(Ok(chunk))),
                            Err(e) => {
                                return Poll::Ready(Some(Err(VllmError::Stream {
                                    message: format!("Failed to parse chunk: {e}"),
                                })));
                            }
                        }
                    }
                }

                // If we got bytes but no complete line, continue polling
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Some(Err(VllmError::Http { source: e })))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Collect all chunks into a single string.
///
/// # Example
///
/// ```ignore
/// use vllm::streaming::collect_chat_stream;
///
/// let stream = client.chat().complete_stream(request).await?;
/// let text = collect_chat_stream(stream).await?;
/// println!("{}", text);
/// ```
pub async fn collect_chat_stream(
    mut stream: impl Stream<Item = Result<ChatCompletionChunk>> + Unpin,
) -> Result<String> {
    let mut text = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        text.push_str(chunk.content());
    }

    Ok(text)
}

/// Collect all chunks into a single string.
pub async fn collect_completion_stream(
    mut stream: impl Stream<Item = Result<CompletionChunk>> + Unpin,
) -> Result<String> {
    let mut text = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        text.push_str(chunk.text());
    }

    Ok(text)
}

/// Helper function to stream chat completions with a simple callback.
///
/// # Example
///
/// ```ignore
/// use vllm::streaming::stream_chat_with_callback;
///
/// stream_chat_with_callback(stream, |chunk| {
///     print!("{}", chunk.content());
///     std::io::Write::flush(&mut std::io::stdout())?;
///     Ok(())
/// }).await?;
/// ```
pub async fn stream_chat_with_callback<F, Fut>(
    mut stream: impl Stream<Item = Result<ChatCompletionChunk>> + Unpin,
    mut callback: F,
) -> Result<()>
where
    F: FnMut(&ChatCompletionChunk) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        callback(&chunk).await?;
    }

    Ok(())
}

/// Stream collector that accumulates chunks.
#[derive(Debug, Default)]
pub struct StreamCollector {
    content: String,
    tool_calls: Vec<ToolCall>,
    finish_reason: Option<String>,
}

impl StreamCollector {
    /// Create a new stream collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a chunk to the collector.
    pub fn add_chunk(&mut self, chunk: &ChatCompletionChunk) {
        // Accumulate content
        self.content.push_str(chunk.content());

        // Accumulate tool calls
        if let Some(calls) = chunk.tool_calls() {
            self.tool_calls.extend(calls.clone());
        }

        // Update finish reason
        if let Some(reason) = chunk.finish_reason() {
            self.finish_reason = Some(reason.to_string());
        }
    }

    /// Get the accumulated content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the accumulated tool calls.
    pub fn tool_calls(&self) -> &[ToolCall] {
        &self.tool_calls
    }

    /// Get the finish reason.
    pub fn finish_reason(&self) -> Option<&str> {
        self.finish_reason.as_deref()
    }

    /// Check if the stream is complete.
    pub fn is_complete(&self) -> bool {
        self.finish_reason.is_some()
    }

    /// Clear the collector.
    pub fn clear(&mut self) {
        self.content.clear();
        self.tool_calls.clear();
        self.finish_reason = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_collector() {
        let mut collector = StreamCollector::new();

        // First chunk
        let chunk1 = ChatCompletionChunk {
            id: "1".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "test".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: Some(Role::Assistant),
                    content: Some("Hello".to_string()),
                    tool_calls: None,
                },
                finish_reason: None,
                logprobs: None,
            }],
            usage: None,
        };
        collector.add_chunk(&chunk1);
        assert_eq!(collector.content(), "Hello");
        assert!(!collector.is_complete());

        // Second chunk
        let chunk2 = ChatCompletionChunk {
            id: "2".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "test".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: None,
                    content: Some(" world!".to_string()),
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
                logprobs: None,
            }],
            usage: None,
        };
        collector.add_chunk(&chunk2);
        assert_eq!(collector.content(), "Hello world!");
        assert!(collector.is_complete());
        assert_eq!(collector.finish_reason(), Some("stop"));
    }
}

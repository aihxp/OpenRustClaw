//! Streaming response handling for Fireworks AI.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{Stream, StreamExt};
use pin_project_lite::pin_project;
use serde::{Deserialize, Serialize};

use crate::error::{FireworksError, Result};

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
    /// Usage statistics (only present in the final chunk when `stream_tokens` is enabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<crate::types::TokenUsage>,
}

impl ChatCompletionChunk {
    /// Get the content delta from the first choice.
    pub fn content(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|c| c.delta.content.as_ref())
            .map(|s| s.as_str())
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
    pub fn tool_calls(&self) -> Option<&Vec<crate::types::ToolCall>> {
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
    pub role: Option<crate::types::Role>,
    /// Content.
    pub content: Option<String>,
    /// Tool calls.
    pub tool_calls: Option<Vec<crate::types::ToolCall>>,
}

pin_project! {
    /// A stream of chat completion chunks.
    pub struct ChatCompletionStream {
        #[pin]
        inner: Pin<Box<dyn Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>> + Send>>,
    }
}

impl ChatCompletionStream {
    /// Create a new chat completion stream from an HTTP response.
    pub fn new(response: reqwest::Response) -> Self {
        Self {
            inner: Box::pin(response.bytes_stream()),
        }
    }
}

impl Stream for ChatCompletionStream {
    type Item = Result<ChatCompletionChunk>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this.inner.poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                // Parse the bytes as SSE event
                let text = String::from_utf8_lossy(&bytes);
                
                // Handle SSE format: "data: {...}\n\n" or multiple lines
                for line in text.lines() {
                    let line = line.trim();
                    if let Some(data) = line.strip_prefix("data: ") {
                        
                        if data == "[DONE]" {
                            continue; // End of stream
                        }

                        match serde_json::from_str::<ChatCompletionChunk>(data) {
                            Ok(chunk) => return Poll::Ready(Some(Ok(chunk))),
                            Err(e) => {
                                return Poll::Ready(Some(Err(FireworksError::Stream {
                                    message: format!("Failed to parse chunk: {e}"),
                                })));
                            }
                        }
                    }
                }
                
                // If we got bytes but no valid data line, continue polling
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Some(Err(FireworksError::Http { source: e })))
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
    }
}

impl CompletionStream {
    /// Create a new completion stream from an HTTP response.
    pub fn new(response: reqwest::Response) -> Self {
        Self {
            inner: Box::pin(response.bytes_stream()),
        }
    }
}

impl Stream for CompletionStream {
    type Item = Result<CompletionChunk>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this.inner.poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                // Parse the bytes as SSE event
                let text = String::from_utf8_lossy(&bytes);
                
                // Handle SSE format: "data: {...}\n\n" or multiple lines
                for line in text.lines() {
                    let line = line.trim();
                    if let Some(data) = line.strip_prefix("data: ") {
                        
                        if data == "[DONE]" {
                            continue; // End of stream
                        }

                        match serde_json::from_str::<CompletionChunk>(data) {
                            Ok(chunk) => return Poll::Ready(Some(Ok(chunk))),
                            Err(e) => {
                                return Poll::Ready(Some(Err(FireworksError::Stream {
                                    message: format!("Failed to parse chunk: {e}"),
                                })));
                            }
                        }
                    }
                }
                
                // If we got bytes but no valid data line, continue polling
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Some(Err(FireworksError::Http { source: e })))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Collect all chunks into a single string.
pub async fn collect_chat_stream(
    mut stream: impl Stream<Item = Result<ChatCompletionChunk>> + Unpin,
) -> Result<String> {
    let mut text = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(content) = chunk.content() {
            text.push_str(content);
        }
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
/// stream_chat_with_callback(&client, request, |chunk| {
///     print!("{}", chunk.content().unwrap_or(""));
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

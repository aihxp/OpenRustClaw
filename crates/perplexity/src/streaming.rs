//! Streaming support for Perplexity chat completions.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{Stream, StreamExt};
use pin_project_lite::pin_project;
use serde::{Deserialize, Serialize};

use crate::error::{PerplexityError, Result};
use crate::types::{Citation, FinishReason, RelatedQuestion, ToolCall};

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
                                return Poll::Ready(Some(Err(PerplexityError::Stream {
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
                Poll::Ready(Some(Err(PerplexityError::Http { source: e })))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// A chunk of a streaming chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    /// The unique identifier for the completion.
    pub id: String,

    /// The object type (always "chat.completion.chunk").
    pub object: String,

    /// The Unix timestamp when the chunk was created.
    pub created: u64,

    /// The model used for the completion.
    pub model: String,

    /// The list of choices in this chunk.
    pub choices: Vec<StreamChoice>,

    /// Citations for the response (Perplexity-specific, may appear in final chunk).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub citations: Vec<Citation>,

    /// Related questions (Perplexity-specific, may appear in final chunk).
    #[serde(
        default,
        rename = "related_questions",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub related_questions: Vec<RelatedQuestion>,
}

impl ChatCompletionChunk {
    /// Get the delta content from the first choice.
    pub fn delta_content(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|c| c.delta.content.as_deref())
    }

    /// Check if this is the final chunk.
    pub fn is_final(&self) -> bool {
        self.choices
            .first()
            .map(|c| c.finish_reason.is_some())
            .unwrap_or(false)
    }

    /// Get the finish reason if this is the final chunk.
    pub fn finish_reason(&self) -> Option<FinishReason> {
        self.choices.first().and_then(|c| c.finish_reason)
    }

    /// Get citations if present (usually in final chunk).
    pub fn citations(&self) -> &[Citation] {
        &self.citations
    }

    /// Check if this chunk has citations.
    pub fn has_citations(&self) -> bool {
        !self.citations.is_empty()
    }

    /// Get related questions if present (usually in final chunk).
    pub fn related_questions(&self) -> &[RelatedQuestion] {
        &self.related_questions
    }

    /// Check if this chunk has related questions.
    pub fn has_related_questions(&self) -> bool {
        !self.related_questions.is_empty()
    }
}

/// A choice in a streaming response chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    /// The index of the choice.
    pub index: usize,

    /// The delta (incremental update).
    pub delta: StreamDelta,

    /// The reason the completion finished.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
}

/// A delta in a streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDelta {
    /// The role (only present in the first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// The content delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Tool calls (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Collector to accumulate streaming chunks into a complete response.
#[derive(Debug, Default)]
pub struct StreamCollector {
    content: String,
    citations: Vec<Citation>,
    related_questions: Vec<RelatedQuestion>,
    finish_reason: Option<FinishReason>,
}

impl StreamCollector {
    /// Create a new stream collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a chunk and accumulate its content.
    pub fn process_chunk(&mut self, chunk: &ChatCompletionChunk) {
        // Accumulate content
        if let Some(content) = chunk.delta_content() {
            self.content.push_str(content);
        }

        // Store citations if present (usually in final chunk)
        if !chunk.citations.is_empty() {
            self.citations = chunk.citations.clone();
        }

        // Store related questions if present (usually in final chunk)
        if !chunk.related_questions.is_empty() {
            self.related_questions = chunk.related_questions.clone();
        }

        // Track finish reason
        if let Some(reason) = chunk.finish_reason() {
            self.finish_reason = Some(reason);
        }
    }

    /// Get the accumulated content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Take the accumulated content.
    pub fn into_content(self) -> String {
        self.content
    }

    /// Get the citations.
    pub fn citations(&self) -> &[Citation] {
        &self.citations
    }

    /// Take the citations.
    pub fn into_citations(self) -> Vec<Citation> {
        self.citations
    }

    /// Get the related questions.
    pub fn related_questions(&self) -> &[RelatedQuestion] {
        &self.related_questions
    }

    /// Take the related questions.
    pub fn into_related_questions(self) -> Vec<RelatedQuestion> {
        self.related_questions
    }

    /// Get the finish reason.
    pub fn finish_reason(&self) -> Option<FinishReason> {
        self.finish_reason
    }

    /// Check if the stream is complete.
    pub fn is_complete(&self) -> bool {
        self.finish_reason.is_some()
    }
}

/// Collect all chunks into a single string.
pub async fn collect_stream(
    mut stream: impl Stream<Item = Result<ChatCompletionChunk>> + Unpin,
) -> Result<String> {
    let mut text = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(content) = chunk.delta_content() {
            text.push_str(content);
        }
    }

    Ok(text)
}

/// Helper function to stream chat completions with a simple callback.
///
/// # Example
///
/// ```ignore
/// stream_chat_with_callback(stream, |chunk| {
///     print!("{}", chunk.delta_content().unwrap_or(""));
///     std::io::Write::flush(&mut std::io::stdout())?;
///     Ok(())
/// }).await?;
/// ```
pub async fn stream_with_callback<F, Fut>(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_collector() {
        let mut collector = StreamCollector::new();

        // Simulate chunks
        let chunk1 = ChatCompletionChunk {
            id: "chunk1".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "llama-3.1-sonar-small-128k-online".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: Some("assistant".to_string()),
                    content: Some("Hello".to_string()),
                    tool_calls: None,
                },
                finish_reason: None,
            }],
            citations: vec![],
            related_questions: vec![],
        };

        let chunk2 = ChatCompletionChunk {
            id: "chunk2".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "llama-3.1-sonar-small-128k-online".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: None,
                    content: Some(" world!".to_string()),
                    tool_calls: None,
                },
                finish_reason: Some(FinishReason::Stop),
            }],
            citations: vec![Citation {
                index: 1,
                url: "https://example.com".to_string(),
                title: Some("Example".to_string()),
                published_date: None,
            }],
            related_questions: vec![RelatedQuestion {
                question: "How are you?".to_string(),
            }],
        };

        collector.process_chunk(&chunk1);
        assert_eq!(collector.content(), "Hello");
        assert!(!collector.is_complete());

        collector.process_chunk(&chunk2);
        assert_eq!(collector.content(), "Hello world!");
        assert!(collector.is_complete());
        assert_eq!(collector.citations().len(), 1);
        assert_eq!(collector.related_questions().len(), 1);
    }

    #[test]
    fn test_chunk_helpers() {
        let chunk = ChatCompletionChunk {
            id: "chunk1".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "llama-3.1-sonar-small-128k-online".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: Some("assistant".to_string()),
                    content: Some("Test".to_string()),
                    tool_calls: None,
                },
                finish_reason: Some(FinishReason::Stop),
            }],
            citations: vec![Citation {
                index: 1,
                url: "https://test.com".to_string(),
                title: None,
                published_date: None,
            }],
            related_questions: vec![],
        };

        assert_eq!(chunk.delta_content(), Some("Test"));
        assert!(chunk.is_final());
        assert_eq!(chunk.finish_reason(), Some(FinishReason::Stop));
        assert!(chunk.has_citations());
        assert!(!chunk.has_related_questions());
    }
}

//! Streaming response handling for Gemini API

use crate::{GenerateContentResponse, GeminiError};
use bytes::Bytes;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};

/// Parse a streaming chunk from Gemini API
pub fn parse_chunk(chunk: &str) -> Vec<Result<GenerateContentResponse, GeminiError>> {
    let mut responses = Vec::new();
    
    // Gemini returns newline-delimited JSON
    for line in chunk.lines() {
        let line = line.trim();
        
        if line.is_empty() {
            continue;
        }
        
        // Handle SSE format (data: prefix)
        let json_str = if line.starts_with("data: ") {
            &line[6..]
        } else {
            line
        };
        
        if json_str == "[DONE]" {
            continue;
        }
        
        match serde_json::from_str::<GenerateContentResponse>(json_str) {
            Ok(response) => responses.push(Ok(response)),
            Err(e) => responses.push(Err(GeminiError::JsonError(e))),
        }
    }
    
    responses
}

/// Stream adapter for converting bytes to Gemini responses
pub struct GeminiStream<S> {
    inner: S,
    buffer: String,
}

impl<S> GeminiStream<S> {
    /// Create new Gemini stream adapter
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            buffer: String::new(),
        }
    }
}

impl<S> Stream for GeminiStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    type Item = Result<GenerateContentResponse, GeminiError>;
    
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                let text = String::from_utf8_lossy(&bytes);
                self.buffer.push_str(&text);
                
                // Try to parse complete responses from buffer
                let lines: Vec<&str> = self.buffer.lines().collect();
                if lines.len() > 1 {
                    // Keep the last (potentially incomplete) line in buffer
                    let complete = lines[..lines.len() - 1].join("\n");
                    self.buffer = lines[lines.len() - 1].to_string();
                    
                    // Parse the complete lines
                    let responses = parse_chunk(&complete);
                    // For simplicity, return just the first response
                    // In production, you'd want to buffer and return all
                    if let Some(response) = responses.into_iter().next() {
                        return Poll::Ready(Some(response));
                    }
                }
                
                Poll::Pending
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(GeminiError::RequestError(e)))),
            Poll::Ready(None) => {
                // Try to parse any remaining buffer
                if !self.buffer.is_empty() {
                    let responses = parse_chunk(&self.buffer.clone());
                    self.buffer.clear();
                    if let Some(response) = responses.into_iter().next() {
                        return Poll::Ready(Some(response));
                    }
                }
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Collect a stream of responses into a single response
pub async fn collect_stream<S>(stream: S) -> Result<GenerateContentResponse, GeminiError>
where
    S: Stream<Item = Result<GenerateContentResponse, GeminiError>>,
{
    use futures::pin_mut;
    
    pin_mut!(stream);
    
    let mut collected_text = String::new();
    let mut final_candidates = Vec::new();
    let mut total_usage = None::<crate::types::UsageMetadata>;
    
    while let Some(result) = stream.next().await {
        let response = result?;
        
        for candidate in &response.candidates {
            for part in &candidate.content.parts {
                if let crate::types::Part::Text { text } = part {
                    collected_text.push_str(text);
                }
            }
        }
        
        // Keep track of the last candidates structure
        if !response.candidates.is_empty() {
            final_candidates = response.candidates.clone();
        }
        
        // Accumulate usage metadata
        if let Some(usage) = &response.usage_metadata {
            total_usage = Some(match total_usage {
                Some(mut existing) => {
                    existing.prompt_token_count += usage.prompt_token_count;
                    existing.candidates_token_count += usage.candidates_token_count;
                    existing.total_token_count += usage.total_token_count;
                    existing
                }
                None => usage.clone(),
            });
        }
    }
    
    // Create final response with collected text
    if final_candidates.is_empty() {
        return Err(GeminiError::InvalidRequest("No candidates in stream".to_string()));
    }
    
    // Update the first candidate with collected text
    if let Some(first) = final_candidates.first_mut() {
        first.content.parts = vec![crate::types::Part::Text { text: collected_text }];
    }
    
    Ok(GenerateContentResponse {
        candidates: final_candidates,
        usage_metadata: total_usage,
        prompt_feedback: None,
    })
}

/// Error type for streaming
#[derive(Debug)]
pub enum StreamError {
    ParseError(String),
    ConnectionError(String),
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            StreamError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
        }
    }
}

impl std::error::Error for StreamError {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_chunk() {
        let chunk = r#"{"candidates": [{"content": {"role": "model", "parts": [{"text": "Hello"}]}}]}
{"candidates": [{"content": {"role": "model", "parts": [{"text": " World"}]}}]}"#;
        
        let results = parse_chunk(chunk);
        assert_eq!(results.len(), 2);
    }
    
    #[test]
    fn test_parse_sse_chunk() {
        let chunk = r#"data: {"candidates": [{"content": {"role": "model", "parts": [{"text": "Hello"}]}}]}

data: {"candidates": [{"content": {"role": "model", "parts": [{"text": " World"}]}}]}

data: [DONE]"#;
        
        let results = parse_chunk(chunk);
        assert_eq!(results.len(), 2);
    }
}

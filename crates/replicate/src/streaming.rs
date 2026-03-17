//! Streaming API - Stream prediction output in real-time.

use futures::{Stream, StreamExt};
use pin_project_lite::pin_project;
use serde::Deserialize;
use std::pin::Pin;
use std::task::{Context, Poll};
use tracing::{debug, trace, warn};

use crate::client::ReplicateClient;
use crate::error::{ReplicateError, Result};

/// Streaming API client.
#[derive(Debug)]
pub struct Streaming<'a> {
    client: &'a ReplicateClient,
}

impl<'a> Streaming<'a> {
    /// Create a new streaming API client.
    pub fn new(client: &'a ReplicateClient) -> Self {
        Self { client }
    }

    /// Stream the output of a prediction.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::ReplicateClient;
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// // First create a prediction
    /// let prediction = client.predictions()
    ///     .create(replicate::PredictionRequest::new().model("meta/meta-llama-3-70b-instruct"))
    ///     .await?;
    ///
    /// // Stream the output
    /// let mut stream = client.streaming().stream_output(&prediction.id).await?;
    /// while let Some(event) = stream.next().await {
    ///     match event {
    ///         Ok(event) => println!("Event: {:?}", event),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn stream_output(&self, prediction_id: &str) -> Result<PredictionStream> {
        // First get the prediction to find the stream URL
        let prediction = crate::predictions::Predictions::new(self.client)
            .get(prediction_id)
            .await?;
        
        let stream_url = prediction
            .urls
            .stream
            .ok_or_else(|| ReplicateError::Stream {
                message: "Prediction does not support streaming".to_string(),
            })?;
        
        debug!(stream_url = %stream_url, "Starting stream");
        
        self.stream_from_url(&stream_url).await
    }

    /// Stream from a specific URL.
    ///
    /// This is useful if you have a stream URL from a webhook or other source.
    pub async fn stream_from_url(&self, url: &str) -> Result<PredictionStream> {
        let response = self
            .client
            .execute_with_retry(|| {
                let url = url.to_string();
                Box::pin(async move {
                    reqwest::Client::new()
                        .get(&url)
                        .header("Accept", "text/event-stream")
                        .send()
                        .await
                        .map_err(ReplicateError::from)
                })
            })
            .await?;
        
        if !response.status().is_success() {
            return Err(ReplicateError::Stream {
                message: format!("Failed to start stream: {}", response.status()),
            });
        }
        
        let stream = response
            .bytes_stream()
            .map(|result| result.map_err(ReplicateError::from));
        
        Ok(PredictionStream::new(stream))
    }
}

pin_project! {
    /// A stream of prediction output events.
    pub struct PredictionStream {
        #[pin]
        inner: Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>,
        buffer: String,
    }
}

impl PredictionStream {
    /// Create a new prediction stream from a byte stream.
    fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<bytes::Bytes>> + Send + 'static,
    {
        let inner = Box::pin(stream.then(|result| async move {
            match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    parse_sse_event(&text)
                }
                Err(e) => Err(e),
            }
        }));
        
        Self {
            inner,
            buffer: String::new(),
        }
    }
}

impl Stream for PredictionStream {
    type Item = Result<StreamEvent>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        this.inner.poll_next(cx)
    }
}

/// An event from a prediction stream.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "event", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum StreamEvent {
    /// New output from the prediction.
    Output(serde_json::Value),
    /// Log output from the prediction.
    Logs(String),
    /// Prediction completed.
    Completed(StreamCompleted),
    /// An error occurred.
    Error(StreamError),
}

/// Stream completed event.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamCompleted {
    /// The prediction ID.
    pub prediction_id: String,
    /// The final status.
    pub status: String,
}

/// Stream error event.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamError {
    /// The error message.
    pub message: String,
}

/// Parse a Server-Sent Events (SSE) message.
fn parse_sse_event(text: &str) -> Result<StreamEvent> {
    trace!(text = %text, "Parsing SSE event");
    
    let mut event_type = None;
    let mut data = None;
    
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        
        if let Some(value) = line.strip_prefix("event: ") {
            event_type = Some(value.trim());
        } else if let Some(value) = line.strip_prefix("data: ") {
            data = Some(value.trim());
        }
    }
    
    let event_type = event_type.unwrap_or("output");
    let data = data.ok_or_else(|| ReplicateError::Stream {
        message: "Missing data in SSE event".to_string(),
    })?;
    
    match event_type {
        "output" => {
            let value = serde_json::from_str(data).map_err(|e| ReplicateError::Stream {
                message: format!("Failed to parse output: {}", e),
            })?;
            Ok(StreamEvent::Output(value))
        }
        "logs" => Ok(StreamEvent::Logs(data.to_string())),
        "completed" => {
            let completed = serde_json::from_str(data).map_err(|e| ReplicateError::Stream {
                message: format!("Failed to parse completed event: {}", e),
            })?;
            Ok(StreamEvent::Completed(completed))
        }
        "error" => {
            let error = serde_json::from_str(data).map_err(|e| ReplicateError::Stream {
                message: format!("Failed to parse error event: {}", e),
            })?;
            Ok(StreamEvent::Error(error))
        }
        _ => {
            warn!(event_type = %event_type, "Unknown event type");
            // Try to parse as generic output
            let value = serde_json::from_str(data).unwrap_or_else(|_| serde_json::json!(data));
            Ok(StreamEvent::Output(value))
        }
    }
}

/// Collect a stream into a single output value.
///
/// This is useful for collecting streaming output into a final result.
///
/// # Example
///
/// ```no_run
/// use replicate::streaming::collect_stream;
/// use replicate::ReplicateClient;
/// use futures::StreamExt;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = ReplicateClient::new("your-api-token")?;
///
/// let prediction = client.predictions()
///     .create(replicate::PredictionRequest::new().model("meta/meta-llama-3-70b-instruct"))
///     .await?;
///
/// let stream = client.streaming().stream_output(&prediction.id).await?;
/// let output = collect_stream(stream).await?;
/// println!("Collected output: {:?}", output);
/// # Ok(())
/// # }
/// ```
pub async fn collect_stream<S>(mut stream: S) -> Result<serde_json::Value>
where
    S: Stream<Item = Result<StreamEvent>> + Unpin,
{
    use serde_json::Value;
    
    let mut outputs: Vec<Value> = Vec::new();
    let mut logs = String::new();
    
    while let Some(event) = stream.next().await {
        match event? {
            StreamEvent::Output(value) => {
                outputs.push(value);
            }
            StreamEvent::Logs(log) => {
                logs.push_str(&log);
                logs.push('\n');
            }
            StreamEvent::Completed(_) => break,
            StreamEvent::Error(e) => {
                return Err(ReplicateError::Stream {
                    message: e.message,
                });
            }
        }
    }
    
    // Combine outputs
    if outputs.is_empty() {
        Ok(Value::Null)
    } else if outputs.len() == 1 {
        Ok(outputs.into_iter().next().unwrap())
    } else {
        Ok(Value::Array(outputs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_output() {
        let text = "event: output\ndata: \"Hello world\"\n\n";
        let event = parse_sse_event(text).unwrap();
        
        match event {
            StreamEvent::Output(value) => {
                assert_eq!(value, serde_json::json!("Hello world"));
            }
            _ => panic!("Expected Output event"),
        }
    }

    #[test]
    fn test_parse_sse_logs() {
        let text = "event: logs\ndata: Processing...\n\n";
        let event = parse_sse_event(text).unwrap();
        
        match event {
            StreamEvent::Logs(logs) => {
                assert_eq!(logs, "Processing...");
            }
            _ => panic!("Expected Logs event"),
        }
    }

    #[test]
    fn test_parse_sse_error() {
        let text = r#"event: error
data: {"message": "Something went wrong"}
"#;
        let event = parse_sse_event(text).unwrap();
        
        match event {
            StreamEvent::Error(error) => {
                assert_eq!(error.message, "Something went wrong");
            }
            _ => panic!("Expected Error event"),
        }
    }

    #[test]
    fn test_parse_sse_missing_data() {
        let text = "event: output\n\n";
        let result = parse_sse_event(text);
        assert!(result.is_err());
    }
}

//! Text completions API for llama.cpp.

use crate::client::LlamaCppClient;
use crate::client::endpoints;
use crate::error::{LlamaCppError, Result};
use crate::types::{CompletionResponse, FinishReason};

/// Client for the text completions API.
#[derive(Debug)]
pub struct Completion<'a> {
    client: &'a LlamaCppClient,
}

impl<'a> Completion<'a> {
    /// Create a new completion client.
    pub fn new(client: &'a LlamaCppClient) -> Self {
        Self { client }
    }

    /// Send a completion request.
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::COMPLETIONS, body).await?;
                    let body = client.handle_response(response).await?;
                    let completion_response: CompletionResponse = serde_json::from_value(body)?;
                    Ok(completion_response)
                })
            })
            .await
    }

    /// Send a streaming completion request.
    #[cfg(feature = "streaming")]
    pub async fn stream(
        &self,
        mut request: CompletionRequest,
    ) -> Result<impl futures::Stream<Item = Result<CompletionChunk>>> {
        use eventsource_stream::Eventsource;
        use futures::StreamExt;

        request.stream = Some(true);
        let body = serde_json::to_value(&request)?;

        let url = format!("{}{}", self.client.base_url(), endpoints::COMPLETIONS);

        tracing::debug!("Initiating streaming completion request");

        let response = self
            .client
            .http()
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(LlamaCppError::from)?;

        let status = response.status();
        if !status.is_success() {
            return Err(LlamaCppError::from_response(response).await);
        }

        let stream = response
            .bytes_stream()
            .eventsource()
            .map(|event| match event {
                Ok(event) => {
                    if event.data == "[DONE]" {
                        return Ok(CompletionChunk::done());
                    }

                    match serde_json::from_str::<CompletionChunk>(&event.data) {
                        Ok(chunk) => Ok(chunk),
                        Err(e) => Err(LlamaCppError::Stream {
                            message: format!("Failed to parse SSE event: {e}"),
                        }),
                    }
                }
                Err(e) => Err(LlamaCppError::Stream {
                    message: format!("SSE error: {e}"),
                }),
            });

        Ok(stream)
    }
}

/// A text completion request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompletionRequest {
    /// The prompt to complete.
    pub prompt: String,
    /// Maximum number of tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    /// Temperature for sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Nucleus sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    /// Minimum probability for sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_p: Option<f32>,
    /// Whether to stream back partial progress.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// Presence penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    /// Frequency penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    /// Number of completions to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<usize>,
    /// Random seed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// Grammar (JSON schema or BNF grammar).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grammar: Option<String>,
    /// JSON mode - force JSON output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_mode: Option<bool>,
    /// Cache the prompt for faster subsequent requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_prompt: Option<bool>,
    /// Slot ID to use (-1 for automatic assignment).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot_id: Option<i32>,
    /// Echo back the prompt in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub echo: Option<bool>,
    /// Return log probabilities of output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<i32>,
    /// Suffix to append to the prompt (for fill-in-the-middle).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
}

impl CompletionRequest {
    /// Create a new completion request builder.
    pub fn builder(prompt: impl Into<String>) -> CompletionRequestBuilder {
        CompletionRequestBuilder::new(prompt)
    }

    /// Create a simple completion request.
    pub fn simple(prompt: impl Into<String>) -> Self {
        Self::builder(prompt).build()
    }
}

/// Builder for completion requests.
#[derive(Debug, Clone)]
pub struct CompletionRequestBuilder {
    prompt: String,
    max_tokens: Option<usize>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    top_k: Option<i32>,
    min_p: Option<f32>,
    stream: Option<bool>,
    stop: Option<Vec<String>>,
    presence_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
    n: Option<usize>,
    seed: Option<i64>,
    grammar: Option<String>,
    json_mode: Option<bool>,
    cache_prompt: Option<bool>,
    slot_id: Option<i32>,
    echo: Option<bool>,
    logprobs: Option<i32>,
    suffix: Option<String>,
}

impl CompletionRequestBuilder {
    /// Create a new builder.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            min_p: None,
            stream: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            n: None,
            seed: None,
            grammar: None,
            json_mode: None,
            cache_prompt: None,
            slot_id: None,
            echo: None,
            logprobs: None,
            suffix: None,
        }
    }

    /// Set max tokens.
    pub fn max_tokens(mut self, tokens: usize) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Set temperature.
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp.clamp(0.0, 2.0));
        self
    }

    /// Set top-p.
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p.clamp(0.0, 1.0));
        self
    }

    /// Set top-k.
    pub fn top_k(mut self, top_k: i32) -> Self {
        self.top_k = Some(top_k.max(1));
        self
    }

    /// Set min-p.
    pub fn min_p(mut self, min_p: f32) -> Self {
        self.min_p = Some(min_p.clamp(0.0, 1.0));
        self
    }

    /// Enable streaming.
    pub fn stream(mut self, enabled: bool) -> Self {
        self.stream = Some(enabled);
        self
    }

    /// Add a stop sequence.
    pub fn stop_sequence(mut self, seq: impl Into<String>) -> Self {
        self.stop.get_or_insert_with(Vec::new).push(seq.into());
        self
    }

    /// Set stop sequences.
    pub fn stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    /// Set presence penalty.
    pub fn presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty.clamp(-2.0, 2.0));
        self
    }

    /// Set frequency penalty.
    pub fn frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty.clamp(-2.0, 2.0));
        self
    }

    /// Set number of completions.
    pub fn n(mut self, n: usize) -> Self {
        self.n = Some(n);
        self
    }

    /// Set random seed.
    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set grammar (JSON schema or BNF grammar).
    pub fn grammar(mut self, grammar: impl Into<String>) -> Self {
        self.grammar = Some(grammar.into());
        self
    }

    /// Enable JSON mode.
    pub fn json_mode(mut self, enabled: bool) -> Self {
        self.json_mode = Some(enabled);
        self
    }

    /// Enable prompt caching.
    pub fn cache_prompt(mut self, enabled: bool) -> Self {
        self.cache_prompt = Some(enabled);
        self
    }

    /// Set specific slot ID.
    pub fn slot_id(mut self, slot_id: i32) -> Self {
        self.slot_id = Some(slot_id);
        self
    }

    /// Enable echo.
    pub fn echo(mut self, enabled: bool) -> Self {
        self.echo = Some(enabled);
        self
    }

    /// Set logprobs.
    pub fn logprobs(mut self, logprobs: i32) -> Self {
        self.logprobs = Some(logprobs.max(0));
        self
    }

    /// Set suffix (for fill-in-the-middle).
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Build the request.
    pub fn build(self) -> CompletionRequest {
        CompletionRequest {
            prompt: self.prompt,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            min_p: self.min_p,
            stream: self.stream,
            stop: self.stop,
            presence_penalty: self.presence_penalty,
            frequency_penalty: self.frequency_penalty,
            n: self.n,
            seed: self.seed,
            grammar: self.grammar,
            json_mode: self.json_mode,
            cache_prompt: self.cache_prompt,
            slot_id: self.slot_id,
            echo: self.echo,
            logprobs: self.logprobs,
            suffix: self.suffix,
        }
    }
}

/// A completion chunk in a stream.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompletionChunk {
    /// Unique identifier for the chunk.
    #[serde(rename = "id")]
    pub id: String,
    /// The object type.
    pub object: String,
    /// The Unix timestamp.
    pub created: i64,
    /// The model used.
    pub model: String,
    /// The list of choices.
    pub choices: Vec<CompletionStreamChoice>,
}

impl CompletionChunk {
    /// Check if this is the final chunk.
    pub fn is_done(&self) -> bool {
        self.choices.is_empty()
            || self
                .choices
                .iter()
                .all(|c| c.text.is_empty() && c.finish_reason.is_some())
    }

    /// Create a done chunk.
    pub fn done() -> Self {
        Self {
            id: String::new(),
            object: "text_completion.chunk".to_string(),
            created: 0,
            model: String::new(),
            choices: vec![],
        }
    }

    /// Get the text from the first choice.
    pub fn text(&self) -> Option<&str> {
        self.choices.first().map(|c| c.text.as_str())
    }

    /// Check if the first choice has finished.
    pub fn finish_reason(&self) -> Option<FinishReason> {
        self.choices.first().and_then(|c| c.finish_reason)
    }
}

/// A choice in a streaming completion chunk.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompletionStreamChoice {
    /// The index of this choice.
    pub index: usize,
    /// The generated text delta.
    pub text: String,
    /// Logprobs (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
    /// The reason the completion finished.
    pub finish_reason: Option<FinishReason>,
}

/// A collector that accumulates streaming completion chunks into a complete response.
#[derive(Debug, Default)]
pub struct StreamCollector {
    completion_id: Option<String>,
    model: Option<String>,
    text: String,
    finish_reason: Option<FinishReason>,
}

impl StreamCollector {
    /// Create a new stream collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a chunk.
    pub fn process_chunk(&mut self, chunk: &CompletionChunk) {
        if !chunk.id.is_empty() {
            self.completion_id = Some(chunk.id.clone());
        }
        if !chunk.model.is_empty() {
            self.model = Some(chunk.model.clone());
        }

        if let Some(choice) = chunk.choices.first() {
            self.text.push_str(&choice.text);

            if let Some(reason) = choice.finish_reason {
                self.finish_reason = Some(reason);
            }
        }
    }

    /// Get the collected text so far.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Check if the stream has finished.
    pub fn is_complete(&self) -> bool {
        self.finish_reason.is_some()
    }

    /// Get the finish reason.
    pub fn finish_reason(&self) -> Option<FinishReason> {
        self.finish_reason
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let request = CompletionRequest::builder("Once upon a time")
            .max_tokens(1024)
            .temperature(0.7)
            .build();

        assert_eq!(request.prompt, "Once upon a time");
        assert_eq!(request.max_tokens, Some(1024));
        assert_eq!(request.temperature, Some(0.7));
    }

    #[test]
    fn test_simple_request() {
        let request = CompletionRequest::simple("Hello world");
        assert_eq!(request.prompt, "Hello world");
    }

    #[test]
    fn test_fill_in_middle() {
        let request = CompletionRequest::builder("def fibonacci(n):")
            .suffix("return result")
            .max_tokens(256)
            .build();

        assert_eq!(request.prompt, "def fibonacci(n):");
        assert_eq!(request.suffix, Some("return result".to_string()));
    }

    #[test]
    fn test_echo() {
        let request = CompletionRequest::builder("Hello").echo(true).build();

        assert_eq!(request.echo, Some(true));
    }

    #[test]
    fn test_logprobs() {
        let request = CompletionRequest::builder("Hello").logprobs(5).build();

        assert_eq!(request.logprobs, Some(5));
    }

    #[test]
    fn test_stream_collector() {
        let mut collector = StreamCollector::new();

        let chunk1 = CompletionChunk {
            id: "cmpl-123".to_string(),
            object: "text_completion.chunk".to_string(),
            created: 1234567890,
            model: "llama-3-8b".to_string(),
            choices: vec![CompletionStreamChoice {
                index: 0,
                text: "Hello".to_string(),
                logprobs: None,
                finish_reason: None,
            }],
        };
        collector.process_chunk(&chunk1);

        let chunk2 = CompletionChunk {
            id: "cmpl-123".to_string(),
            object: "text_completion.chunk".to_string(),
            created: 1234567890,
            model: "llama-3-8b".to_string(),
            choices: vec![CompletionStreamChoice {
                index: 0,
                text: " world!".to_string(),
                logprobs: None,
                finish_reason: Some(FinishReason::Stop),
            }],
        };
        collector.process_chunk(&chunk2);

        assert_eq!(collector.text(), "Hello world!");
        assert!(collector.is_complete());
    }
}

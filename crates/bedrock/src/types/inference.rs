//! Inference configuration types.

use serde::{Deserialize, Serialize};

/// Inference configuration for model requests.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InferenceConfiguration {
    /// Maximum number of tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    /// Temperature for sampling (0.0 - 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling (0.0 - 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

impl InferenceConfiguration {
    /// Create new inference configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum tokens.
    pub fn max_tokens(mut self, max_tokens: i32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set temperature.
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature.clamp(0.0, 1.0));
        self
    }

    /// Set top-p.
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p.clamp(0.0, 1.0));
        self
    }

    /// Add a stop sequence.
    pub fn stop_sequence(mut self, sequence: impl Into<String>) -> Self {
        self.stop_sequences
            .get_or_insert_with(Vec::new)
            .push(sequence.into());
        self
    }

    /// Set stop sequences.
    pub fn stop_sequences(mut self, sequences: Vec<String>) -> Self {
        self.stop_sequences = Some(sequences);
        self
    }
}

/// Performance configuration for latency optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PerformanceConfiguration {
    /// Optimized for latency.
    LatencyOptimized,
    /// Standard performance.
    Standard,
}

impl PerformanceConfiguration {
    /// Get the configuration as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            PerformanceConfiguration::LatencyOptimized => "latencyOptimized",
            PerformanceConfiguration::Standard => "standard",
        }
    }
}

/// System prompt content block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemContentBlock {
    /// The text content.
    pub text: String,
    /// Guardrail configuration for the system prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guardrail_config: Option<GuardrailConfiguration>,
}

impl SystemContentBlock {
    /// Create a new system content block.
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            text: content.into(),
            guardrail_config: None,
        }
    }

    /// Set guardrail configuration.
    pub fn with_guardrail(mut self, config: GuardrailConfiguration) -> Self {
        self.guardrail_config = Some(config);
        self
    }
}

/// Guardrail configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailConfiguration {
    /// The guardrail identifier.
    pub guardrail_identifier: String,
    /// The guardrail version.
    pub guardrail_version: String,
    /// Whether to enable trace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<Trace>,
    /// Whether to stream the trace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_trace: Option<bool>,
}

impl GuardrailConfiguration {
    /// Create new guardrail configuration.
    pub fn new(identifier: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            guardrail_identifier: identifier.into(),
            guardrail_version: version.into(),
            trace: None,
            stream_trace: None,
        }
    }

    /// Enable trace.
    pub fn with_trace(mut self, trace: Trace) -> Self {
        self.trace = Some(trace);
        self
    }

    /// Enable streaming trace.
    pub fn with_stream_trace(mut self) -> Self {
        self.stream_trace = Some(true);
        self
    }
}

/// Trace level for guardrails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Trace {
    /// Enabled trace.
    Enabled,
    /// Disabled trace.
    Disabled,
}

/// Prompt variable for dynamic prompts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptVariable {
    /// The variable name.
    pub name: String,
    /// The variable value.
    pub value: serde_json::Value,
}

impl PromptVariable {
    /// Create a new prompt variable.
    pub fn new(name: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Create a string variable.
    pub fn string(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(name, value.into())
    }

    /// Create a number variable.
    pub fn number(name: impl Into<String>, value: impl Into<f64>) -> Self {
        Self::new(name, serde_json::Value::from(value.into()))
    }

    /// Create a boolean variable.
    pub fn boolean(name: impl Into<String>, value: bool) -> Self {
        Self::new(name, value)
    }
}

/// Request metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RequestMetadata {
    /// Trace ID for the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

impl RequestMetadata {
    /// Create new request metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the trace ID.
    pub fn trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inference_configuration() {
        let config = InferenceConfiguration::new()
            .max_tokens(1000)
            .temperature(0.7)
            .top_p(0.9)
            .stop_sequence("END");

        assert_eq!(config.max_tokens, Some(1000));
        assert_eq!(config.temperature, Some(0.7));
        assert_eq!(config.top_p, Some(0.9));
        assert_eq!(config.stop_sequences, Some(vec!["END".to_string()]));
    }

    #[test]
    fn test_inference_configuration_clamping() {
        let config = InferenceConfiguration::new()
            .temperature(1.5)
            .top_p(-0.5);

        assert_eq!(config.temperature, Some(1.0));
        assert_eq!(config.top_p, Some(0.0));
    }

    #[test]
    fn test_guardrail_configuration() {
        let config = GuardrailConfiguration::new("guardrail-123", "1")
            .with_trace(Trace::Enabled)
            .with_stream_trace();

        assert_eq!(config.guardrail_identifier, "guardrail-123");
        assert_eq!(config.guardrail_version, "1");
        assert_eq!(config.trace, Some(Trace::Enabled));
        assert_eq!(config.stream_trace, Some(true));
    }

    #[test]
    fn test_system_content_block() {
        let block = SystemContentBlock::text("You are a helpful assistant.");
        assert_eq!(block.text, "You are a helpful assistant.");
        assert!(block.guardrail_config.is_none());
    }

    #[test]
    fn test_prompt_variable() {
        let var = PromptVariable::string("name", "Alice");
        assert_eq!(var.name, "name");
        assert_eq!(var.value, "Alice");

        let var = PromptVariable::number("age", 30.0);
        assert_eq!(var.value, 30.0);

        let var = PromptVariable::boolean("active", true);
        assert_eq!(var.value, true);
    }

    #[test]
    fn test_request_metadata() {
        let metadata = RequestMetadata::new().trace_id("trace-123");
        assert_eq!(metadata.trace_id, Some("trace-123".to_string()));
    }
}

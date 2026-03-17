//! Type definitions for the Ollama API.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The role of a message author.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System message.
    System,
    /// User message.
    User,
    /// Assistant message.
    Assistant,
    /// Tool message.
    Tool,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::Tool => write!(f, "tool"),
        }
    }
}

/// A chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// The role of the message author.
    pub role: Role,
    /// The content of the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Tool calls made by the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Images for multi-modal inputs (base64 encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
}

impl ChatMessage {
    /// Create a new message with the given role and content.
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: Some(content.into()),
            tool_calls: None,
            images: None,
        }
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content)
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }

    /// Create a user message with images for vision models.
    pub fn user_with_images(
        content: impl Into<String>,
        images: Vec<ImageInput>,
    ) -> crate::error::Result<Self> {
        let encoded_images: Vec<String> = images
            .into_iter()
            .map(|img| img.to_base64())
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            role: Role::User,
            content: Some(content.into()),
            tool_calls: None,
            images: Some(encoded_images),
        })
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }

    /// Create a tool message.
    pub fn tool(content: impl Into<String>) -> Self {
        Self::new(Role::Tool, content)
    }

    /// Create an assistant message with tool calls.
    pub fn assistant_with_tools(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: Role::Assistant,
            content: Some(content.into()),
            tool_calls: Some(tool_calls),
            images: None,
        }
    }
}

/// Image input for multi-modal models.
#[derive(Debug, Clone)]
pub enum ImageInput {
    /// Raw bytes of the image.
    Bytes(Vec<u8>),
    /// Path to the image file.
    Path(std::path::PathBuf),
    /// Already base64-encoded string.
    Base64(String),
}

impl ImageInput {
    /// Create an image input from a file path.
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Self {
        Self::Path(path.as_ref().to_path_buf())
    }

    /// Create an image input from raw bytes.
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self::Bytes(bytes.into())
    }

    /// Create an image input from a base64-encoded string.
    pub fn from_base64(encoded: impl Into<String>) -> Self {
        Self::Base64(encoded.into())
    }

    /// Convert the image to base64 encoding.
    pub fn to_base64(&self) -> crate::error::Result<String> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        
        match self {
            Self::Base64(s) => Ok(s.clone()),
            Self::Bytes(bytes) => Ok(STANDARD.encode(bytes)),
            Self::Path(path) => {
                let bytes = std::fs::read(path)?;
                Ok(STANDARD.encode(bytes))
            }
        }
    }

    /// Load the image asynchronously and convert to base64.
    pub async fn to_base64_async(&self) -> crate::error::Result<String> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        
        match self {
            Self::Base64(s) => Ok(s.clone()),
            Self::Bytes(bytes) => Ok(STANDARD.encode(bytes)),
            Self::Path(path) => {
                let bytes = tokio::fs::read(path).await?;
                Ok(STANDARD.encode(bytes))
            }
        }
    }
}

/// A tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// The function to call.
    pub function: ToolCallFunction,
}

/// A tool call function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    /// The name of the function to call.
    pub name: String,
    /// The arguments to pass to the function (as a JSON object).
    pub arguments: serde_json::Value,
}

/// A tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// The type of tool (always "function").
    #[serde(rename = "type")]
    pub tool_type: String,
    /// The function definition.
    pub function: Function,
}

impl Tool {
    /// Create a new tool from a function definition.
    pub fn function(function: Function) -> Self {
        Self {
            tool_type: "function".to_string(),
            function,
        }
    }
}

/// A function definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    /// The name of the function.
    pub name: String,
    /// A description of what the function does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The parameters the function accepts (JSON Schema).
    pub parameters: serde_json::Value,
}

impl Function {
    /// Create a new function definition.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: Some(description.into()),
            parameters: serde_json::json!({"type": "object"}),
        }
    }

    /// Set the parameters schema.
    pub fn parameters(mut self, params: serde_json::Value) -> Self {
        self.parameters = params;
        self
    }

    /// Builder for function parameters.
    pub fn builder(name: impl Into<String>, description: impl Into<String>) -> FunctionBuilder {
        FunctionBuilder::new(name, description)
    }
}

/// Builder for function definitions.
#[derive(Debug, Clone)]
pub struct FunctionBuilder {
    name: String,
    description: String,
    properties: Vec<(&'static str, serde_json::Value)>,
    required: Vec<&'static str>,
}

impl FunctionBuilder {
    /// Create a new function builder.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            properties: Vec::new(),
            required: Vec::new(),
        }
    }

    /// Add a string property.
    pub fn string_property(
        mut self,
        name: &'static str,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let prop = serde_json::json!({
            "type": "string",
            "description": description.into(),
        });
        self.properties.push((name, prop));
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add an integer property.
    pub fn integer_property(
        mut self,
        name: &'static str,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let prop = serde_json::json!({
            "type": "integer",
            "description": description.into(),
        });
        self.properties.push((name, prop));
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add a number property.
    pub fn number_property(
        mut self,
        name: &'static str,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let prop = serde_json::json!({
            "type": "number",
            "description": description.into(),
        });
        self.properties.push((name, prop));
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add a boolean property.
    pub fn boolean_property(
        mut self,
        name: &'static str,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let prop = serde_json::json!({
            "type": "boolean",
            "description": description.into(),
        });
        self.properties.push((name, prop));
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add an enum property.
    pub fn enum_property(
        mut self,
        name: &'static str,
        description: impl Into<String>,
        variants: Vec<&'static str>,
        required: bool,
    ) -> Self {
        let prop = serde_json::json!({
            "type": "string",
            "description": description.into(),
            "enum": variants,
        });
        self.properties.push((name, prop));
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add an array property.
    pub fn array_property(
        mut self,
        name: &'static str,
        description: impl Into<String>,
        item_type: serde_json::Value,
        required: bool,
    ) -> Self {
        let prop = serde_json::json!({
            "type": "array",
            "description": description.into(),
            "items": item_type,
        });
        self.properties.push((name, prop));
        if required {
            self.required.push(name);
        }
        self
    }

    /// Build the function.
    pub fn build(self) -> Function {
        let properties: serde_json::Map<String, serde_json::Value> = self
            .properties
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();

        let mut params = serde_json::json!({
            "type": "object",
            "properties": properties,
        });

        if !self.required.is_empty() {
            params["required"] = serde_json::json!(self.required);
        }

        Function {
            name: self.name,
            description: Some(self.description),
            parameters: params,
        }
    }
}

/// Response format for structured outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FormatType {
    /// Standard text or JSON output.
    Type(String),
    /// JSON schema validation (Ollama 0.5.0+).
    JsonSchema {
        /// The type of format - should be "json" with a schema.
        #[serde(rename = "type")]
        format_type: String,
        /// The JSON schema.
        schema: serde_json::Value,
    },
}

impl FormatType {
    /// Create a JSON format type.
    pub fn json() -> Self {
        Self::Type("json".to_string())
    }

    /// Create a text format type.
    pub fn text() -> Self {
        Self::Type("text".to_string())
    }

    /// Create a JSON schema format type.
    pub fn json_schema(schema: serde_json::Value) -> Self {
        Self::JsonSchema {
            format_type: "json".to_string(),
            schema,
        }
    }
}

/// Keep-alive configuration for controlling model loading.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KeepAlive {
    /// Duration in seconds (negative values keep loaded indefinitely).
    Seconds(i64),
    /// Duration string (e.g., "5m", "1h", "30s").
    String(String),
}

impl KeepAlive {
    /// Keep the model loaded for a specific number of seconds.
    pub fn seconds(s: i64) -> Self {
        Self::Seconds(s)
    }

    /// Keep the model loaded indefinitely.
    pub fn indefinitely() -> Self {
        Self::Seconds(-1)
    }

    /// Keep the model loaded for the specified duration string.
    pub fn duration(s: impl Into<String>) -> Self {
        Self::String(s.into())
    }

    /// Unload immediately after the request.
    pub fn unload_immediately() -> Self {
        Self::Seconds(0)
    }
}

/// Inference options for controlling generation parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Options {
    /// Temperature for sampling (0.0 - 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Nucleus sampling parameter (0.0 - 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    /// Maximum number of tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<i32>,
    /// Seed for deterministic generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i32>,
    /// Penalty for repeating tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_penalty: Option<f32>,
    /// Frequency penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    /// Presence penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// Number of tokens to keep from the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_keep: Option<i32>,
    /// Batch size for prompt processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_batch: Option<i32>,
    /// Number of GPUs to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_gpu: Option<i32>,
    /// Number of threads to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_thread: Option<i32>,
    /// Mirostat mode (0 = disabled, 1 = mirostat, 2 = mirostat 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirostat: Option<i32>,
    /// Mirostat learning rate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirostat_eta: Option<f32>,
    /// Mirostat tau.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirostat_tau: Option<f32>,
    /// Penalize newlines.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub penalize_newline: Option<bool>,
    /// Additional options.
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

impl Options {
    /// Create a new options builder.
    pub fn builder() -> OptionsBuilder {
        OptionsBuilder::default()
    }
}

/// Builder for inference options.
#[derive(Debug, Clone, Default)]
pub struct OptionsBuilder {
    temperature: Option<f32>,
    top_p: Option<f32>,
    top_k: Option<i32>,
    num_predict: Option<i32>,
    seed: Option<i32>,
    repeat_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    stop: Option<Vec<String>>,
    num_keep: Option<i32>,
    num_batch: Option<i32>,
    num_gpu: Option<i32>,
    num_thread: Option<i32>,
    mirostat: Option<i32>,
    mirostat_eta: Option<f32>,
    mirostat_tau: Option<f32>,
    penalize_newline: Option<bool>,
    additional: HashMap<String, serde_json::Value>,
}

impl OptionsBuilder {
    /// Set the temperature.
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp.clamp(0.0, 2.0));
        self
    }

    /// Set the top-p value.
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p.clamp(0.0, 1.0));
        self
    }

    /// Set the top-k value.
    pub fn top_k(mut self, top_k: i32) -> Self {
        self.top_k = Some(top_k);
        self
    }

    /// Set the maximum number of tokens to predict.
    pub fn num_predict(mut self, n: i32) -> Self {
        self.num_predict = Some(n);
        self
    }

    /// Set the random seed.
    pub fn seed(mut self, seed: i32) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set the repeat penalty.
    pub fn repeat_penalty(mut self, penalty: f32) -> Self {
        self.repeat_penalty = Some(penalty);
        self
    }

    /// Set the frequency penalty.
    pub fn frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    /// Set the presence penalty.
    pub fn presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty);
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

    /// Set the number of threads.
    pub fn num_thread(mut self, n: i32) -> Self {
        self.num_thread = Some(n);
        self
    }

    /// Set mirostat mode.
    pub fn mirostat(mut self, mode: i32) -> Self {
        self.mirostat = Some(mode);
        self
    }

    /// Add an additional option.
    pub fn additional(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.additional.insert(key.into(), value.into());
        self
    }

    /// Build the options.
    pub fn build(self) -> Options {
        Options {
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            num_predict: self.num_predict,
            seed: self.seed,
            repeat_penalty: self.repeat_penalty,
            frequency_penalty: self.frequency_penalty,
            presence_penalty: self.presence_penalty,
            stop: self.stop,
            num_keep: self.num_keep,
            num_batch: self.num_batch,
            num_gpu: self.num_gpu,
            num_thread: self.num_thread,
            mirostat: self.mirostat,
            mirostat_eta: self.mirostat_eta,
            mirostat_tau: self.mirostat_tau,
            penalize_newline: self.penalize_newline,
            additional: self.additional,
        }
    }
}

/// A chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// The model used for completion.
    pub model: String,
    /// The creation timestamp.
    pub created_at: String,
    /// The response message.
    pub message: ChatMessage,
    /// Whether this is the final response.
    pub done: bool,
    /// Total duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<u64>,
    /// Load duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_duration: Option<u64>,
    /// Prompt evaluation count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_count: Option<u64>,
    /// Prompt evaluation duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_duration: Option<u64>,
    /// Evaluation count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_count: Option<u64>,
    /// Evaluation duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_duration: Option<u64>,
}

impl ChatResponse {
    /// Get the content of the response.
    pub fn content(&self) -> &str {
        self.message.content.as_deref().unwrap_or("")
    }

    /// Check if the response contains tool calls.
    pub fn has_tool_calls(&self) -> bool {
        self.message
            .tool_calls
            .as_ref()
            .map(|t| !t.is_empty())
            .unwrap_or(false)
    }

    /// Get tool calls from the response.
    pub fn tool_calls(&self) -> Option<&[ToolCall]> {
        self.message.tool_calls.as_deref()
    }

    /// Get the number of prompt tokens.
    pub fn prompt_tokens(&self) -> u64 {
        self.prompt_eval_count.unwrap_or(0)
    }

    /// Get the number of completion tokens.
    pub fn completion_tokens(&self) -> u64 {
        self.eval_count.unwrap_or(0)
    }
}

/// A generate completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateResponse {
    /// The model used for completion.
    pub model: String,
    /// The creation timestamp.
    pub created_at: String,
    /// The generated response.
    pub response: String,
    /// Whether this is the final response.
    pub done: bool,
    /// Context for subsequent generate requests (deprecated in favor of messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<u64>>,
    /// Total duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<u64>,
    /// Load duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_duration: Option<u64>,
    /// Prompt evaluation count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_count: Option<u64>,
    /// Prompt evaluation duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_duration: Option<u64>,
    /// Evaluation count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_count: Option<u64>,
    /// Evaluation duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_duration: Option<u64>,
}

/// Model information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// The model name.
    pub name: String,
    /// The model identifier (same as name).
    pub model: String,
    /// Size of the model in bytes.
    pub size: u64,
    /// Size in a human-readable format.
    #[serde(rename = "size_vram")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_vram: Option<u64>,
    /// Digest/hash of the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// Modification date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<String>,
    /// Model details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<ModelDetails>,
}

/// Model details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDetails {
    /// The model architecture/format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// The model family.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    /// Model families (for MoE models).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub families: Option<Vec<String>>,
    /// Parameter size (e.g., "7B").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_size: Option<String>,
    /// Quantization level (e.g., "Q4_0").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantization_level: Option<String>,
}

/// Response from listing models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListModelsResponse {
    /// The list of models.
    pub models: Vec<ModelInfo>,
}

/// Pull status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullStatus {
    /// Status message.
    pub status: String,
    /// Digest being pulled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// Total size to download.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    /// Bytes completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<u64>,
}

impl PullStatus {
    /// Check if the pull is complete.
    pub fn is_complete(&self) -> bool {
        self.status.contains("success")
    }

    /// Get the progress as a percentage (0.0 - 1.0).
    pub fn progress(&self) -> Option<f64> {
        match (self.total, self.completed) {
            (Some(total), Some(completed)) if total > 0 => Some(completed as f64 / total as f64),
            _ => None,
        }
    }
}

/// Push status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushStatus {
    /// Status message.
    pub status: String,
    /// Digest being pushed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// Total size to upload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    /// Bytes completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<u64>,
}

impl PushStatus {
    /// Check if the push is complete.
    pub fn is_complete(&self) -> bool {
        self.status.contains("success")
    }

    /// Get the progress as a percentage (0.0 - 1.0).
    pub fn progress(&self) -> Option<f64> {
        match (self.total, self.completed) {
            (Some(total), Some(completed)) if total > 0 => Some(completed as f64 / total as f64),
            _ => None,
        }
    }
}

/// Generic progress status for create/copy operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressStatus {
    /// Status message.
    pub status: String,
}

impl ProgressStatus {
    /// Check if the operation is complete.
    pub fn is_complete(&self) -> bool {
        self.status.contains("success")
    }
}

/// Create model request.
#[derive(Debug, Clone, Serialize)]
pub struct CreateModelRequest {
    /// The name for the new model.
    pub model: String,
    /// The Modelfile content.
    pub modelfile: String,
    /// Path to the Modelfile (alternative to modelfile content).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Quantize the model to this level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantize: Option<String>,
    /// Stream the progress.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// Create model status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateModelStatus {
    /// Status message.
    pub status: String,
}

impl CreateModelStatus {
    /// Check if the create operation is complete.
    pub fn is_complete(&self) -> bool {
        self.status.contains("success")
    }
}

/// Copy model request.
#[derive(Debug, Clone, Serialize)]
pub struct CopyModelRequest {
    /// Source model name.
    pub source: String,
    /// Destination model name.
    pub destination: String,
}

/// Delete model request.
#[derive(Debug, Clone, Serialize)]
pub struct DeleteModelRequest {
    /// Model name to delete.
    pub model: String,
}

/// Show model request.
#[derive(Debug, Clone, Serialize)]
pub struct ShowModelRequest {
    /// Model name.
    pub model: String,
}

/// Show model response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowModelResponse {
    /// License text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Modelfile content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modelfile: Option<String>,
    /// Parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<String>,
    /// Template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// System prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<ModelDetails>,
    /// Messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<ChatMessage>>,
}

/// Embedding response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    /// The embedding vector.
    pub embedding: Vec<f32>,
}

/// Running model information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningModel {
    /// The model name.
    pub name: String,
    /// The model identifier.
    pub model: String,
    /// Size of the model in bytes.
    pub size: u64,
    /// Size in VRAM.
    #[serde(rename = "size_vram")]
    pub size_vram: u64,
    /// Digest.
    pub digest: String,
    /// Details.
    pub details: ModelDetails,
    /// Expiration time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// Response from listing running models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningModelsResponse {
    /// The list of running models.
    pub models: Vec<RunningModel>,
}

/// Version response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionResponse {
    /// Ollama version.
    pub version: String,
}

/// A function call (for tool results).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// The name of the function that was called.
    pub name: String,
    /// The arguments passed to the function (JSON string).
    pub arguments: String,
}

/// Tool result for sending back to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The name of the tool/function.
    pub name: String,
    /// The result/output of the tool call.
    pub content: String,
}

/// Message role for compatibility.
pub type MessageRole = Role;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = ChatMessage::user("Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, Some("Hello".to_string()));
    }

    #[test]
    fn test_function_builder() {
        let func = Function::builder("get_weather", "Get weather")
            .string_property("location", "City name", true)
            .enum_property("unit", "Temperature unit", vec!["celsius", "fahrenheit"], false)
            .build();

        assert_eq!(func.name, "get_weather");
        assert!(func.parameters.get("required").is_some());
    }

    #[test]
    fn test_keep_alive() {
        let ka = KeepAlive::seconds(300);
        assert!(matches!(ka, KeepAlive::Seconds(300)));

        let ka = KeepAlive::indefinitely();
        assert!(matches!(ka, KeepAlive::Seconds(-1)));
    }

    #[test]
    fn test_options_builder() {
        let opts = Options::builder()
            .temperature(0.7)
            .num_predict(1024)
            .seed(42)
            .build();

        assert_eq!(opts.temperature, Some(0.7));
        assert_eq!(opts.num_predict, Some(1024));
        assert_eq!(opts.seed, Some(42));
    }

    #[test]
    fn test_format_type() {
        let json = FormatType::json();
        assert!(matches!(json, FormatType::Type(s) if s == "json"));

        let text = FormatType::text();
        assert!(matches!(text, FormatType::Type(s) if s == "text"));
    }

    #[test]
    fn test_image_input_base64() {
        let encoded = "aGVsbG8=";
        let img = ImageInput::from_base64(encoded);
        assert_eq!(img.to_base64().unwrap(), encoded);
    }

    #[test]
    fn test_pull_status_progress() {
        let status = PullStatus {
            status: "pulling".to_string(),
            digest: None,
            total: Some(100),
            completed: Some(50),
        };
        assert_eq!(status.progress(), Some(0.5));
        assert!(!status.is_complete());
    }

    #[test]
    fn test_chat_response_helpers() {
        let response = ChatResponse {
            model: "llama3.2".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            message: ChatMessage::assistant("Hello!"),
            done: true,
            total_duration: None,
            load_duration: None,
            prompt_eval_count: Some(10),
            prompt_eval_duration: None,
            eval_count: Some(5),
            eval_duration: None,
        };

        assert_eq!(response.content(), "Hello!");
        assert_eq!(response.prompt_tokens(), 10);
        assert_eq!(response.completion_tokens(), 5);
    }
}

//! Classify API for Cohere's classification models.

use serde::{Deserialize, Serialize};

use crate::client::CohereClient;
use crate::constants::endpoints;
use crate::error::Result;
use crate::types::ApiMeta;

/// Client for the Classify API.
#[derive(Debug)]
pub struct ClassifyEndpoint<'a> {
    pub(crate) client: &'a CohereClient,
}

impl<'a> ClassifyEndpoint<'a> {
    /// Send a classify request.
    pub async fn create(&self, request: ClassifyRequest) -> Result<ClassifyResponse> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::CLASSIFY, body).await?;
                    let body = client.handle_response(response).await?;
                    let classify_response: ClassifyResponse = serde_json::from_value(body)?;
                    Ok(classify_response)
                })
            })
            .await
    }
}

/// A request to the Cohere Classify API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyRequest {
    /// The texts to classify.
    pub inputs: Vec<String>,

    /// The model to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Examples for few-shot classification (name format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<Example>>,

    /// Label IDs for zero-shot classification (v3 models).
    #[serde(rename = "label_ids", skip_serializing_if = "Option::is_none")]
    pub label_ids: Option<Vec<String>>,

    /// Preset for classification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,

    /// Truncate the input if it's too long.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncate: Option<TruncateMode>,
}

impl ClassifyRequest {
    /// Create a new request builder.
    pub fn builder() -> ClassifyRequestBuilder {
        ClassifyRequestBuilder::new()
    }

    /// Create a few-shot classification request.
    pub fn few_shot(inputs: Vec<String>, examples: Vec<Example>) -> Self {
        Self::builder().inputs(inputs).examples(examples).build()
    }

    /// Create a zero-shot classification request (v3 models).
    pub fn zero_shot(model: impl Into<String>, inputs: Vec<String>, labels: Vec<String>) -> Self {
        Self::builder()
            .model(model)
            .inputs(inputs)
            .label_ids(labels)
            .build()
    }
}

/// Builder for classify requests.
#[derive(Debug, Clone)]
pub struct ClassifyRequestBuilder {
    inputs: Vec<String>,
    model: Option<String>,
    examples: Option<Vec<Example>>,
    label_ids: Option<Vec<String>>,
    preset: Option<String>,
    truncate: Option<TruncateMode>,
}

impl ClassifyRequestBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            model: None,
            examples: None,
            label_ids: None,
            preset: None,
            truncate: None,
        }
    }

    /// Set the model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Add an input text.
    pub fn add_input(mut self, input: impl Into<String>) -> Self {
        self.inputs.push(input.into());
        self
    }

    /// Set the input texts.
    pub fn inputs(mut self, inputs: Vec<String>) -> Self {
        self.inputs = inputs;
        self
    }

    /// Add an example.
    pub fn add_example(mut self, text: impl Into<String>, label: impl Into<String>) -> Self {
        self.examples
            .get_or_insert_with(Vec::new)
            .push(Example::new(text, label));
        self
    }

    /// Set the examples.
    pub fn examples(mut self, examples: Vec<Example>) -> Self {
        self.examples = Some(examples);
        self
    }

    /// Add a label ID (for zero-shot).
    pub fn add_label(mut self, label: impl Into<String>) -> Self {
        self.label_ids
            .get_or_insert_with(Vec::new)
            .push(label.into());
        self
    }

    /// Set the label IDs (for zero-shot).
    pub fn label_ids(mut self, labels: Vec<String>) -> Self {
        self.label_ids = Some(labels);
        self
    }

    /// Set the preset.
    pub fn preset(mut self, preset: impl Into<String>) -> Self {
        self.preset = Some(preset.into());
        self
    }

    /// Set the truncation mode.
    pub fn truncate(mut self, mode: TruncateMode) -> Self {
        self.truncate = Some(mode);
        self
    }

    /// Build the request.
    pub fn build(self) -> ClassifyRequest {
        ClassifyRequest {
            inputs: self.inputs,
            model: self.model,
            examples: self.examples,
            label_ids: self.label_ids,
            preset: self.preset,
            truncate: self.truncate,
        }
    }
}

impl Default for ClassifyRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// An example for few-shot classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    /// The example text.
    pub text: String,
    /// The label for the example.
    pub label: String,
}

impl Example {
    /// Create a new example.
    pub fn new(text: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            label: label.into(),
        }
    }
}

/// Truncation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TruncateMode {
    /// Truncate only the start.
    Start,
    /// Truncate only the end.
    End,
    /// Truncate both start and end.
    Both,
    /// Don't truncate (error if too long).
    None,
}

/// A response from the Cohere Classify API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyResponse {
    /// The classifications.
    pub classifications: Vec<Classification>,

    /// API metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ApiMeta>,
}

impl ClassifyResponse {
    /// Get the classification for the first input.
    pub fn first(&self) -> Option<&Classification> {
        self.classifications.first()
    }

    /// Get the classification at index.
    pub fn get(&self, index: usize) -> Option<&Classification> {
        self.classifications.get(index)
    }

    /// Get the number of classifications.
    pub fn len(&self) -> usize {
        self.classifications.len()
    }

    /// Check if there are no classifications.
    pub fn is_empty(&self) -> bool {
        self.classifications.is_empty()
    }
}

/// A single classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Classification {
    /// The input text.
    pub input: String,

    /// The predicted label.
    #[serde(rename = "prediction")]
    pub prediction: String,

    /// Confidence scores for each label.
    pub confidence: f32,

    /// Confidence scores for all labels.
    pub labels: serde_json::Value,
}

impl Classification {
    /// Get the predicted label.
    pub fn label(&self) -> &str {
        &self.prediction
    }

    /// Get the confidence score.
    pub fn confidence(&self) -> f32 {
        self.confidence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_few_shot() {
        let request = ClassifyRequest::builder()
            .add_input("This is great!")
            .add_input("This is terrible...")
            .add_example("I love this!", "positive")
            .add_example("I hate this!", "negative")
            .build();

        assert_eq!(request.inputs.len(), 2);
        assert_eq!(request.examples.as_ref().map(|e| e.len()), Some(2));
    }

    #[test]
    fn test_builder_zero_shot() {
        let request = ClassifyRequest::builder()
            .model("embed-english-v3.0")
            .add_input("This is a test")
            .add_label("sports")
            .add_label("politics")
            .add_label("technology")
            .build();

        assert_eq!(request.model, Some("embed-english-v3.0".to_string()));
        assert_eq!(request.label_ids.as_ref().map(|l| l.len()), Some(3));
    }

    #[test]
    fn test_example_creation() {
        let example = Example::new("Great product!", "positive");

        assert_eq!(example.text, "Great product!");
        assert_eq!(example.label, "positive");
    }

    #[test]
    fn test_response_helpers() {
        let response = ClassifyResponse {
            classifications: vec![Classification {
                input: "test".to_string(),
                prediction: "positive".to_string(),
                confidence: 0.95,
                labels: serde_json::json!({
                    "positive": 0.95,
                    "negative": 0.05
                }),
            }],
            meta: None,
        };

        assert_eq!(response.len(), 1);
        let first = response.first().unwrap();
        assert_eq!(first.label(), "positive");
        assert_eq!(first.confidence(), 0.95);
    }
}

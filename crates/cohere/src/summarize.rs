//! Summarize API for Cohere's summarization models.

use serde::{Deserialize, Serialize};

use crate::client::CohereClient;
use crate::constants::endpoints;
use crate::error::Result;
use crate::types::ApiMeta;

/// Client for the Summarize API.
#[derive(Debug)]
pub struct SummarizeEndpoint<'a> {
    pub(crate) client: &'a CohereClient,
}

impl<'a> SummarizeEndpoint<'a> {
    /// Send a summarize request.
    pub async fn create(&self, request: SummarizeRequest) -> Result<SummarizeResponse> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::SUMMARIZE, body).await?;
                    let body = client.handle_response(response).await?;
                    let summarize_response: SummarizeResponse = serde_json::from_value(body)?;
                    Ok(summarize_response)
                })
            })
            .await
    }
}

/// A request to the Cohere Summarize API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizeRequest {
    /// The text to summarize.
    pub text: String,

    /// The length of the summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<SummaryLength>,

    /// The format of the summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<SummaryFormat>,

    /// The model to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Additional command for the summarization.
    #[serde(rename = "additional_command", skip_serializing_if = "Option::is_none")]
    pub additional_command: Option<String>,

    /// Temperature for sampling (0.0 to 5.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Truncate the input if it's too long.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncate: Option<TruncateMode>,
}

impl SummarizeRequest {
    /// Create a new request builder.
    pub fn builder() -> SummarizeRequestBuilder {
        SummarizeRequestBuilder::new()
    }

    /// Create a simple summarize request.
    pub fn simple(text: impl Into<String>) -> Self {
        Self::builder().text(text).build()
    }
}

/// Builder for summarize requests.
#[derive(Debug, Clone)]
pub struct SummarizeRequestBuilder {
    text: String,
    length: Option<SummaryLength>,
    format: Option<SummaryFormat>,
    model: Option<String>,
    additional_command: Option<String>,
    temperature: Option<f32>,
    truncate: Option<TruncateMode>,
}

impl SummarizeRequestBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            text: String::new(),
            length: None,
            format: None,
            model: None,
            additional_command: None,
            temperature: None,
            truncate: None,
        }
    }

    /// Set the text to summarize.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    /// Set the summary length.
    pub fn length(mut self, length: SummaryLength) -> Self {
        self.length = Some(length);
        self
    }

    /// Set the summary format.
    pub fn format(mut self, format: SummaryFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Set the model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set an additional command.
    pub fn additional_command(mut self, command: impl Into<String>) -> Self {
        self.additional_command = Some(command.into());
        self
    }

    /// Set the temperature.
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp.clamp(0.0, 5.0));
        self
    }

    /// Set the truncation mode.
    pub fn truncate(mut self, mode: TruncateMode) -> Self {
        self.truncate = Some(mode);
        self
    }

    /// Build the request.
    pub fn build(self) -> SummarizeRequest {
        SummarizeRequest {
            text: self.text,
            length: self.length,
            format: self.format,
            model: self.model,
            additional_command: self.additional_command,
            temperature: self.temperature,
            truncate: self.truncate,
        }
    }
}

impl Default for SummarizeRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary length options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SummaryLength {
    /// Short summary (1-2 sentences).
    Short,
    /// Medium summary (3-4 sentences).
    Medium,
    /// Long summary (5+ sentences).
    Long,
    /// Automatic length based on input.
    Auto,
}

/// Summary format options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SummaryFormat {
    /// Paragraph format.
    Paragraph,
    /// Bullet points format.
    Bullets,
    /// Automatic format.
    Auto,
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

/// A response from the Cohere Summarize API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizeResponse {
    /// The generated summary.
    pub summary: String,

    /// The ID of the response.
    pub id: String,

    /// API metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ApiMeta>,
}

impl SummarizeResponse {
    /// Get the summary text.
    pub fn text(&self) -> &str {
        &self.summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let request = SummarizeRequest::builder()
            .text("This is a long text that needs to be summarized...")
            .length(SummaryLength::Medium)
            .format(SummaryFormat::Paragraph)
            .build();

        assert_eq!(request.text, "This is a long text that needs to be summarized...");
        assert_eq!(request.length, Some(SummaryLength::Medium));
        assert_eq!(request.format, Some(SummaryFormat::Paragraph));
    }

    #[test]
    fn test_builder_with_options() {
        let request = SummarizeRequest::builder()
            .text("Text to summarize")
            .model("summarize-xlarge")
            .additional_command("Focus on key points")
            .temperature(0.5)
            .truncate(TruncateMode::End)
            .build();

        assert_eq!(request.model, Some("summarize-xlarge".to_string()));
        assert_eq!(request.additional_command, Some("Focus on key points".to_string()));
        assert_eq!(request.temperature, Some(0.5));
        assert_eq!(request.truncate, Some(TruncateMode::End));
    }

    #[test]
    fn test_response() {
        let response = SummarizeResponse {
            summary: "This is the summary.".to_string(),
            id: "summ_123".to_string(),
            meta: None,
        };

        assert_eq!(response.text(), "This is the summary.");
    }
}

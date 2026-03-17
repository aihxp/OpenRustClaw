//! Summarization API for Cloudflare Workers AI.

use crate::client::CloudflareAiClient;
use crate::error::Result;
use crate::types::SummarizationResponse;

/// Client for the summarization API.
#[derive(Debug)]
pub struct Summarization<'a> {
    client: &'a CloudflareAiClient,
}

impl<'a> Summarization<'a> {
    /// Create a new summarization client.
    pub fn new(client: &'a CloudflareAiClient) -> Self {
        Self { client }
    }

    /// Summarize a text.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::{CloudflareAiClient, SummarizationRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// let text = "Long text to summarize...".to_string();
    /// let request = SummarizationRequest::new(text);
    /// let response = client.summarization().summarize(request).await?;
    /// 
    /// if let Some(summary) = response.summary() {
    ///     println!("Summary: {}", summary);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn summarize(&self, request: SummarizationRequest) -> Result<SummarizationResponse> {
        let model = request.model.clone();
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                let model = model.clone();
                Box::pin(async move {
                    let response = client.post(&model, body).await?;
                    let result: SummarizationResponse = client.handle_response(response).await?;
                    Ok(result)
                })
            })
            .await
    }

    /// Summarize text with a simple interface.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::CloudflareAiClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// let response = client.summarization()
    ///     .summarize_text("Long text to summarize...")
    ///     .await?;
    /// 
    /// if let Some(summary) = response.summary() {
    ///     println!("Summary: {}", summary);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn summarize_text(
        &self,
        text: impl Into<String>,
    ) -> Result<SummarizationResponse> {
        let request = SummarizationRequest::new(text);
        self.summarize(request).await
    }

    /// Summarize text with a maximum word count.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::CloudflareAiClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// let response = client.summarization()
    ///     .summarize_with_max_length("Long text to summarize...", 100)
    ///     .await?;
    /// 
    /// if let Some(summary) = response.summary() {
    ///     println!("Summary: {}", summary);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn summarize_with_max_length(
        &self,
        text: impl Into<String>,
        max_length: usize,
    ) -> Result<SummarizationResponse> {
        let request = SummarizationRequest::builder()
            .text(text)
            .max_length(max_length)
            .build();
        self.summarize(request).await
    }
}

/// A summarization request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SummarizationRequest {
    /// ID of the model to use (default: "@cf/facebook/bart-large-cnn").
    #[serde(skip_serializing)]
    pub model: String,
    /// The text to summarize.
    pub text: String,
    /// Maximum length of the summary (in words).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    /// Minimum length of the summary (in words).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
}

impl SummarizationRequest {
    /// Create a new summarization request.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to summarize
    ///
    /// # Example
    ///
    /// ```
    /// use cloudflare_ai::SummarizationRequest;
    ///
    /// let request = SummarizationRequest::new("Long text to summarize...");
    /// ```
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            model: "@cf/facebook/bart-large-cnn".to_string(),
            text: text.into(),
            max_length: None,
            min_length: None,
        }
    }

    /// Create a summarization request with length constraints.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to summarize
    /// * `max_length` - Maximum length of the summary in words
    ///
    /// # Example
    ///
    /// ```
    /// use cloudflare_ai::SummarizationRequest;
    ///
    /// let request = SummarizationRequest::with_max_length("Long text to summarize...", 100);
    /// ```
    pub fn with_max_length(text: impl Into<String>, max_length: usize) -> Self {
        Self {
            model: "@cf/facebook/bart-large-cnn".to_string(),
            text: text.into(),
            max_length: Some(max_length),
            min_length: None,
        }
    }

    /// Create a builder for summarization requests.
    pub fn builder() -> SummarizationRequestBuilder {
        SummarizationRequestBuilder::new()
    }

    /// Set a custom model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the maximum length.
    pub fn max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }

    /// Set the minimum length.
    pub fn min_length(mut self, min_length: usize) -> Self {
        self.min_length = Some(min_length);
        self
    }
}

/// Builder for summarization requests.
#[derive(Debug, Clone, Default)]
pub struct SummarizationRequestBuilder {
    model: String,
    text: Option<String>,
    max_length: Option<usize>,
    min_length: Option<usize>,
}

impl SummarizationRequestBuilder {
    /// Create a new summarization request builder.
    pub fn new() -> Self {
        Self {
            model: "@cf/facebook/bart-large-cnn".to_string(),
            text: None,
            max_length: None,
            min_length: None,
        }
    }

    /// Set the text to summarize.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Set the maximum length.
    pub fn max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }

    /// Set the minimum length.
    pub fn min_length(mut self, min_length: usize) -> Self {
        self.min_length = Some(min_length);
        self
    }

    /// Set a custom model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Build the request.
    pub fn build(self) -> SummarizationRequest {
        SummarizationRequest {
            model: self.model,
            text: self.text.unwrap_or_default(),
            max_length: self.max_length,
            min_length: self.min_length,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summarization_request_new() {
        let req = SummarizationRequest::new("Long text...");
        assert_eq!(req.text, "Long text...");
        assert_eq!(req.max_length, None);
        assert_eq!(req.model, "@cf/facebook/bart-large-cnn");
    }

    #[test]
    fn test_summarization_request_with_max_length() {
        let req = SummarizationRequest::with_max_length("Long text...", 100);
        assert_eq!(req.text, "Long text...");
        assert_eq!(req.max_length, Some(100));
        assert_eq!(req.min_length, None);
    }

    #[test]
    fn test_summarization_request_with_model() {
        let req = SummarizationRequest::new("Long text...")
            .with_model("@cf/custom/model");
        assert_eq!(req.model, "@cf/custom/model");
    }

    #[test]
    fn test_builder() {
        let req = SummarizationRequest::builder()
            .text("Long text to summarize...")
            .max_length(100)
            .min_length(10)
            .build();

        assert_eq!(req.text, "Long text to summarize...");
        assert_eq!(req.max_length, Some(100));
        assert_eq!(req.min_length, Some(10));
    }

    #[test]
    fn test_summarization_response() {
        let response: SummarizationResponse = serde_json::from_str(
            r#"{
                "summary": "Short summary."
            }"#
        ).unwrap();

        assert_eq!(response.summary(), Some("Short summary."));
    }
}

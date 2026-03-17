//! Gemini API client

use crate::types::*;
use crate::error::GeminiError;
use reqwest::{Client, Response};
use secrecy::{ExposeSecret, SecretString};
use std::time::Duration;

/// Gemini API client
pub struct GeminiClient {
    client: Client,
    api_key: SecretString,
    base_url: String,
    default_model: GeminiModel,
}

impl GeminiClient {
    /// Create a new Gemini client
    pub fn new(api_key: impl Into<SecretString>) -> Self {
        Self {
            client: Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(120))
                .build()
                .expect("Failed to build HTTP client"),
            api_key: api_key.into(),
            base_url: crate::DEFAULT_BASE_URL.to_string(),
            default_model: GeminiModel::default(),
        }
    }
    
    /// Create with custom HTTP client
    pub fn with_client(mut self, client: Client) -> Self {
        self.client = client;
        self
    }
    
    /// Set base URL (for testing or different regions)
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
    
    /// Set default model
    pub fn with_model(mut self, model: GeminiModel) -> Self {
        self.default_model = model;
        self
    }
    
    /// Get the default model
    pub fn default_model(&self) -> &GeminiModel {
        &self.default_model
    }
    
    /// Build request URL
    fn build_url(&self, model: &GeminiModel, stream: bool) -> String {
        let action = if stream { "streamGenerateContent" } else { "generateContent" };
        format!(
            "{}/models/{}:{}?key={}",
            self.base_url,
            model.as_str(),
            action,
            self.api_key.expose_secret()
        )
    }
    
    /// Generate content
    pub async fn generate_content(
        &self,
        request: GenerateContentRequest,
    ) -> Result<GenerateContentResponse, GeminiError> {
        self.generate_content_with_model(self.default_model.clone(), request).await
    }
    
    /// Generate content with specific model
    pub async fn generate_content_with_model(
        &self,
        model: GeminiModel,
        request: GenerateContentRequest,
    ) -> Result<GenerateContentResponse, GeminiError> {
        let url = self.build_url(&model, false);
        
        tracing::debug!("Sending generate content request to {}", url);
        
        let response = self.client
            .post(&url)
            .json(&request)
            .timeout(Duration::from_secs(60))
            .send()
            .await?;
            
        self.handle_response(response).await
    }
    
    /// Stream generate content
    pub async fn stream_generate_content(
        &self,
        request: GenerateContentRequest,
    ) -> Result<impl futures::Stream<Item = Result<GenerateContentResponse, GeminiError>>, GeminiError> {
        let url = self.build_url(&self.default_model, true);
        
        tracing::debug!("Sending streaming request to {}", url);
        
        let response = self.client
            .post(&url)
            .json(&request)
            .timeout(Duration::from_secs(120))
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error = self.parse_error(response).await?;
            return Err(error);
        }
        
        Ok(crate::streaming::GeminiStream::new(response.bytes_stream()))
    }
    
    /// Create chat session
    pub fn chat(&self) -> crate::chat::ChatSession<'_> {
        crate::chat::ChatSession::new(self)
    }
    
    /// Get embedding for content
    pub async fn embed_content(
        &self,
        content: Content,
        output_dimensionality: Option<i32>,
    ) -> Result<Embedding, GeminiError> {
        let request = EmbedContentRequest {
            model: format!("models/{}", GeminiModel::Embedding004.as_str()),
            content,
            output_dimensionality,
        };
        
        let url = format!("{}/models/{}:embedContent?key={}",
            self.base_url,
            GeminiModel::Embedding004.as_str(),
            self.api_key.expose_secret()
        );
        
        tracing::debug!("Sending embedding request to {}", url);
        
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;
            
        let result: EmbedContentResponse = self.parse_response(response).await?;
        Ok(result.embedding)
    }
    
    /// Get embeddings for multiple contents
    pub async fn batch_embed_contents(
        &self,
        contents: Vec<Content>,
        output_dimensionality: Option<i32>,
    ) -> Result<Vec<Embedding>, GeminiError> {
        let requests: Vec<EmbedContentRequest> = contents
            .into_iter()
            .map(|content| EmbedContentRequest {
                model: format!("models/{}", GeminiModel::Embedding004.as_str()),
                content,
                output_dimensionality,
            })
            .collect();
        
        let request = BatchEmbedContentsRequest { requests };
        
        let url = format!("{}/models/{}:batchEmbedContents?key={}",
            self.base_url,
            GeminiModel::Embedding004.as_str(),
            self.api_key.expose_secret()
        );
        
        tracing::debug!("Sending batch embedding request to {}", url);
        
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;
            
        let result: BatchEmbedContentsResponse = self.parse_response(response).await?;
        Ok(result.embeddings)
    }
    
    /// Handle API response
    async fn handle_response(&self, response: Response) -> Result<GenerateContentResponse, GeminiError> {
        if response.status().is_success() {
            let result = response.json().await?;
            Ok(result)
        } else {
            Err(self.parse_error(response).await?)
        }
    }
    
    /// Parse error response
    async fn parse_error(&self, response: Response) -> Result<GeminiError, GeminiError> {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        
        tracing::error!("API error: HTTP {} - {}", status, text);
        
        Ok(GeminiError::ApiError {
            status,
            message: text,
        })
    }
    
    /// Parse successful response
    async fn parse_response<T: serde::de::DeserializeOwned>(&self, response: Response) -> Result<T, GeminiError> {
        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(self.parse_error(response).await?)
        }
    }
    
    /// List available models
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, GeminiError> {
        let url = format!("{}/models?key={}", self.base_url, self.api_key.expose_secret());
        
        tracing::debug!("Listing models from {}", url);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;
            
        let result: ListModelsResponse = self.parse_response(response).await?;
        Ok(result.models)
    }
    
    /// Count tokens in content
    pub async fn count_tokens(&self, contents: Vec<Content>) -> Result<CountTokensResponse, GeminiError> {
        let url = format!("{}/models/{}:countTokens?key={}",
            self.base_url,
            self.default_model.as_str(),
            self.api_key.expose_secret()
        );

        let request = CountTokensRequest { contents };
        
        tracing::debug!("Sending count tokens request to {}", url);
        
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;
            
        self.parse_response(response).await
    }
    
    /// Get model info
    pub async fn get_model(&self, model_name: &str) -> Result<ModelInfo, GeminiError> {
        let url = format!("{}/models/{}?key={}", self.base_url, model_name, self.api_key.expose_secret());
        
        tracing::debug!("Getting model info from {}", url);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;
            
        self.parse_response(response).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_client_creation() {
        let client = GeminiClient::new("test-api-key");
        assert!(matches!(client.default_model, GeminiModel::Gemini15Pro));
    }
    
    #[test]
    fn test_client_with_model() {
        let client = GeminiClient::new("test-api-key")
            .with_model(GeminiModel::Gemini15Flash);
        assert!(matches!(client.default_model, GeminiModel::Gemini15Flash));
    }
    
    #[test]
    fn test_build_url() {
        let client = GeminiClient::new("test-key");
        let url = client.build_url(&GeminiModel::Gemini15Pro, false);
        assert!(url.contains("generateContent"));
        assert!(url.contains("test-key"));
        
        let stream_url = client.build_url(&GeminiModel::Gemini15Pro, true);
        assert!(stream_url.contains("streamGenerateContent"));
    }
}

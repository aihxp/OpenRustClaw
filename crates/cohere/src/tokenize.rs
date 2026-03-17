//! Tokenize and Detokenize APIs for Cohere's tokenization.

use serde::{Deserialize, Serialize};

use crate::client::CohereClient;
use crate::constants::endpoints;
use crate::error::Result;
use crate::types::ApiMeta;

/// Client for the Tokenize API.
#[derive(Debug)]
pub struct TokenizeEndpoint<'a> {
    pub(crate) client: &'a CohereClient,
}

impl<'a> TokenizeEndpoint<'a> {
    /// Tokenize text.
    pub async fn tokenize(&self, request: TokenizeRequest) -> Result<TokenizeResponse> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::TOKENIZE, body).await?;
                    let body = client.handle_response(response).await?;
                    let tokenize_response: TokenizeResponse = serde_json::from_value(body)?;
                    Ok(tokenize_response)
                })
            })
            .await
    }

    /// Detokenize token IDs back to text.
    pub async fn detokenize(&self, request: DetokenizeRequest) -> Result<DetokenizeResponse> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::DETOKENIZE, body).await?;
                    let body = client.handle_response(response).await?;
                    let detokenize_response: DetokenizeResponse = serde_json::from_value(body)?;
                    Ok(detokenize_response)
                })
            })
            .await
    }

    /// Convenience method to tokenize a single text.
    pub async fn encode(
        &self,
        model: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<Vec<i64>> {
        let request = TokenizeRequest::new(model, text);
        let response = self.tokenize(request).await?;
        Ok(response.tokens)
    }

    /// Convenience method to detokenize token IDs.
    pub async fn decode(&self, model: impl Into<String>, tokens: Vec<i64>) -> Result<String> {
        let request = DetokenizeRequest::new(model, tokens);
        let response = self.detokenize(request).await?;
        Ok(response.text)
    }
}

/// A request to the Cohere Tokenize API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizeRequest {
    /// The text to tokenize.
    pub text: String,

    /// The model to use for tokenization.
    pub model: String,
}

impl TokenizeRequest {
    /// Create a new tokenize request.
    pub fn new(model: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            model: model.into(),
        }
    }
}

/// A response from the Cohere Tokenize API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizeResponse {
    /// The token IDs.
    pub tokens: Vec<i64>,

    /// The token strings (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_strings: Option<Vec<String>>,

    /// The text that was tokenized.
    pub text: String,

    /// API metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ApiMeta>,
}

impl TokenizeResponse {
    /// Get the tokens.
    pub fn tokens(&self) -> &[i64] {
        &self.tokens
    }

    /// Get the number of tokens.
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Check if there are no tokens.
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Get the token strings if available.
    pub fn token_strings(&self) -> Option<&[String]> {
        self.token_strings.as_deref()
    }
}

/// A request to the Cohere Detokenize API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetokenizeRequest {
    /// The token IDs to detokenize.
    pub tokens: Vec<i64>,

    /// The model to use for detokenization.
    pub model: String,
}

impl DetokenizeRequest {
    /// Create a new detokenize request.
    pub fn new(model: impl Into<String>, tokens: Vec<i64>) -> Self {
        Self {
            tokens,
            model: model.into(),
        }
    }
}

/// A response from the Cohere Detokenize API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetokenizeResponse {
    /// The detokenized text.
    pub text: String,

    /// API metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ApiMeta>,
}

impl DetokenizeResponse {
    /// Get the text.
    pub fn text(&self) -> &str {
        &self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_request() {
        let request = TokenizeRequest::new("command-r", "Hello world");

        assert_eq!(request.text, "Hello world");
        assert_eq!(request.model, "command-r");
    }

    #[test]
    fn test_detokenize_request() {
        let tokens = vec![1, 2, 3, 4, 5];
        let request = DetokenizeRequest::new("command-r", tokens.clone());

        assert_eq!(request.tokens, tokens);
        assert_eq!(request.model, "command-r");
    }

    #[test]
    fn test_tokenize_response() {
        let response = TokenizeResponse {
            tokens: vec![100, 200, 300],
            token_strings: Some(vec![
                "Hello".to_string(),
                " world".to_string(),
                "!".to_string(),
            ]),
            text: "Hello world!".to_string(),
            meta: None,
        };

        assert_eq!(response.len(), 3);
        assert_eq!(response.tokens(), &[100, 200, 300]);
        assert_eq!(
            response.token_strings(),
            Some(vec!["Hello".to_string(), " world".to_string(), "!".to_string()].as_slice())
        );
    }

    #[test]
    fn test_detokenize_response() {
        let response = DetokenizeResponse {
            text: "Hello world!".to_string(),
            meta: None,
        };

        assert_eq!(response.text(), "Hello world!");
    }
}

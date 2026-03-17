//! Tokenization API for AI21 models.

use serde::{Deserialize, Serialize};

use crate::client::Ai21Client;
use crate::constants::endpoints;
use crate::error::Result;

/// Client for the Tokenization API.
#[derive(Debug)]
pub struct TokenizeEndpoint<'a> {
    pub(crate) client: &'a Ai21Client,
}

impl<'a> TokenizeEndpoint<'a> {
    /// Tokenize text using the specified model.
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

    /// Helper method to count tokens in text.
    pub async fn count_tokens(
        &self,
        model: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<usize> {
        let request = TokenizeRequest::new(model, text);
        let response = self.tokenize(request).await?;
        Ok(response.tokens.len())
    }

    /// Helper method to encode text to tokens.
    pub async fn encode(
        &self,
        model: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<Vec<Token>> {
        let request = TokenizeRequest::new(model, text);
        let response = self.tokenize(request).await?;
        Ok(response.tokens)
    }

    /// Helper method to decode tokens to text.
    pub async fn decode(
        &self,
        model: impl Into<String>,
        tokens: Vec<u64>,
    ) -> Result<String> {
        let request = DetokenizeRequest::new(model, tokens);
        let response = self.detokenize(request).await?;
        Ok(response.text)
    }
}

/// A request to the Tokenize API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizeRequest {
    /// The model to use for tokenization (e.g., "j2-ultra").
    pub model: String,

    /// The text to tokenize.
    pub text: String,
}

impl TokenizeRequest {
    /// Create a new tokenize request.
    pub fn new(model: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            text: text.into(),
        }
    }
}

/// A response from the Tokenize API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizeResponse {
    /// The tokens generated from the text.
    pub tokens: Vec<Token>,
}

impl TokenizeResponse {
    /// Get the number of tokens.
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Check if there are no tokens.
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Get the token IDs.
    pub fn token_ids(&self) -> Vec<u64> {
        self.tokens.iter().map(|t| t.id).collect()
    }

    /// Get the token texts.
    pub fn token_texts(&self) -> Vec<&str> {
        self.tokens.iter().map(|t| t.text.as_str()).collect()
    }
}

/// A token in the response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    /// The token ID.
    #[serde(rename = "token")]
    pub id: u64,

    /// The token text.
    #[serde(rename = "text")]
    pub text: String,
}

/// A request to the Detokenize API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetokenizeRequest {
    /// The model that was used to generate the tokens.
    pub model: String,

    /// The tokens to detokenize.
    pub tokens: Vec<u64>,
}

impl DetokenizeRequest {
    /// Create a new detokenize request.
    pub fn new(model: impl Into<String>, tokens: Vec<u64>) -> Self {
        Self {
            model: model.into(),
            tokens,
        }
    }
}

/// A response from the Detokenize API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetokenizeResponse {
    /// The detokenized text.
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_request() {
        let request = TokenizeRequest::new("j2-ultra", "Hello world");
        assert_eq!(request.model, "j2-ultra");
        assert_eq!(request.text, "Hello world");
    }

    #[test]
    fn test_detokenize_request() {
        let tokens = vec![1, 2, 3, 4, 5];
        let request = DetokenizeRequest::new("j2-mid", tokens.clone());
        assert_eq!(request.model, "j2-mid");
        assert_eq!(request.tokens, tokens);
    }

    #[test]
    fn test_tokenize_response_helpers() {
        let response = TokenizeResponse {
            tokens: vec![
                Token {
                    id: 100,
                    text: "Hello".to_string(),
                },
                Token {
                    id: 101,
                    text: " world".to_string(),
                },
            ],
        };

        assert_eq!(response.len(), 2);
        assert!(!response.is_empty());
        assert_eq!(response.token_ids(), vec![100, 101]);
        assert_eq!(response.token_texts(), vec!["Hello", " world"]);
    }
}

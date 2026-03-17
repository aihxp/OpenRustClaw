//! Tokenization API for llama.cpp.

use crate::client::LlamaCppClient;
use crate::client::endpoints;
use crate::error::Result;
use crate::types::TokenizeResponse;

/// Client for the tokenization API.
#[derive(Debug)]
pub struct Tokenize<'a> {
    client: &'a LlamaCppClient,
}

impl<'a> Tokenize<'a> {
    /// Create a new tokenize client.
    pub fn new(client: &'a LlamaCppClient) -> Self {
        Self { client }
    }

    /// Tokenize text into token IDs.
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

    /// Convenience method to tokenize a simple string.
    pub async fn tokenize_text(&self, text: impl Into<String>) -> Result<TokenizeResponse> {
        self.tokenize(TokenizeRequest::new(text)).await
    }

    /// Count the number of tokens in a text.
    pub async fn count_tokens(&self, text: impl Into<String>) -> Result<usize> {
        let response = self.tokenize_text(text).await?;
        Ok(response.tokens.len())
    }
}

/// A tokenize request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenizeRequest {
    /// The text to tokenize.
    pub content: String,
    /// Add special tokens (like BOS/EOS).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_special: Option<bool>,
    /// Parse special tokens in the content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_pieces: Option<bool>,
}

impl TokenizeRequest {
    /// Create a new tokenize request.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            add_special: None,
            with_pieces: None,
        }
    }

    /// Create a builder.
    pub fn builder(content: impl Into<String>) -> TokenizeRequestBuilder {
        TokenizeRequestBuilder::new(content)
    }

    /// Set whether to add special tokens.
    pub fn add_special(mut self, add: bool) -> Self {
        self.add_special = Some(add);
        self
    }

    /// Set whether to return token pieces.
    pub fn with_pieces(mut self, with_pieces: bool) -> Self {
        self.with_pieces = Some(with_pieces);
        self
    }
}

/// Builder for tokenize requests.
#[derive(Debug, Clone)]
pub struct TokenizeRequestBuilder {
    content: String,
    add_special: Option<bool>,
    with_pieces: Option<bool>,
}

impl TokenizeRequestBuilder {
    /// Create a new builder.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            add_special: None,
            with_pieces: None,
        }
    }

    /// Set whether to add special tokens.
    pub fn add_special(mut self, add: bool) -> Self {
        self.add_special = Some(add);
        self
    }

    /// Set whether to return token pieces.
    pub fn with_pieces(mut self, with_pieces: bool) -> Self {
        self.with_pieces = Some(with_pieces);
        self
    }

    /// Build the request.
    pub fn build(self) -> TokenizeRequest {
        TokenizeRequest {
            content: self.content,
            add_special: self.add_special,
            with_pieces: self.with_pieces,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_new() {
        let request = TokenizeRequest::new("Hello world");
        assert_eq!(request.content, "Hello world");
        assert_eq!(request.add_special, None);
    }

    #[test]
    fn test_request_builder() {
        let request = TokenizeRequest::builder("Hello world")
            .add_special(true)
            .with_pieces(true)
            .build();

        assert_eq!(request.content, "Hello world");
        assert_eq!(request.add_special, Some(true));
        assert_eq!(request.with_pieces, Some(true));
    }

    #[test]
    fn test_request_methods() {
        let request = TokenizeRequest::new("Hello")
            .add_special(true)
            .with_pieces(false);

        assert_eq!(request.add_special, Some(true));
        assert_eq!(request.with_pieces, Some(false));
    }
}

//! Tokenization API for vLLM (vLLM-specific endpoints).

use crate::client::VllmClient;
use crate::constants::endpoints;
use crate::error::Result;

/// Client for tokenize/detokenize operations.
#[derive(Debug)]
pub struct Tokenize<'a> {
    client: &'a VllmClient,
}

impl<'a> Tokenize<'a> {
    /// Create a new tokenize client.
    pub fn new(client: &'a VllmClient) -> Self {
        Self { client }
    }

    /// Tokenize text into token IDs.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vllm::VllmClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = VllmClient::new("http://localhost:8000")?;
    ///
    /// let tokens = client.tokenize()
    ///     .tokenize("meta-llama/Llama-3-8b-chat-hf", "Hello, world!")
    ///     .await?;
    ///
    /// println!("Token count: {}", tokens.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn tokenize(
        &self,
        model: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Result<Vec<i64>> {
        let request = TokenizeRequest {
            model: model.into(),
            prompt: prompt.into(),
        };

        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::TOKENIZE, body).await?;
                    let body = client.handle_response(response).await?;
                    let tokenize_response: TokenizeResponse = serde_json::from_value(body)?;
                    Ok(tokenize_response.tokens)
                })
            })
            .await
    }

    /// Detokenize token IDs back into text.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vllm::VllmClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = VllmClient::new("http://localhost:8000")?;
    ///
    /// let tokens = vec![9906, 11, 1917, 0]; // Token IDs for "Hello, world!"
    /// let text = client.tokenize()
    ///     .detokenize("meta-llama/Llama-3-8b-chat-hf", &tokens)
    ///     .await?;
    ///
    /// println!("Text: {}", text);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn detokenize(
        &self,
        model: impl Into<String>,
        tokens: &[i64],
    ) -> Result<String> {
        let request = DetokenizeRequest {
            model: model.into(),
            tokens: tokens.to_vec(),
        };

        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::DETOKENIZE, body).await?;
                    let body = client.handle_response(response).await?;
                    let detokenize_response: DetokenizeResponse = serde_json::from_value(body)?;
                    Ok(detokenize_response.prompt)
                })
            })
            .await
    }

    /// Count tokens in text.
    ///
    /// This is a convenience method that tokenizes and returns the count.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vllm::VllmClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = VllmClient::new("http://localhost:8000")?;
    ///
    /// let count = client.tokenize()
    ///     .count_tokens("meta-llama/Llama-3-8b-chat-hf", "Hello, world!")
    ///     .await?;
    ///
    /// println!("Token count: {}", count);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn count_tokens(
        &self,
        model: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Result<usize> {
        let tokens = self.tokenize(model, prompt).await?;
        Ok(tokens.len())
    }
}

/// A tokenize request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenizeRequest {
    /// Model ID.
    pub model: String,
    /// Text to tokenize.
    pub prompt: String,
}

/// A tokenize response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenizeResponse {
    /// The token IDs.
    pub tokens: Vec<i64>,
    /// Number of tokens.
    pub count: usize,
    /// Maximum sequence length for the model.
    #[serde(rename = "max_model_len", skip_serializing_if = "Option::is_none")]
    pub max_model_len: Option<usize>,
}

/// A detokenize request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DetokenizeRequest {
    /// Model ID.
    pub model: String,
    /// Token IDs to detokenize.
    pub tokens: Vec<i64>,
}

/// A detokenize response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DetokenizeResponse {
    /// The detokenized text.
    pub prompt: String,
}

/// Token information for analysis.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenInfo {
    /// The token ID.
    pub id: i64,
    /// The token text (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Whether this is a special token.
    #[serde(default)]
    pub special: bool,
}

/// Batch tokenize request for multiple prompts.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchTokenizeRequest {
    /// Model ID.
    pub model: String,
    /// Texts to tokenize.
    pub prompts: Vec<String>,
}

/// Batch tokenize response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchTokenizeResponse {
    /// Tokenization results for each prompt.
    pub results: Vec<TokenizeResponse>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_request_serialization() {
        let request = TokenizeRequest {
            model: "meta-llama/Llama-3-8b".to_string(),
            prompt: "Hello, world!".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("meta-llama/Llama-3-8b"));
        assert!(json.contains("Hello, world!"));
    }

    #[test]
    fn test_tokenize_response_deserialization() {
        let json = r#"{
            "tokens": [9906, 11, 1917, 0],
            "count": 4,
            "max_model_len": 8192
        }"#;

        let response: TokenizeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.tokens, vec![9906, 11, 1917, 0]);
        assert_eq!(response.count, 4);
        assert_eq!(response.max_model_len, Some(8192));
    }

    #[test]
    fn test_detokenize_request_serialization() {
        let request = DetokenizeRequest {
            model: "meta-llama/Llama-3-8b".to_string(),
            tokens: vec![9906, 11, 1917, 0],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("meta-llama/Llama-3-8b"));
        assert!(json.contains("9906"));
    }
}

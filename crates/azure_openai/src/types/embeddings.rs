//! Embedding types for the Azure OpenAI API.

use serde::{Deserialize, Serialize};

/// An embedding response from the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsResponse {
    /// The object type (always "list").
    pub object: String,
    /// The list of embeddings.
    pub data: Vec<Embedding>,
    /// The model used.
    pub model: String,
    /// Usage statistics.
    pub usage: EmbeddingUsage,
}

impl EmbeddingsResponse {
    /// Get all embedding vectors.
    pub fn embeddings(&self) -> Vec<Vec<f32>> {
        self.data.iter().map(|e| e.embedding.clone()).collect()
    }

    /// Get the first embedding.
    pub fn first(&self) -> Option<&Embedding> {
        self.data.first()
    }
}

/// A single embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    /// The object type (always "embedding").
    pub object: String,
    /// The embedding vector.
    pub embedding: Vec<f32>,
    /// The index of this embedding in the list.
    pub index: usize,
}

impl Embedding {
    /// Get the dimensionality of this embedding.
    pub fn dimensions(&self) -> usize {
        self.embedding.len()
    }
}

/// Usage statistics for embedding requests.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    /// Tokens in the prompt.
    #[serde(rename = "prompt_tokens")]
    pub prompt_tokens: usize,
    /// Total tokens.
    #[serde(rename = "total_tokens")]
    pub total_tokens: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_dimensions() {
        let embedding = Embedding {
            object: "embedding".to_string(),
            embedding: vec![0.1, 0.2, 0.3, 0.4],
            index: 0,
        };
        assert_eq!(embedding.dimensions(), 4);
    }

    #[test]
    fn test_response_first() {
        let response = EmbeddingsResponse {
            object: "list".to_string(),
            data: vec![
                Embedding {
                    object: "embedding".to_string(),
                    embedding: vec![0.1, 0.2],
                    index: 0,
                },
                Embedding {
                    object: "embedding".to_string(),
                    embedding: vec![0.3, 0.4],
                    index: 1,
                },
            ],
            model: "text-embedding-3-small".to_string(),
            usage: EmbeddingUsage {
                prompt_tokens: 10,
                total_tokens: 10,
            },
        };

        assert_eq!(response.first().unwrap().index, 0);
        assert_eq!(response.embeddings().len(), 2);
    }
}

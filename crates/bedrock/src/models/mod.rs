//! Bedrock model-specific types and utilities.
//!
//! This module provides model-specific request/response formats for models
//! that don't fully support the Converse API.

use crate::constants::Model;

pub mod anthropic;
pub mod amazon;
pub mod cohere;
pub mod meta;
pub mod mistral;
pub mod ai21;
pub mod stability;

/// Get the provider-specific request format for a model.
pub fn get_request_format(model: &Model) -> ModelRequestFormat {
    match model.provider() {
        crate::constants::ModelProvider::Anthropic => ModelRequestFormat::Anthropic,
        crate::constants::ModelProvider::Amazon => ModelRequestFormat::Amazon,
        crate::constants::ModelProvider::Meta => ModelRequestFormat::Meta,
        crate::constants::ModelProvider::Mistral => ModelRequestFormat::Mistral,
        crate::constants::ModelProvider::Cohere => ModelRequestFormat::Cohere,
        crate::constants::ModelProvider::Ai21 => ModelRequestFormat::Ai21,
        crate::constants::ModelProvider::Stability => ModelRequestFormat::Stability,
    }
}

/// Model request format type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelRequestFormat {
    /// Anthropic format.
    Anthropic,
    /// Amazon format.
    Amazon,
    /// Meta format.
    Meta,
    /// Mistral format.
    Mistral,
    /// Cohere format.
    Cohere,
    /// AI21 format.
    Ai21,
    /// Stability AI format.
    Stability,
}

/// Helper to check if a model supports system prompts.
pub fn supports_system_prompt(model: &Model) -> bool {
    matches!(
        model,
        Model::Claude3Opus
            | Model::Claude3Sonnet
            | Model::Claude3Haiku
            | Model::Claude35Sonnet
            | Model::Claude35Haiku
            | Model::Claude35SonnetV2
            | Model::Llama38BInstruct
            | Model::Llama370BInstruct
            | Model::Llama318BInstruct
            | Model::Llama3170BInstruct
            | Model::Llama31405BInstruct
            | Model::Llama321BInstruct
            | Model::Llama323BInstruct
            | Model::Llama3211BVisionInstruct
            | Model::Llama3290BVisionInstruct
            | Model::CohereCommandR
            | Model::CohereCommandRPlus
    )
}

/// Helper to check if a model supports tool use.
pub fn supports_tool_use(model: &Model) -> bool {
    matches!(
        model,
        Model::Claude3Opus
            | Model::Claude3Sonnet
            | Model::Claude3Haiku
            | Model::Claude35Sonnet
            | Model::Claude35Haiku
            | Model::Claude35SonnetV2
    )
}

/// Helper to check if a model supports streaming.
pub fn supports_streaming(model: &Model) -> bool {
    // Most models support streaming
    !matches!(
        model,
        Model::TitanEmbeddingsG1 | Model::TitanEmbeddingsG1Text
    )
}

/// Helper to check if a model supports vision.
pub fn supports_vision(model: &Model) -> bool {
    matches!(
        model,
        Model::Claude3Opus
            | Model::Claude3Sonnet
            | Model::Claude3Haiku
            | Model::Claude35Sonnet
            | Model::Claude35Haiku
            | Model::Claude35SonnetV2
            | Model::Llama3211BVisionInstruct
            | Model::Llama3290BVisionInstruct
    )
}

/// Model capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModelCapabilities {
    /// Supports system prompts.
    pub system_prompt: bool,
    /// Supports tool use.
    pub tool_use: bool,
    /// Supports streaming.
    pub streaming: bool,
    /// Supports vision/image input.
    pub vision: bool,
    /// Supports the Converse API.
    pub converse_api: bool,
}

impl ModelCapabilities {
    /// Get capabilities for a model.
    pub fn for_model(model: &Model) -> Self {
        Self {
            system_prompt: supports_system_prompt(model),
            tool_use: supports_tool_use(model),
            streaming: supports_streaming(model),
            vision: supports_vision(model),
            converse_api: model.supports_converse(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_format() {
        assert_eq!(
            get_request_format(&Model::Claude3Opus),
            ModelRequestFormat::Anthropic
        );
        assert_eq!(
            get_request_format(&Model::Llama38BInstruct),
            ModelRequestFormat::Meta
        );
    }

    #[test]
    fn test_supports_system_prompt() {
        assert!(supports_system_prompt(&Model::Claude3Opus));
        assert!(supports_system_prompt(&Model::Llama38BInstruct));
        assert!(!supports_system_prompt(&Model::Mistral7BInstruct));
    }

    #[test]
    fn test_supports_tool_use() {
        assert!(supports_tool_use(&Model::Claude3Opus));
        assert!(!supports_tool_use(&Model::Llama38BInstruct));
    }

    #[test]
    fn test_supports_vision() {
        assert!(supports_vision(&Model::Claude3Opus));
        assert!(supports_vision(&Model::Llama3211BVisionInstruct));
        assert!(!supports_vision(&Model::Llama38BInstruct));
    }

    #[test]
    fn test_model_capabilities() {
        let caps = ModelCapabilities::for_model(&Model::Claude3Opus);
        assert!(caps.system_prompt);
        assert!(caps.tool_use);
        assert!(caps.streaming);
        assert!(caps.vision);
        assert!(caps.converse_api);

        let caps = ModelCapabilities::for_model(&Model::Mistral7BInstruct);
        assert!(!caps.system_prompt);
        assert!(!caps.tool_use);
        assert!(caps.streaming);
        assert!(!caps.vision);
        assert!(caps.converse_api);
    }
}

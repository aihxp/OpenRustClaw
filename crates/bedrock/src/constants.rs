//! Constants for AWS Bedrock.

use std::time::Duration;

/// Default timeout for API requests.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

/// Default maximum number of retries.
pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// Default retry delay.
pub const DEFAULT_RETRY_DELAY: Duration = Duration::from_millis(1000);

/// Maximum retry delay.
pub const MAX_RETRY_DELAY: Duration = Duration::from_millis(32000);

/// Backoff multiplier for retries.
pub const BACKOFF_MULTIPLIER: f64 = 2.0;

/// AWS Bedrock service name for SigV4 signing.
pub const BEDROCK_SERVICE_NAME: &str = "bedrock";

/// AWS Bedrock Runtime service name for SigV4 signing.
pub const BEDROCK_RUNTIME_SERVICE_NAME: &str = "bedrock-runtime";

/// AWS Bedrock Agent service name for SigV4 signing.
pub const BEDROCK_AGENT_SERVICE_NAME: &str = "bedrock-agent";

/// AWS Bedrock Agent Runtime service name for SigV4 signing.
pub const BEDROCK_AGENT_RUNTIME_SERVICE_NAME: &str = "bedrock-agent-runtime";

/// HTTP headers used by the SDK.
pub mod headers {
    /// Content type header.
    pub const CONTENT_TYPE: &str = "content-type";
    /// Accept header.
    pub const ACCEPT: &str = "accept";
    /// Amz SDK invocation ID.
    pub const AMZ_SDK_INVOCATION_ID: &str = "amz-sdk-invocation-id";
    /// Amz SDK request header.
    pub const AMZ_SDK_REQUEST: &str = "amz-sdk-request";
    /// Amzn trace ID header.
    pub const AMZN_TRACE_ID: &str = "x-amzn-trace-id";
    /// Amz Bedrock Guardrail ID.
    pub const AMZ_BEDROCK_GUARDRAIL_ID: &str = "x-amzn-bedrock-guardrailidentifier";
    /// Amz Bedrock Guardrail version.
    pub const AMZ_BEDROCK_GUARDRAIL_VERSION: &str = "x-amzn-bedrock-guardrailversion";
    /// Amz Bedrock Trace.
pub const AMZ_BEDROCK_TRACE: &str = "x-amzn-bedrock-trace";
}

/// API endpoints.
pub mod endpoints {
    /// Converse endpoint.
    pub const CONVERSE: &str = "/model/{modelId}/converse";
    /// Converse stream endpoint.
    pub const CONVERSE_STREAM: &str = "/model/{modelId}/converse-stream";
    /// Invoke model endpoint.
    pub const INVOKE_MODEL: &str = "/model/{modelId}/invoke";
    /// Invoke model with response stream endpoint.
    pub const INVOKE_MODEL_STREAM: &str = "/model/{modelId}/invoke-with-response-stream";
    /// List models endpoint.
    pub const LIST_MODELS: &str = "/foundation-models";
    /// Get model endpoint.
    pub const GET_MODEL: &str = "/foundation-models/{modelId}";
    /// List custom models endpoint.
    pub const LIST_CUSTOM_MODELS: &str = "/custom-models";
    /// Create agent endpoint.
    pub const CREATE_AGENT: &str = "/agents";
    /// Get agent endpoint.
    pub const GET_AGENT: &str = "/agents/{agentId}";
    /// Invoke agent endpoint.
    pub const INVOKE_AGENT: &str = "/agents/{agentId}/agentAliases/{agentAliasId}/sessions/{sessionId}/text";
    /// Retrieve knowledge base endpoint.
    pub const RETRIEVE_KNOWLEDGE: &str = "/knowledgebases/{knowledgeBaseId}/retrieve";
}

/// Model identifiers for Amazon Bedrock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Model {
    // Anthropic Claude Models
    /// Claude 3 Opus (most capable)
    Claude3Opus,
    /// Claude 3 Sonnet (balanced)
    Claude3Sonnet,
    /// Claude 3 Haiku (fastest)
    Claude3Haiku,
    /// Claude 3.5 Sonnet (latest)
    Claude35Sonnet,
    /// Claude 3.5 Haiku
    Claude35Haiku,
    /// Claude 3.5 Sonnet v2
    Claude35SonnetV2,
    /// Claude 3 Haiku 20240307
    Claude3Haiku20240307,

    // Amazon Titan Models
    /// Titan Text Express
    TitanTextExpress,
    /// Titan Text Lite
    TitanTextLite,
    /// Titan Text Premier
    TitanTextPremier,
    /// Titan Embeddings G1
    TitanEmbeddingsG1,
    /// Titan Embeddings G1 Text
    TitanEmbeddingsG1Text,
    /// Titan Image Generator G1
    TitanImageGeneratorG1,

    // Meta Llama Models
    /// Llama 2 13B
    Llama213B,
    /// Llama 2 70B
    Llama270B,
    /// Llama 3 8B Instruct
    Llama38BInstruct,
    /// Llama 3 70B Instruct
    Llama370BInstruct,
    /// Llama 3.1 8B Instruct
    Llama318BInstruct,
    /// Llama 3.1 70B Instruct
    Llama3170BInstruct,
    /// Llama 3.1 405B Instruct
    Llama31405BInstruct,
    /// Llama 3.2 1B Instruct
    Llama321BInstruct,
    /// Llama 3.2 3B Instruct
    Llama323BInstruct,
    /// Llama 3.2 11B Vision Instruct
    Llama3211BVisionInstruct,
    /// Llama 3.2 90B Vision Instruct
    Llama3290BVisionInstruct,

    // Mistral AI Models
    /// Mistral 7B Instruct
    Mistral7BInstruct,
    /// Mistral 8x7B Instruct
    Mistral8x7BInstruct,
    /// Mistral Large
    MistralLarge,
    /// Mistral Large 2 (24.07)
    MistralLarge2407,
    /// Mistral Small
    MistralSmall,

    // Cohere Models
    /// Cohere Command
    CohereCommand,
    /// Cohere Command Light
    CohereCommandLight,
    /// Cohere Command R
    CohereCommandR,
    /// Cohere Command R Plus
    CohereCommandRPlus,
    /// Cohere Embed English
    CohereEmbedEnglish,
    /// Cohere Embed Multilingual
    CohereEmbedMultilingual,

    // AI21 Labs Models
    /// Jurassic-2 Ultra
    Jurassic2Ultra,
    /// Jurassic-2 Mid
    Jurassic2Mid,
    /// Jamba-Instruct
    JambaInstruct,

    // Stability AI Models
    /// Stable Diffusion XL 0.x
    StableDiffusionXL0,
    /// Stable Diffusion XL 1.x
    StableDiffusionXL1,
    /// Stable Diffusion 3 Large
    StableDiffusion3Large,
    /// Stable Diffusion 3 Ultra
    StableDiffusion3Ultra,
    /// Stable Image Core
    StableImageCore,
    /// Stable Image Ultra
    StableImageUltra,
}

impl Model {
    /// Get the model identifier string for Bedrock API.
    pub fn as_str(&self) -> &'static str {
        match self {
            // Anthropic
            Model::Claude3Opus => "anthropic.claude-3-opus-20240229-v1:0",
            Model::Claude3Sonnet => "anthropic.claude-3-sonnet-20240229-v1:0",
            Model::Claude3Haiku => "anthropic.claude-3-haiku-20240307-v1:0",
            Model::Claude35Sonnet => "anthropic.claude-3-5-sonnet-20240620-v1:0",
            Model::Claude35Haiku => "anthropic.claude-3-5-haiku-20241022-v1:0",
            Model::Claude35SonnetV2 => "anthropic.claude-3-5-sonnet-20241022-v2:0",
            Model::Claude3Haiku20240307 => "anthropic.claude-3-haiku-20240307-v1:0",

            // Amazon
            Model::TitanTextExpress => "amazon.titan-text-express-v1",
            Model::TitanTextLite => "amazon.titan-text-lite-v1",
            Model::TitanTextPremier => "amazon.titan-text-premier-v1:0",
            Model::TitanEmbeddingsG1 => "amazon.titan-embeddings-g1-v1",
            Model::TitanEmbeddingsG1Text => "amazon.titan-embeddings-text-v1",
            Model::TitanImageGeneratorG1 => "amazon.titan-image-generator-v1",

            // Meta
            Model::Llama213B => "meta.llama2-13b-v1",
            Model::Llama270B => "meta.llama2-70b-v1",
            Model::Llama38BInstruct => "meta.llama3-8b-instruct-v1:0",
            Model::Llama370BInstruct => "meta.llama3-70b-instruct-v1:0",
            Model::Llama318BInstruct => "meta.llama3-1-8b-instruct-v1:0",
            Model::Llama3170BInstruct => "meta.llama3-1-70b-instruct-v1:0",
            Model::Llama31405BInstruct => "meta.llama3-1-405b-instruct-v1:0",
            Model::Llama321BInstruct => "meta.llama3-2-1b-instruct-v1:0",
            Model::Llama323BInstruct => "meta.llama3-2-3b-instruct-v1:0",
            Model::Llama3211BVisionInstruct => "meta.llama3-2-11b-vision-instruct-v1:0",
            Model::Llama3290BVisionInstruct => "meta.llama3-2-90b-vision-instruct-v1:0",

            // Mistral
            Model::Mistral7BInstruct => "mistral.mistral-7b-instruct-v0:2",
            Model::Mistral8x7BInstruct => "mistral.mixtral-8x7b-instruct-v0:1",
            Model::MistralLarge => "mistral.mistral-large-2402-v1:0",
            Model::MistralLarge2407 => "mistral.mistral-large-2407-v1:0",
            Model::MistralSmall => "mistral.mistral-small-2402-v1:0",

            // Cohere
            Model::CohereCommand => "cohere.command-text-v14",
            Model::CohereCommandLight => "cohere.command-light-text-v14",
            Model::CohereCommandR => "cohere.command-r-v1:0",
            Model::CohereCommandRPlus => "cohere.command-r-plus-v1:0",
            Model::CohereEmbedEnglish => "cohere.embed-english-v3",
            Model::CohereEmbedMultilingual => "cohere.embed-multilingual-v3",

            // AI21
            Model::Jurassic2Ultra => "ai21.j2-ultra-v1",
            Model::Jurassic2Mid => "ai21.j2-mid-v1",
            Model::JambaInstruct => "ai21.jamba-instruct-v1:0",

            // Stability AI
            Model::StableDiffusionXL0 => "stability.stable-diffusion-xl-v0",
            Model::StableDiffusionXL1 => "stability.stable-diffusion-xl-v1",
            Model::StableDiffusion3Large => "stability.sd3-large-v1:0",
            Model::StableDiffusion3Ultra => "stability.sd3-ultra-v1:0",
            Model::StableImageCore => "stability.stable-image-core-v1:0",
            Model::StableImageUltra => "stability.stable-image-ultra-v1:0",
        }
    }

    /// Get the provider of this model.
    pub fn provider(&self) -> ModelProvider {
        match self {
            Model::Claude3Opus
            | Model::Claude3Sonnet
            | Model::Claude3Haiku
            | Model::Claude35Sonnet
            | Model::Claude35Haiku
            | Model::Claude35SonnetV2
            | Model::Claude3Haiku20240307 => ModelProvider::Anthropic,

            Model::TitanTextExpress
            | Model::TitanTextLite
            | Model::TitanTextPremier
            | Model::TitanEmbeddingsG1
            | Model::TitanEmbeddingsG1Text
            | Model::TitanImageGeneratorG1 => ModelProvider::Amazon,

            Model::Llama213B
            | Model::Llama270B
            | Model::Llama38BInstruct
            | Model::Llama370BInstruct
            | Model::Llama318BInstruct
            | Model::Llama3170BInstruct
            | Model::Llama31405BInstruct
            | Model::Llama321BInstruct
            | Model::Llama323BInstruct
            | Model::Llama3211BVisionInstruct
            | Model::Llama3290BVisionInstruct => ModelProvider::Meta,

            Model::Mistral7BInstruct
            | Model::Mistral8x7BInstruct
            | Model::MistralLarge
            | Model::MistralLarge2407
            | Model::MistralSmall => ModelProvider::Mistral,

            Model::CohereCommand
            | Model::CohereCommandLight
            | Model::CohereCommandR
            | Model::CohereCommandRPlus
            | Model::CohereEmbedEnglish
            | Model::CohereEmbedMultilingual => ModelProvider::Cohere,

            Model::Jurassic2Ultra | Model::Jurassic2Mid | Model::JambaInstruct => {
                ModelProvider::Ai21
            }

            Model::StableDiffusionXL0
            | Model::StableDiffusionXL1
            | Model::StableDiffusion3Large
            | Model::StableDiffusion3Ultra
            | Model::StableImageCore
            | Model::StableImageUltra => ModelProvider::Stability,
        }
    }

    /// Get the modality supported by this model.
    pub fn modality(&self) -> ModelModality {
        match self {
            // Text-only models
            Model::TitanTextExpress
            | Model::TitanTextLite
            | Model::TitanTextPremier
            | Model::Llama213B
            | Model::Llama270B
            | Model::Llama38BInstruct
            | Model::Llama370BInstruct
            | Model::Llama318BInstruct
            | Model::Llama3170BInstruct
            | Model::Llama31405BInstruct
            | Model::Llama321BInstruct
            | Model::Llama323BInstruct
            | Model::Mistral7BInstruct
            | Model::Mistral8x7BInstruct
            | Model::MistralLarge
            | Model::MistralLarge2407
            | Model::MistralSmall
            | Model::CohereCommand
            | Model::CohereCommandLight
            | Model::CohereCommandR
            | Model::CohereCommandRPlus
            | Model::Jurassic2Ultra
            | Model::Jurassic2Mid
            | Model::JambaInstruct => ModelModality::Text,

            // Vision models
            Model::Claude3Opus
            | Model::Claude3Sonnet
            | Model::Claude3Haiku
            | Model::Claude35Sonnet
            | Model::Claude35Haiku
            | Model::Claude35SonnetV2
            | Model::Claude3Haiku20240307
            | Model::Llama3211BVisionInstruct
            | Model::Llama3290BVisionInstruct => ModelModality::VisionText,

            // Embedding models
            Model::TitanEmbeddingsG1 | Model::TitanEmbeddingsG1Text => ModelModality::Embedding,
            Model::CohereEmbedEnglish | Model::CohereEmbedMultilingual => ModelModality::Embedding,

            // Image generation models
            Model::TitanImageGeneratorG1
            | Model::StableDiffusionXL0
            | Model::StableDiffusionXL1
            | Model::StableDiffusion3Large
            | Model::StableDiffusion3Ultra
            | Model::StableImageCore
            | Model::StableImageUltra => ModelModality::Image,
        }
    }

    /// Check if this model supports the Converse API.
    pub fn supports_converse(&self) -> bool {
        matches!(
            self,
            Model::Claude3Opus
                | Model::Claude3Sonnet
                | Model::Claude3Haiku
                | Model::Claude35Sonnet
                | Model::Claude35Haiku
                | Model::Claude35SonnetV2
                | Model::Claude3Haiku20240307
                | Model::Llama38BInstruct
                | Model::Llama370BInstruct
                | Model::Llama318BInstruct
                | Model::Llama3170BInstruct
                | Model::Llama31405BInstruct
                | Model::Llama321BInstruct
                | Model::Llama323BInstruct
                | Model::Llama3211BVisionInstruct
                | Model::Llama3290BVisionInstruct
                | Model::Mistral7BInstruct
                | Model::Mistral8x7BInstruct
                | Model::MistralLarge
                | Model::MistralLarge2407
                | Model::MistralSmall
                | Model::CohereCommandR
                | Model::CohereCommandRPlus
        )
    }
}

impl std::fmt::Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Model {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // Anthropic
        if s.contains("claude-3-opus") {
            return Ok(Model::Claude3Opus);
        }
        if s.contains("claude-3-5-sonnet-20241022") || s.contains("claude-3-5-sonnet-v2") {
            return Ok(Model::Claude35SonnetV2);
        }
        if s.contains("claude-3-5-sonnet") {
            return Ok(Model::Claude35Sonnet);
        }
        if s.contains("claude-3-5-haiku") {
            return Ok(Model::Claude35Haiku);
        }
        if s.contains("claude-3-sonnet") {
            return Ok(Model::Claude3Sonnet);
        }
        if s.contains("claude-3-haiku") {
            return Ok(Model::Claude3Haiku);
        }

        // Amazon
        if s.contains("titan-text-express") {
            return Ok(Model::TitanTextExpress);
        }
        if s.contains("titan-text-lite") {
            return Ok(Model::TitanTextLite);
        }
        if s.contains("titan-text-premier") {
            return Ok(Model::TitanTextPremier);
        }
        if s.contains("titan-embeddings-g1") {
            return Ok(Model::TitanEmbeddingsG1);
        }
        if s.contains("titan-embeddings-text") {
            return Ok(Model::TitanEmbeddingsG1Text);
        }
        if s.contains("titan-image-generator") {
            return Ok(Model::TitanImageGeneratorG1);
        }

        // Meta
        if s.contains("llama2-13b") {
            return Ok(Model::Llama213B);
        }
        if s.contains("llama2-70b") {
            return Ok(Model::Llama270B);
        }
        if s.contains("llama3-2-90b") {
            return Ok(Model::Llama3290BVisionInstruct);
        }
        if s.contains("llama3-2-11b") {
            return Ok(Model::Llama3211BVisionInstruct);
        }
        if s.contains("llama3-2-3b") {
            return Ok(Model::Llama323BInstruct);
        }
        if s.contains("llama3-2-1b") {
            return Ok(Model::Llama321BInstruct);
        }
        if s.contains("llama3-1-405b") {
            return Ok(Model::Llama31405BInstruct);
        }
        if s.contains("llama3-1-70b") {
            return Ok(Model::Llama3170BInstruct);
        }
        if s.contains("llama3-1-8b") {
            return Ok(Model::Llama318BInstruct);
        }
        if s.contains("llama3-70b") {
            return Ok(Model::Llama370BInstruct);
        }
        if s.contains("llama3-8b") {
            return Ok(Model::Llama38BInstruct);
        }

        // Mistral
        if s.contains("mistral-large-2407") {
            return Ok(Model::MistralLarge2407);
        }
        if s.contains("mistral-large") {
            return Ok(Model::MistralLarge);
        }
        if s.contains("mistral-small") {
            return Ok(Model::MistralSmall);
        }
        if s.contains("mixtral-8x7b") {
            return Ok(Model::Mistral8x7BInstruct);
        }
        if s.contains("mistral-7b") {
            return Ok(Model::Mistral7BInstruct);
        }

        // Cohere
        if s.contains("command-r-plus") {
            return Ok(Model::CohereCommandRPlus);
        }
        if s.contains("command-r") {
            return Ok(Model::CohereCommandR);
        }
        if s.contains("command-light") {
            return Ok(Model::CohereCommandLight);
        }
        if s.contains("command-text") {
            return Ok(Model::CohereCommand);
        }
        if s.contains("embed-english") {
            return Ok(Model::CohereEmbedEnglish);
        }
        if s.contains("embed-multilingual") {
            return Ok(Model::CohereEmbedMultilingual);
        }

        // AI21
        if s.contains("j2-ultra") {
            return Ok(Model::Jurassic2Ultra);
        }
        if s.contains("j2-mid") {
            return Ok(Model::Jurassic2Mid);
        }
        if s.contains("jamba") {
            return Ok(Model::JambaInstruct);
        }

        // Stability
        if s.contains("stable-image-ultra") {
            return Ok(Model::StableImageUltra);
        }
        if s.contains("stable-image-core") {
            return Ok(Model::StableImageCore);
        }
        if s.contains("sd3-ultra") {
            return Ok(Model::StableDiffusion3Ultra);
        }
        if s.contains("sd3-large") {
            return Ok(Model::StableDiffusion3Large);
        }
        if s.contains("stable-diffusion-xl") {
            if s.contains("v1") {
                return Ok(Model::StableDiffusionXL1);
            }
            return Ok(Model::StableDiffusionXL0);
        }

        Err(format!("Unknown model identifier: {s}"))
    }
}

/// Model provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelProvider {
    /// Anthropic
    Anthropic,
    /// Amazon
    Amazon,
    /// Meta
    Meta,
    /// Mistral AI
    Mistral,
    /// Cohere
    Cohere,
    /// AI21 Labs
    Ai21,
    /// Stability AI
    Stability,
}

impl ModelProvider {
    /// Get the provider name.
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelProvider::Anthropic => "anthropic",
            ModelProvider::Amazon => "amazon",
            ModelProvider::Meta => "meta",
            ModelProvider::Mistral => "mistral",
            ModelProvider::Cohere => "cohere",
            ModelProvider::Ai21 => "ai21",
            ModelProvider::Stability => "stability",
        }
    }
}

impl std::fmt::Display for ModelProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Model modality (type of input/output supported).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelModality {
    /// Text-only model.
    Text,
    /// Vision and text model.
    VisionText,
    /// Embedding model.
    Embedding,
    /// Image generation model.
    Image,
}

/// Inference type for cross-region inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InferenceType {
    /// On-demand inference.
    OnDemand,
    /// Cross-region (optimized) inference.
    CrossRegion,
}

/// Cross-region inference profile.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InferenceProfile {
    /// US cross-region profile.
    Us,
    /// EU cross-region profile.
    Eu,
    /// Asia Pacific cross-region profile.
    Apac,
    /// Custom profile.
    Custom(String),
}

impl InferenceProfile {
    /// Get the profile identifier.
    pub fn as_str(&self) -> &str {
        match self {
            InferenceProfile::Us => "us",
            InferenceProfile::Eu => "eu",
            InferenceProfile::Apac => "apac",
            InferenceProfile::Custom(s) => s.as_str(),
        }
    }

    /// Create an inference profile ID for a model.
    pub fn create_profile_id(&self, model_id: &str) -> String {
        format!("{}.{}", self.as_str(), model_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_as_str() {
        assert_eq!(
            Model::Claude3Sonnet.as_str(),
            "anthropic.claude-3-sonnet-20240229-v1:0"
        );
        assert_eq!(Model::Llama38BInstruct.as_str(), "meta.llama3-8b-instruct-v1:0");
    }

    #[test]
    fn test_model_provider() {
        assert_eq!(Model::Claude3Opus.provider(), ModelProvider::Anthropic);
        assert_eq!(Model::Llama38BInstruct.provider(), ModelProvider::Meta);
        assert_eq!(Model::MistralLarge.provider(), ModelProvider::Mistral);
    }

    #[test]
    fn test_model_modality() {
        assert_eq!(Model::Claude3Opus.modality(), ModelModality::VisionText);
        assert_eq!(Model::Llama38BInstruct.modality(), ModelModality::Text);
        assert_eq!(
            Model::TitanImageGeneratorG1.modality(),
            ModelModality::Image
        );
    }

    #[test]
    fn test_model_from_str() {
        assert_eq!(
            "anthropic.claude-3-opus-20240229-v1:0".parse::<Model>().unwrap(),
            Model::Claude3Opus
        );
        assert_eq!(
            "meta.llama3-8b-instruct-v1:0".parse::<Model>().unwrap(),
            Model::Llama38BInstruct
        );
    }

    #[test]
    fn test_inference_profile() {
        let profile = InferenceProfile::Us;
        assert_eq!(profile.as_str(), "us");
        assert_eq!(
            profile.create_profile_id("anthropic.claude-3-sonnet-20240229-v1:0"),
            "us.anthropic.claude-3-sonnet-20240229-v1:0"
        );
    }
}

//! Constants for the AI21 API.

/// Default base URL for the AI21 API.
pub const DEFAULT_BASE_URL: &str = "https://api.ai21.com";

/// Default API version.
pub const DEFAULT_API_VERSION: &str = "v1";

/// API endpoints.
pub mod endpoints {
    /// Chat completions endpoint (Jamba models).
    pub const CHAT_COMPLETIONS: &str = "/studio/v1/chat/completions";

    /// Completions endpoint (Jurassic models).
    pub const COMPLETIONS: &str = "/studio/v1/completions";

    /// Contextual Answers endpoint (RAG).
    pub const CONTEXTUAL_ANSWERS: &str = "/studio/v1/contextualAnswers";

    /// Tokenize endpoint.
    pub const TOKENIZE: &str = "/studio/v1/tokenize";

    /// Detokenize endpoint.
    pub const DETOKENIZE: &str = "/studio/v1/detokenize";
}

/// Default retry configuration.
pub mod retry {
    /// Maximum number of retry attempts.
    pub const MAX_RETRIES: u32 = 3;

    /// Initial retry delay in milliseconds.
    pub const INITIAL_DELAY_MS: u64 = 1000;

    /// Maximum retry delay in milliseconds.
    pub const MAX_DELAY_MS: u64 = 32000;

    /// Exponential backoff multiplier.
    pub const BACKOFF_MULTIPLIER: f64 = 2.0;
}

/// HTTP header names.
pub mod headers {
    /// Authorization header.
    pub const AUTHORIZATION: &str = "Authorization";

    /// Content-Type header.
    pub const CONTENT_TYPE: &str = "Content-Type";

    /// Request ID header (for debugging).
    pub const REQUEST_ID: &str = "X-Request-ID";

    /// Client version header.
    pub const CLIENT_VERSION: &str = "X-Client-Version";
}

/// Jamba model identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JambaModel {
    /// Jamba 1.5 Large - Most capable Jamba model
    Jamba15Large,
    /// Jamba 1.5 Mini - Faster, cost-effective
    Jamba15Mini,
    /// Jamba Instruct - Instruction-tuned
    JambaInstruct,
}

impl JambaModel {
    /// Get the model identifier string.
    pub fn as_str(&self) -> &'static str {
        match self {
            JambaModel::Jamba15Large => "jamba-1.5-large",
            JambaModel::Jamba15Mini => "jamba-1.5-mini",
            JambaModel::JambaInstruct => "jamba-instruct",
        }
    }

    /// Get the maximum context window for this model.
    pub fn max_context_tokens(&self) -> usize {
        match self {
            JambaModel::Jamba15Large => 256_000,
            JambaModel::Jamba15Mini => 256_000,
            JambaModel::JambaInstruct => 256_000,
        }
    }

    /// Get the maximum output tokens for this model.
    pub fn max_output_tokens(&self) -> usize {
        match self {
            JambaModel::Jamba15Large => 4096,
            JambaModel::Jamba15Mini => 4096,
            JambaModel::JambaInstruct => 4096,
        }
    }
}

impl std::fmt::Display for JambaModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for JambaModel {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "jamba-1.5-large" => Ok(JambaModel::Jamba15Large),
            "jamba-1.5-mini" => Ok(JambaModel::Jamba15Mini),
            "jamba-instruct" => Ok(JambaModel::JambaInstruct),
            _ => Err(format!("Unknown Jamba model: {s}")),
        }
    }
}

/// Jurassic model identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JurassicModel {
    /// Jurassic-2 Ultra - Most capable
    J2Ultra,
    /// Jurassic-2 Mid - Balanced
    J2Mid,
    /// Jurassic-2 Light - Fastest
    J2Light,
}

impl JurassicModel {
    /// Get the model identifier string.
    pub fn as_str(&self) -> &'static str {
        match self {
            JurassicModel::J2Ultra => "j2-ultra",
            JurassicModel::J2Mid => "j2-mid",
            JurassicModel::J2Light => "j2-light",
        }
    }

    /// Get the maximum context window for this model.
    pub fn max_context_tokens(&self) -> usize {
        match self {
            JurassicModel::J2Ultra => 8192,
            JurassicModel::J2Mid => 8192,
            JurassicModel::J2Light => 8192,
        }
    }

    /// Get the maximum output tokens for this model.
    pub fn max_output_tokens(&self) -> usize {
        match self {
            JurassicModel::J2Ultra => 4096,
            JurassicModel::J2Mid => 4096,
            JurassicModel::J2Light => 4096,
        }
    }
}

impl std::fmt::Display for JurassicModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for JurassicModel {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "j2-ultra" => Ok(JurassicModel::J2Ultra),
            "j2-mid" => Ok(JurassicModel::J2Mid),
            "j2-light" => Ok(JurassicModel::J2Light),
            _ => Err(format!("Unknown Jurassic model: {s}")),
        }
    }
}

/// All AI21 model identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Model {
    /// Jamba model
    Jamba(JambaModel),
    /// Jurassic model
    Jurassic(JurassicModel),
}

impl Model {
    /// Get the model identifier string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Model::Jamba(m) => m.as_str(),
            Model::Jurassic(m) => m.as_str(),
        }
    }

    /// Get the maximum context window for this model.
    pub fn max_context_tokens(&self) -> usize {
        match self {
            Model::Jamba(m) => m.max_context_tokens(),
            Model::Jurassic(m) => m.max_context_tokens(),
        }
    }

    /// Get the maximum output tokens for this model.
    pub fn max_output_tokens(&self) -> usize {
        match self {
            Model::Jamba(m) => m.max_output_tokens(),
            Model::Jurassic(m) => m.max_output_tokens(),
        }
    }

    /// Check if this is a Jamba model.
    pub fn is_jamba(&self) -> bool {
        matches!(self, Model::Jamba(_))
    }

    /// Check if this is a Jurassic model.
    pub fn is_jurassic(&self) -> bool {
        matches!(self, Model::Jurassic(_))
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
        // Try Jamba models first
        if let Ok(jamba) = s.parse::<JambaModel>() {
            return Ok(Model::Jamba(jamba));
        }
        // Try Jurassic models
        if let Ok(jurassic) = s.parse::<JurassicModel>() {
            return Ok(Model::Jurassic(jurassic));
        }
        Err(format!("Unknown AI21 model: {s}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jamba_model_as_str() {
        assert_eq!(JambaModel::Jamba15Large.as_str(), "jamba-1.5-large");
        assert_eq!(JambaModel::Jamba15Mini.as_str(), "jamba-1.5-mini");
        assert_eq!(JambaModel::JambaInstruct.as_str(), "jamba-instruct");
    }

    #[test]
    fn test_jurassic_model_as_str() {
        assert_eq!(JurassicModel::J2Ultra.as_str(), "j2-ultra");
        assert_eq!(JurassicModel::J2Mid.as_str(), "j2-mid");
        assert_eq!(JurassicModel::J2Light.as_str(), "j2-light");
    }

    #[test]
    fn test_model_from_str() {
        assert!(matches!(
            "jamba-1.5-large".parse::<Model>().unwrap(),
            Model::Jamba(JambaModel::Jamba15Large)
        ));
        assert!(matches!(
            "j2-ultra".parse::<Model>().unwrap(),
            Model::Jurassic(JurassicModel::J2Ultra)
        ));
    }

    #[test]
    fn test_model_display() {
        assert_eq!(JambaModel::Jamba15Large.to_string(), "jamba-1.5-large");
        assert_eq!(JurassicModel::J2Ultra.to_string(), "j2-ultra");
    }
}

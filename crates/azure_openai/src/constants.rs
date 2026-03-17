//! Constants for the Azure OpenAI API.

use serde::{Deserialize, Serialize};

/// Default Azure OpenAI API version.
pub const DEFAULT_API_VERSION: &str = "2024-06-01";

/// Alternative stable API versions.
pub mod api_versions {
    /// GA API version (June 2024).
    pub const V2024_06_01: &str = "2024-06-01";
    /// Preview API version (October 2024).
    pub const V2024_10_01_PREVIEW: &str = "2024-10-01-preview";
    /// Preview API version (December 2024).
    pub const V2024_12_01_PREVIEW: &str = "2024-12-01-preview";
    /// Preview API version (February 2025).
    pub const V2025_02_01_PREVIEW: &str = "2025-02-01-preview";
}

/// Base URL template for Azure OpenAI.
/// Format: `https://{resource_name}.openai.azure.com`
pub const AZURE_BASE_URL_TEMPLATE: &str = "https://{}.openai.azure.com";

/// Build the base URL for an Azure OpenAI resource.
pub fn build_base_url(resource_name: &str) -> String {
    format!("https://{}.openai.azure.com", resource_name)
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

/// Azure OpenAI model deployments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AzureOpenAIModel {
    /// GPT-4o
    Gpt4O,
    /// GPT-4o mini
    Gpt4OMini,
    /// GPT-4 Turbo
    Gpt4Turbo,
    /// GPT-4
    Gpt4,
    /// GPT-3.5 Turbo
    Gpt35Turbo,
    /// GPT-3.5 Turbo 16K
    Gpt35Turbo16K,
    /// Text Embedding 3 Small
    TextEmbedding3Small,
    /// Text Embedding 3 Large
    TextEmbedding3Large,
    /// Text Embedding Ada 002
    TextEmbeddingAda002,
    /// DALL-E 3
    DallE3,
    /// DALL-E 2
    DallE2,
    /// Whisper
    Whisper,
    /// Text-to-Speech HD
    TtsHd,
    /// Text-to-Speech Standard
    Tts,
}

impl AzureOpenAIModel {
    /// Get the model identifier string.
    pub fn as_str(&self) -> &'static str {
        match self {
            AzureOpenAIModel::Gpt4O => "gpt-4o",
            AzureOpenAIModel::Gpt4OMini => "gpt-4o-mini",
            AzureOpenAIModel::Gpt4Turbo => "gpt-4-turbo",
            AzureOpenAIModel::Gpt4 => "gpt-4",
            AzureOpenAIModel::Gpt35Turbo => "gpt-35-turbo",
            AzureOpenAIModel::Gpt35Turbo16K => "gpt-35-turbo-16k",
            AzureOpenAIModel::TextEmbedding3Small => "text-embedding-3-small",
            AzureOpenAIModel::TextEmbedding3Large => "text-embedding-3-large",
            AzureOpenAIModel::TextEmbeddingAda002 => "text-embedding-ada-002",
            AzureOpenAIModel::DallE3 => "dall-e-3",
            AzureOpenAIModel::DallE2 => "dall-e-2",
            AzureOpenAIModel::Whisper => "whisper",
            AzureOpenAIModel::TtsHd => "tts-hd",
            AzureOpenAIModel::Tts => "tts",
        }
    }

    /// Get the maximum context window for this model.
    pub fn max_context_tokens(&self) -> usize {
        match self {
            AzureOpenAIModel::Gpt4O => 128_000,
            AzureOpenAIModel::Gpt4OMini => 128_000,
            AzureOpenAIModel::Gpt4Turbo => 128_000,
            AzureOpenAIModel::Gpt4 => 8_192,
            AzureOpenAIModel::Gpt35Turbo => 4_096,
            AzureOpenAIModel::Gpt35Turbo16K => 16_384,
            AzureOpenAIModel::TextEmbedding3Small => 8_191,
            AzureOpenAIModel::TextEmbedding3Large => 8_191,
            AzureOpenAIModel::TextEmbeddingAda002 => 8_191,
            AzureOpenAIModel::DallE3 => 0,
            AzureOpenAIModel::DallE2 => 0,
            AzureOpenAIModel::Whisper => 0,
            AzureOpenAIModel::TtsHd => 0,
            AzureOpenAIModel::Tts => 0,
        }
    }

    /// Check if this is an embedding model.
    pub fn is_embedding(&self) -> bool {
        matches!(
            self,
            AzureOpenAIModel::TextEmbedding3Small
                | AzureOpenAIModel::TextEmbedding3Large
                | AzureOpenAIModel::TextEmbeddingAda002
        )
    }

    /// Check if this is a chat model.
    pub fn is_chat(&self) -> bool {
        matches!(
            self,
            AzureOpenAIModel::Gpt4O
                | AzureOpenAIModel::Gpt4OMini
                | AzureOpenAIModel::Gpt4Turbo
                | AzureOpenAIModel::Gpt4
                | AzureOpenAIModel::Gpt35Turbo
                | AzureOpenAIModel::Gpt35Turbo16K
        )
    }

    /// Check if this is an image generation model.
    pub fn is_image(&self) -> bool {
        matches!(self, AzureOpenAIModel::DallE3 | AzureOpenAIModel::DallE2)
    }

    /// Check if this is an audio model.
    pub fn is_audio(&self) -> bool {
        matches!(
            self,
            AzureOpenAIModel::Whisper | AzureOpenAIModel::TtsHd | AzureOpenAIModel::Tts
        )
    }
}

impl std::fmt::Display for AzureOpenAIModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for AzureOpenAIModel {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "gpt-4o" => Ok(AzureOpenAIModel::Gpt4O),
            "gpt-4o-mini" => Ok(AzureOpenAIModel::Gpt4OMini),
            "gpt-4-turbo" | "gpt-4-turbo-preview" => Ok(AzureOpenAIModel::Gpt4Turbo),
            "gpt-4" => Ok(AzureOpenAIModel::Gpt4),
            "gpt-35-turbo" | "gpt-3.5-turbo" => Ok(AzureOpenAIModel::Gpt35Turbo),
            "gpt-35-turbo-16k" | "gpt-3.5-turbo-16k" => Ok(AzureOpenAIModel::Gpt35Turbo16K),
            "text-embedding-3-small" => Ok(AzureOpenAIModel::TextEmbedding3Small),
            "text-embedding-3-large" => Ok(AzureOpenAIModel::TextEmbedding3Large),
            "text-embedding-ada-002" => Ok(AzureOpenAIModel::TextEmbeddingAda002),
            "dall-e-3" => Ok(AzureOpenAIModel::DallE3),
            "dall-e-2" => Ok(AzureOpenAIModel::DallE2),
            "whisper" => Ok(AzureOpenAIModel::Whisper),
            "tts-hd" => Ok(AzureOpenAIModel::TtsHd),
            "tts" => Ok(AzureOpenAIModel::Tts),
            _ => Err(format!("Unknown model: {s}")),
        }
    }
}

/// Azure regions for OpenAI deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AzureRegion {
    /// East US
    EastUS,
    /// East US 2
    EastUS2,
    /// North Central US
    NorthCentralUS,
    /// South Central US
    SouthCentralUS,
    /// West US
    WestUS,
    /// West US 2
    WestUS2,
    /// West US 3
    WestUS3,
    /// Canada Central
    CanadaCentral,
    /// Canada East
    CanadaEast,
    /// Brazil South
    BrazilSouth,
    /// North Europe
    NorthEurope,
    /// West Europe
    WestEurope,
    /// UK South
    UKSouth,
    /// UK West
    UKWest,
    /// France Central
    FranceCentral,
    /// Germany West Central
    GermanyWestCentral,
    /// Norway East
    NorwayEast,
    /// Poland Central
    PolandCentral,
    /// Sweden Central
    SwedenCentral,
    /// Switzerland North
    SwitzerlandNorth,
    /// UAE North
    UAENorth,
    /// South Africa North
    SouthAfricaNorth,
    /// East Asia
    EastAsia,
    /// Southeast Asia
    SoutheastAsia,
    /// Australia East
    AustraliaEast,
    /// Australia Southeast
    AustraliaSoutheast,
    /// Japan East
    JapanEast,
    /// Japan West
    JapanWest,
    /// Korea Central
    KoreaCentral,
    /// Central India
    CentralIndia,
    /// Jio India West
    JioIndiaWest,
    /// Qatar Central
    QatarCentral,
    /// Israel Central
    IsraelCentral,
    /// Italy North
    ItalyNorth,
    /// Spain Central
    SpainCentral,
    /// Mexico Central
    MexicoCentral,
    /// New Zealand North
    NewZealandNorth,
    /// Taiwan North
    TaiwanNorth,
    /// Korea South
    KoreaSouth,
    /// South India
    SouthIndia,
}

impl AzureRegion {
    /// Get the region identifier string.
    pub fn as_str(&self) -> &'static str {
        match self {
            AzureRegion::EastUS => "eastus",
            AzureRegion::EastUS2 => "eastus2",
            AzureRegion::NorthCentralUS => "northcentralus",
            AzureRegion::SouthCentralUS => "southcentralus",
            AzureRegion::WestUS => "westus",
            AzureRegion::WestUS2 => "westus2",
            AzureRegion::WestUS3 => "westus3",
            AzureRegion::CanadaCentral => "canadacentral",
            AzureRegion::CanadaEast => "canadaeast",
            AzureRegion::BrazilSouth => "brazilsouth",
            AzureRegion::NorthEurope => "northeurope",
            AzureRegion::WestEurope => "westeurope",
            AzureRegion::UKSouth => "uksouth",
            AzureRegion::UKWest => "ukwest",
            AzureRegion::FranceCentral => "francecentral",
            AzureRegion::GermanyWestCentral => "germanywestcentral",
            AzureRegion::NorwayEast => "norwayeast",
            AzureRegion::PolandCentral => "polandcentral",
            AzureRegion::SwedenCentral => "swedencentral",
            AzureRegion::SwitzerlandNorth => "switzerlandnorth",
            AzureRegion::UAENorth => "uaenorth",
            AzureRegion::SouthAfricaNorth => "southafricanorth",
            AzureRegion::EastAsia => "eastasia",
            AzureRegion::SoutheastAsia => "southeastasia",
            AzureRegion::AustraliaEast => "australiaeast",
            AzureRegion::AustraliaSoutheast => "australiasoutheast",
            AzureRegion::JapanEast => "japaneast",
            AzureRegion::JapanWest => "japanwest",
            AzureRegion::KoreaCentral => "koreacentral",
            AzureRegion::CentralIndia => "centralindia",
            AzureRegion::JioIndiaWest => "jioindiawest",
            AzureRegion::QatarCentral => "qatarcentral",
            AzureRegion::IsraelCentral => "israelcentral",
            AzureRegion::ItalyNorth => "italynorth",
            AzureRegion::SpainCentral => "spaincentral",
            AzureRegion::MexicoCentral => "mexicocentral",
            AzureRegion::NewZealandNorth => "newzealandnorth",
            AzureRegion::TaiwanNorth => "taiwannorth",
            AzureRegion::KoreaSouth => "koreasouth",
            AzureRegion::SouthIndia => "southindia",
        }
    }

    /// Get the region name for display.
    pub fn display_name(&self) -> &'static str {
        match self {
            AzureRegion::EastUS => "East US",
            AzureRegion::EastUS2 => "East US 2",
            AzureRegion::NorthCentralUS => "North Central US",
            AzureRegion::SouthCentralUS => "South Central US",
            AzureRegion::WestUS => "West US",
            AzureRegion::WestUS2 => "West US 2",
            AzureRegion::WestUS3 => "West US 3",
            AzureRegion::CanadaCentral => "Canada Central",
            AzureRegion::CanadaEast => "Canada East",
            AzureRegion::BrazilSouth => "Brazil South",
            AzureRegion::NorthEurope => "North Europe",
            AzureRegion::WestEurope => "West Europe",
            AzureRegion::UKSouth => "UK South",
            AzureRegion::UKWest => "UK West",
            AzureRegion::FranceCentral => "France Central",
            AzureRegion::GermanyWestCentral => "Germany West Central",
            AzureRegion::NorwayEast => "Norway East",
            AzureRegion::PolandCentral => "Poland Central",
            AzureRegion::SwedenCentral => "Sweden Central",
            AzureRegion::SwitzerlandNorth => "Switzerland North",
            AzureRegion::UAENorth => "UAE North",
            AzureRegion::SouthAfricaNorth => "South Africa North",
            AzureRegion::EastAsia => "East Asia",
            AzureRegion::SoutheastAsia => "Southeast Asia",
            AzureRegion::AustraliaEast => "Australia East",
            AzureRegion::AustraliaSoutheast => "Australia Southeast",
            AzureRegion::JapanEast => "Japan East",
            AzureRegion::JapanWest => "Japan West",
            AzureRegion::KoreaCentral => "Korea Central",
            AzureRegion::CentralIndia => "Central India",
            AzureRegion::JioIndiaWest => "Jio India West",
            AzureRegion::QatarCentral => "Qatar Central",
            AzureRegion::IsraelCentral => "Israel Central",
            AzureRegion::ItalyNorth => "Italy North",
            AzureRegion::SpainCentral => "Spain Central",
            AzureRegion::MexicoCentral => "Mexico Central",
            AzureRegion::NewZealandNorth => "New Zealand North",
            AzureRegion::TaiwanNorth => "Taiwan North",
            AzureRegion::KoreaSouth => "Korea South",
            AzureRegion::SouthIndia => "South India",
        }
    }
}

impl std::fmt::Display for AzureRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for AzureRegion {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "eastus" | "east-us" => Ok(AzureRegion::EastUS),
            "eastus2" | "east-us-2" => Ok(AzureRegion::EastUS2),
            "northcentralus" | "north-central-us" => Ok(AzureRegion::NorthCentralUS),
            "southcentralus" | "south-central-us" => Ok(AzureRegion::SouthCentralUS),
            "westus" | "west-us" => Ok(AzureRegion::WestUS),
            "westus2" | "west-us-2" => Ok(AzureRegion::WestUS2),
            "westus3" | "west-us-3" => Ok(AzureRegion::WestUS3),
            "canadacentral" | "canada-central" => Ok(AzureRegion::CanadaCentral),
            "canadaeast" | "canada-east" => Ok(AzureRegion::CanadaEast),
            "brazilsouth" | "brazil-south" => Ok(AzureRegion::BrazilSouth),
            "northeurope" | "north-europe" => Ok(AzureRegion::NorthEurope),
            "westeurope" | "west-europe" => Ok(AzureRegion::WestEurope),
            "uksouth" | "uk-south" => Ok(AzureRegion::UKSouth),
            "ukwest" | "uk-west" => Ok(AzureRegion::UKWest),
            "francecentral" | "france-central" => Ok(AzureRegion::FranceCentral),
            "germanywestcentral" | "germany-west-central" => Ok(AzureRegion::GermanyWestCentral),
            "norwayeast" | "norway-east" => Ok(AzureRegion::NorwayEast),
            "polandcentral" | "poland-central" => Ok(AzureRegion::PolandCentral),
            "swedencentral" | "sweden-central" => Ok(AzureRegion::SwedenCentral),
            "switzerlandnorth" | "switzerland-north" => Ok(AzureRegion::SwitzerlandNorth),
            "uaenorth" | "uae-north" => Ok(AzureRegion::UAENorth),
            "southafricanorth" | "south-africa-north" => Ok(AzureRegion::SouthAfricaNorth),
            "eastasia" | "east-asia" => Ok(AzureRegion::EastAsia),
            "southeastasia" | "southeast-asia" => Ok(AzureRegion::SoutheastAsia),
            "australiaeast" | "australia-east" => Ok(AzureRegion::AustraliaEast),
            "australiasoutheast" | "australia-southeast" => Ok(AzureRegion::AustraliaSoutheast),
            "japaneast" | "japan-east" => Ok(AzureRegion::JapanEast),
            "japanwest" | "japan-west" => Ok(AzureRegion::JapanWest),
            "koreacentral" | "korea-central" => Ok(AzureRegion::KoreaCentral),
            "centralindia" | "central-india" => Ok(AzureRegion::CentralIndia),
            "jioindiawest" | "jio-india-west" => Ok(AzureRegion::JioIndiaWest),
            "qatarcentral" | "qatar-central" => Ok(AzureRegion::QatarCentral),
            "israelcentral" | "israel-central" => Ok(AzureRegion::IsraelCentral),
            "italynorth" | "italy-north" => Ok(AzureRegion::ItalyNorth),
            "spaincentral" | "spain-central" => Ok(AzureRegion::SpainCentral),
            "mexicocentral" | "mexico-central" => Ok(AzureRegion::MexicoCentral),
            "newzealandnorth" | "new-zealand-north" => Ok(AzureRegion::NewZealandNorth),
            "taiwannorth" | "taiwan-north" => Ok(AzureRegion::TaiwanNorth),
            "koreasouth" | "korea-south" => Ok(AzureRegion::KoreaSouth),
            "southindia" | "south-india" => Ok(AzureRegion::SouthIndia),
            _ => Err(format!("Unknown Azure region: {s}")),
        }
    }
}

/// API endpoints.
pub mod endpoints {
    /// Chat completions endpoint template.
    /// Format: `/openai/deployments/{deployment_name}/chat/completions`
    pub const CHAT_COMPLETIONS: &str = "/openai/deployments/{}/chat/completions";

    /// Completions endpoint template.
    /// Format: `/openai/deployments/{deployment_name}/completions`
    pub const COMPLETIONS: &str = "/openai/deployments/{}/completions";

    /// Embeddings endpoint template.
    /// Format: `/openai/deployments/{deployment_name}/embeddings`
    pub const EMBEDDINGS: &str = "/openai/deployments/{}/embeddings";

    /// Images generations endpoint template.
    /// Format: `/openai/deployments/{deployment_name}/images/generations`
    pub const IMAGE_GENERATIONS: &str = "/openai/deployments/{}/images/generations";

    /// Audio transcription endpoint template.
    /// Format: `/openai/deployments/{deployment_name}/audio/transcriptions`
    pub const AUDIO_TRANSCRIPTIONS: &str = "/openai/deployments/{}/audio/transcriptions";

    /// Audio translation endpoint template.
    /// Format: `/openai/deployments/{deployment_name}/audio/translations`
    pub const AUDIO_TRANSLATIONS: &str = "/openai/deployments/{}/audio/translations";

    /// Audio speech endpoint template.
    /// Format: `/openai/deployments/{deployment_name}/audio/speech`
    pub const AUDIO_SPEECH: &str = "/openai/deployments/{}/audio/speech";

    /// Assistants endpoint.
    pub const ASSISTANTS: &str = "/openai/assistants";

    /// Threads endpoint.
    pub const THREADS: &str = "/openai/threads";

    /// Batch endpoint.
    pub const BATCH: &str = "/openai/batches";

    /// Files endpoint.
    pub const FILES: &str = "/openai/files";

    /// Fine-tuning endpoint.
    pub const FINE_TUNING: &str = "/openai/fine_tuning/jobs";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_as_str() {
        assert_eq!(AzureOpenAIModel::Gpt4O.as_str(), "gpt-4o");
        assert_eq!(AzureOpenAIModel::Gpt35Turbo.as_str(), "gpt-35-turbo");
    }

    #[test]
    fn test_model_from_str() {
        assert_eq!(
            "gpt-4o".parse::<AzureOpenAIModel>().unwrap(),
            AzureOpenAIModel::Gpt4O
        );
        assert_eq!(
            "gpt-35-turbo".parse::<AzureOpenAIModel>().unwrap(),
            AzureOpenAIModel::Gpt35Turbo
        );
    }

    #[test]
    fn test_region_display() {
        assert_eq!(AzureRegion::EastUS.as_str(), "eastus");
        assert_eq!(AzureRegion::WestEurope.display_name(), "West Europe");
    }

    #[test]
    fn test_build_base_url() {
        let url = build_base_url("my-resource");
        assert_eq!(url, "https://my-resource.openai.azure.com");
    }
}

//! Content Safety API for Azure OpenAI.
//!
//! This module provides access to Azure Content Safety integration,
//! including content filtering results and standalone content safety checks.

use secrecy::ExposeSecret;

use crate::client::AzureOpenAIClient;
use crate::error::Result;

/// Client for the content safety API.
#[derive(Debug)]
pub struct ContentSafety<'a> {
    client: &'a AzureOpenAIClient,
}

impl<'a> ContentSafety<'a> {
    /// Create a new content safety client.
    pub fn new(client: &'a AzureOpenAIClient) -> Self {
        Self { client }
    }

    /// Analyze text for harmful content.
    ///
    /// Note: This uses Azure AI Content Safety service, which requires
    /// a separate endpoint from Azure OpenAI.
    pub async fn analyze_text(&self, request: TextAnalysisRequest) -> Result<TextAnalysisResponse> {
        // Content Safety API uses a different endpoint
        // The base URL should be configured separately
        let content_safety_endpoint = self
            .client
            .config()
            .region
            .map(|r| {
                format!(
                    "https://{}.api.cognitive.microsoft.com/contentsafety",
                    r.as_str()
                )
            })
            .unwrap_or_else(|| {
                // Fallback to using the OpenAI resource
                format!(
                    "https://{}.cognitiveservices.azure.com/contentsafety",
                    self.client.resource_name()
                )
            });

        let url = format!(
            "{}/text:analyze?api-version=2023-10-01",
            content_safety_endpoint
        );

        let body = serde_json::to_value(&request)?;

        let mut request_builder = self.client.http().post(&url);

        // Add authentication headers
        if self.client.is_azure_ad() {
            let auth_header = self.client.config().authorization_header().await?;
            request_builder = request_builder.header("Authorization", auth_header);
        } else {
            if let crate::AzureCredential::ApiKey(key) = &self.client.config().credential {
                request_builder =
                    request_builder.header("Ocp-Apim-Subscription-Key", key.expose_secret());
            }
        }

        let response = request_builder
            .json(&body)
            .send()
            .await
            .map_err(crate::error::AzureOpenAIError::from)?;

        if !response.status().is_success() {
            return Err(crate::error::AzureOpenAIError::from_response(response).await);
        }

        let analysis_response: TextAnalysisResponse = response
            .json()
            .await
            .map_err(|e| crate::error::AzureOpenAIError::Http { source: e })?;

        Ok(analysis_response)
    }

    /// Check if content is safe based on severity thresholds.
    pub fn is_content_safe(response: &TextAnalysisResponse, thresholds: &SafetyThresholds) -> bool {
        if let Some(categories) = &response.categories_analysis {
            for category in categories {
                let threshold = match category.category.as_str() {
                    "Hate" => thresholds.hate,
                    "SelfHarm" => thresholds.self_harm,
                    "Sexual" => thresholds.sexual,
                    "Violence" => thresholds.violence,
                    _ => 0,
                };

                if category.severity > threshold {
                    return false;
                }
            }
        }
        true
    }
}

/// Text analysis request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextAnalysisRequest {
    /// The text to analyze.
    pub text: String,
    /// The categories to analyze.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<HarmCategory>>,
    /// The blocklist names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocklist_names: Option<Vec<String>>,
    /// Whether to halt on blocklist hit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub halt_on_blocklist_hit: Option<bool>,
    /// The output type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_type: Option<OutputType>,
}

impl TextAnalysisRequest {
    /// Create a new text analysis request.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            categories: None,
            blocklist_names: None,
            halt_on_blocklist_hit: None,
            output_type: None,
        }
    }

    /// Set the categories to analyze.
    pub fn categories(mut self, categories: Vec<HarmCategory>) -> Self {
        self.categories = Some(categories);
        self
    }

    /// Set the blocklist names.
    pub fn blocklist_names(mut self, names: Vec<String>) -> Self {
        self.blocklist_names = Some(names);
        self
    }

    /// Set whether to halt on blocklist hit.
    pub fn halt_on_blocklist_hit(mut self, halt: bool) -> Self {
        self.halt_on_blocklist_hit = Some(halt);
        self
    }

    /// Set the output type.
    pub fn output_type(mut self, output_type: OutputType) -> Self {
        self.output_type = Some(output_type);
        self
    }
}

/// Harm category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum HarmCategory {
    /// Hate speech.
    Hate,
    /// Self-harm.
    SelfHarm,
    /// Sexual content.
    Sexual,
    /// Violence.
    Violence,
}

/// Output type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputType {
    /// Four severity levels.
    FourSeverityLevels,
    /// Eight severity levels.
    EightSeverityLevels,
}

/// Text analysis response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextAnalysisResponse {
    /// The blocklists match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocklists_match: Option<Vec<BlocklistMatch>>,
    /// The categories analysis.
    #[serde(rename = "categoriesAnalysis", skip_serializing_if = "Option::is_none")]
    pub categories_analysis: Option<Vec<CategoryAnalysis>>,
}

/// Blocklist match.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlocklistMatch {
    /// The blocklist name.
    #[serde(rename = "blocklistName")]
    pub blocklist_name: String,
    /// The blocklist item ID.
    #[serde(rename = "blocklistItemId")]
    pub blocklist_item_id: String,
    /// The blocklist item text.
    #[serde(rename = "blocklistItemText")]
    pub blocklist_item_text: String,
}

/// Category analysis.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CategoryAnalysis {
    /// The category.
    pub category: String,
    /// The severity level.
    pub severity: i32,
}

/// Safety thresholds for content filtering.
#[derive(Debug, Clone, Copy)]
pub struct SafetyThresholds {
    /// Threshold for hate speech (0-4, where 0 is most restrictive).
    pub hate: i32,
    /// Threshold for self-harm.
    pub self_harm: i32,
    /// Threshold for sexual content.
    pub sexual: i32,
    /// Threshold for violence.
    pub violence: i32,
}

impl SafetyThresholds {
    /// Create new safety thresholds.
    pub fn new(hate: i32, self_harm: i32, sexual: i32, violence: i32) -> Self {
        Self {
            hate,
            self_harm,
            sexual,
            violence,
        }
    }

    /// Most restrictive thresholds (block all harmful content).
    pub fn most_restrictive() -> Self {
        Self::new(0, 0, 0, 0)
    }

    /// Moderate thresholds.
    pub fn moderate() -> Self {
        Self::new(2, 2, 2, 2)
    }

    /// Permissive thresholds (only block high severity).
    pub fn permissive() -> Self {
        Self::new(3, 3, 3, 3)
    }
}

impl Default for SafetyThresholds {
    fn default() -> Self {
        Self::moderate()
    }
}

/// Content filter result from Azure OpenAI.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContentFilterResult {
    /// Whether the content was filtered.
    pub filtered: bool,
    /// The severity level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
}

/// Content filter details from Azure OpenAI response.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ContentFilterDetails {
    /// Hate speech filter result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hate: Option<ContentFilterResult>,
    /// Self-harm filter result.
    #[serde(rename = "self_harm", skip_serializing_if = "Option::is_none")]
    pub self_harm: Option<ContentFilterResult>,
    /// Sexual content filter result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sexual: Option<ContentFilterResult>,
    /// Violence filter result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violence: Option<ContentFilterResult>,
    /// Profanity filter result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profanity: Option<ContentFilterResult>,
}

impl ContentFilterDetails {
    /// Check if any content was filtered.
    pub fn was_filtered(&self) -> bool {
        self.hate.as_ref().map(|r| r.filtered).unwrap_or(false)
            || self.self_harm.as_ref().map(|r| r.filtered).unwrap_or(false)
            || self.sexual.as_ref().map(|r| r.filtered).unwrap_or(false)
            || self.violence.as_ref().map(|r| r.filtered).unwrap_or(false)
            || self.profanity.as_ref().map(|r| r.filtered).unwrap_or(false)
    }

    /// Get the highest severity level found.
    pub fn highest_severity(&self) -> Option<SeverityLevel> {
        let mut highest = None;

        for result in [&self.hate, &self.self_harm, &self.sexual, &self.violence]
            .iter()
            .filter_map(|&r| r.as_ref())
        {
            if let Some(severity) = &result.severity {
                let level = SeverityLevel::from_str(severity);
                if highest.is_none() || level > highest.unwrap() {
                    highest = Some(level);
                }
            }
        }

        highest
    }
}

/// Severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SeverityLevel {
    /// Safe content.
    Safe,
    /// Low severity.
    Low,
    /// Medium severity.
    Medium,
    /// High severity.
    High,
}

impl SeverityLevel {
    /// Parse severity from string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "safe" => SeverityLevel::Safe,
            "low" => SeverityLevel::Low,
            "medium" => SeverityLevel::Medium,
            "high" => SeverityLevel::High,
            _ => SeverityLevel::Safe,
        }
    }
}

/// Utility functions for content safety.
pub mod utils {
    use super::*;

    /// Check if content is safe based on Azure OpenAI filter results.
    pub fn check_openai_filter_safe(filter_results: &ContentFilterDetails) -> bool {
        !filter_results.was_filtered()
    }

    /// Check if content requires review.
    pub fn requires_review(filter_results: &ContentFilterDetails) -> bool {
        if let Some(severity) = filter_results.highest_severity() {
            severity >= SeverityLevel::Medium
        } else {
            false
        }
    }

    /// Format a content filter report.
    pub fn format_filter_report(filter_results: &ContentFilterDetails) -> String {
        let mut report = Vec::new();

        if let Some(hate) = &filter_results.hate {
            if hate.filtered {
                report.push(format!(
                    "Hate: filtered (severity: {})",
                    hate.severity.as_deref().unwrap_or("unknown")
                ));
            }
        }

        if let Some(self_harm) = &filter_results.self_harm {
            if self_harm.filtered {
                report.push(format!(
                    "Self-harm: filtered (severity: {})",
                    self_harm.severity.as_deref().unwrap_or("unknown")
                ));
            }
        }

        if let Some(sexual) = &filter_results.sexual {
            if sexual.filtered {
                report.push(format!(
                    "Sexual: filtered (severity: {})",
                    sexual.severity.as_deref().unwrap_or("unknown")
                ));
            }
        }

        if let Some(violence) = &filter_results.violence {
            if violence.filtered {
                report.push(format!(
                    "Violence: filtered (severity: {})",
                    violence.severity.as_deref().unwrap_or("unknown")
                ));
            }
        }

        if report.is_empty() {
            "Content passed all filters".to_string()
        } else {
            report.join("\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_analysis_request() {
        let request = TextAnalysisRequest::new("Test text")
            .categories(vec![HarmCategory::Hate, HarmCategory::Violence])
            .halt_on_blocklist_hit(true)
            .output_type(OutputType::FourSeverityLevels);

        assert_eq!(request.text, "Test text");
        assert_eq!(request.categories.as_ref().unwrap().len(), 2);
        assert_eq!(request.halt_on_blocklist_hit, Some(true));
    }

    #[test]
    fn test_safety_thresholds() {
        let restrictive = SafetyThresholds::most_restrictive();
        assert_eq!(restrictive.hate, 0);
        assert_eq!(restrictive.violence, 0);

        let permissive = SafetyThresholds::permissive();
        assert_eq!(permissive.hate, 3);
        assert_eq!(permissive.violence, 3);
    }

    #[test]
    fn test_content_filter_details() {
        let details = ContentFilterDetails {
            hate: Some(ContentFilterResult {
                filtered: true,
                severity: Some("medium".to_string()),
            }),
            violence: Some(ContentFilterResult {
                filtered: false,
                severity: Some("low".to_string()),
            }),
            ..Default::default()
        };

        assert!(details.was_filtered());
        assert_eq!(details.highest_severity(), Some(SeverityLevel::Medium));
    }

    #[test]
    fn test_severity_level_ordering() {
        assert!(SeverityLevel::Safe < SeverityLevel::Low);
        assert!(SeverityLevel::Low < SeverityLevel::Medium);
        assert!(SeverityLevel::Medium < SeverityLevel::High);
    }

    #[test]
    fn test_severity_level_from_str() {
        assert_eq!(SeverityLevel::from_str("safe"), SeverityLevel::Safe);
        assert_eq!(SeverityLevel::from_str("Low"), SeverityLevel::Low);
        assert_eq!(SeverityLevel::from_str("MEDIUM"), SeverityLevel::Medium);
        assert_eq!(SeverityLevel::from_str("high"), SeverityLevel::High);
    }

    #[test]
    fn test_utils_format_report() {
        let details = ContentFilterDetails {
            hate: Some(ContentFilterResult {
                filtered: true,
                severity: Some("high".to_string()),
            }),
            ..Default::default()
        };

        let report = utils::format_filter_report(&details);
        assert!(report.contains("Hate"));
        assert!(report.contains("filtered"));
    }

    #[test]
    fn test_utils_check_safe() {
        let safe_details = ContentFilterDetails::default();
        assert!(utils::check_openai_filter_safe(&safe_details));

        let unsafe_details = ContentFilterDetails {
            hate: Some(ContentFilterResult {
                filtered: true,
                severity: None,
            }),
            ..Default::default()
        };
        assert!(!utils::check_openai_filter_safe(&unsafe_details));
    }
}

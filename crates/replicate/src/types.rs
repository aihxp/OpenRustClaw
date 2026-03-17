//! Common types used across the Replicate SDK.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

/// Serialize DateTime to RFC3339 string
fn serialize_datetime<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&dt.to_rfc3339())
}

/// Serialize optional DateTime to RFC3339 string
fn serialize_datetime_opt<S>(dt: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match dt {
        Some(dt) => serializer.serialize_str(&dt.to_rfc3339()),
        None => serializer.serialize_none(),
    }
}

/// Deserialize DateTime from RFC3339 string
fn deserialize_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(serde::de::Error::custom)
}

/// Deserialize optional DateTime from RFC3339 string
fn deserialize_datetime_opt<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) => DateTime::parse_from_rfc3339(&s)
            .map(|dt| Some(dt.with_timezone(&Utc)))
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

/// A prediction object returned by the Replicate API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    /// Unique identifier for the prediction.
    pub id: String,
    /// The model identifier (e.g., "black-forest-labs/flux-schnell").
    pub model: Option<String>,
    /// The version ID used for the prediction.
    pub version: Option<String>,
    /// The status of the prediction.
    pub status: PredictionStatus,
    /// The input parameters for the prediction.
    pub input: Option<serde_json::Value>,
    /// The output of the prediction (available when status is "succeeded").
    pub output: Option<serde_json::Value>,
    /// Error message if the prediction failed.
    pub error: Option<String>,
    /// Logs from the prediction.
    pub logs: Option<String>,
    /// Metrics for the prediction.
    pub metrics: Option<PredictionMetrics>,
    /// URLs for interacting with the prediction.
    pub urls: PredictionUrls,
    /// When the prediction was created.
    #[serde(
        serialize_with = "serialize_datetime",
        deserialize_with = "deserialize_datetime"
    )]
    pub created_at: DateTime<Utc>,
    /// When the prediction started processing.
    #[serde(
        serialize_with = "serialize_datetime_opt",
        deserialize_with = "deserialize_datetime_opt",
        default
    )]
    pub started_at: Option<DateTime<Utc>>,
    /// When the prediction completed.
    #[serde(
        serialize_with = "serialize_datetime_opt",
        deserialize_with = "deserialize_datetime_opt",
        default
    )]
    pub completed_at: Option<DateTime<Utc>>,
    /// The source of the prediction ("web" or "api").
    pub source: Option<String>,
    /// Whether the input/output data has been removed.
    pub data_removed: Option<bool>,
}

/// Status of a prediction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PredictionStatus {
    /// The prediction is starting up.
    Starting,
    /// The prediction is currently running.
    Processing,
    /// The prediction completed successfully.
    Succeeded,
    /// The prediction encountered an error.
    Failed,
    /// The prediction was cancelled.
    Canceled,
}

impl PredictionStatus {
    /// Check if the prediction is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Canceled)
    }

    /// Check if the prediction completed successfully.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Succeeded)
    }

    /// Check if the prediction is still running.
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Starting | Self::Processing)
    }
}

impl fmt::Display for PredictionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Starting => write!(f, "starting"),
            Self::Processing => write!(f, "processing"),
            Self::Succeeded => write!(f, "succeeded"),
            Self::Failed => write!(f, "failed"),
            Self::Canceled => write!(f, "canceled"),
        }
    }
}

/// Metrics for a prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionMetrics {
    /// The amount of CPU/GPU time used (in seconds).
    pub predict_time: Option<f64>,
    /// The total time taken (in seconds).
    pub total_time: Option<f64>,
}

/// URLs associated with a prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionUrls {
    /// URL to get the prediction.
    pub get: String,
    /// URL to cancel the prediction.
    pub cancel: Option<String>,
    /// URL to stream the prediction output.
    pub stream: Option<String>,
}

/// A model object returned by the Replicate API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// The owner of the model.
    pub owner: String,
    /// The name of the model.
    pub name: String,
    /// A description of the model.
    pub description: Option<String>,
    /// URL for the model's cover image.
    pub cover_image_url: Option<String>,
    /// URL for the model's source code on GitHub.
    pub github_url: Option<String>,
    /// URL for the model's paper.
    pub paper_url: Option<String>,
    /// URL for the model's license.
    pub license_url: Option<String>,
    /// Whether the model is public or private.
    pub visibility: Option<Visibility>,
    /// The default example prediction.
    pub default_example: Option<Prediction>,
    /// The latest version of the model.
    pub latest_version: Option<ModelVersion>,
}

/// Visibility of a model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    /// Public model, visible to everyone.
    Public,
    /// Private model, only visible to owner.
    Private,
}

/// A model version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVersion {
    /// Unique identifier for the version.
    pub id: String,
    /// When the version was created.
    #[serde(
        serialize_with = "serialize_datetime",
        deserialize_with = "deserialize_datetime"
    )]
    pub created_at: DateTime<Utc>,
    /// The OpenAPI schema for the version.
    pub openapi_schema: Option<serde_json::Value>,
    /// The cog version used.
    pub cog_version: Option<String>,
    /// The cog commit hash.
    pub cog_commit: Option<String>,
}

/// A deployment object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    /// The owner of the deployment.
    pub owner: String,
    /// The name of the deployment.
    pub name: String,
    /// The current release of the deployment.
    pub current_release: Option<DeploymentRelease>,
}

/// A deployment release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentRelease {
    /// The release number.
    pub number: u32,
    /// The model being deployed.
    pub model: String,
    /// The version ID being deployed.
    pub version: String,
    /// The hardware SKU.
    pub hardware: String,
    /// Minimum number of instances.
    pub min_instances: u32,
    /// Maximum number of instances.
    pub max_instances: u32,
    /// When the release was created.
    #[serde(
        serialize_with = "serialize_datetime",
        deserialize_with = "deserialize_datetime"
    )]
    pub created_at: DateTime<Utc>,
}

/// A training object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Training {
    /// Unique identifier for the training.
    pub id: String,
    /// The status of the training.
    pub status: PredictionStatus,
    /// The input parameters for the training.
    pub input: Option<serde_json::Value>,
    /// The output of the training.
    pub output: Option<serde_json::Value>,
    /// Error message if the training failed.
    pub error: Option<String>,
    /// Metrics for the training.
    pub metrics: Option<PredictionMetrics>,
    /// URLs for interacting with the training.
    pub urls: PredictionUrls,
    /// When the training was created.
    #[serde(
        serialize_with = "serialize_datetime",
        deserialize_with = "deserialize_datetime"
    )]
    pub created_at: DateTime<Utc>,
    /// When the training started processing.
    #[serde(
        serialize_with = "serialize_datetime_opt",
        deserialize_with = "deserialize_datetime_opt",
        default
    )]
    pub started_at: Option<DateTime<Utc>>,
    /// When the training completed.
    #[serde(
        serialize_with = "serialize_datetime_opt",
        deserialize_with = "deserialize_datetime_opt",
        default
    )]
    pub completed_at: Option<DateTime<Utc>>,
}

/// Hardware SKU for running models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hardware {
    /// The hardware SKU.
    pub sku: String,
    /// A human-readable name.
    pub name: String,
}

/// Pagination response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// The results.
    pub results: Vec<T>,
    /// URL for the next page.
    pub next: Option<String>,
    /// URL for the previous page.
    pub previous: Option<String>,
}

/// Account information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// The type of account ("user" or "organization").
    #[serde(rename = "type")]
    pub account_type: String,
    /// The username.
    pub username: String,
    /// The name of the user or organization.
    pub name: Option<String>,
    /// The GitHub account URL.
    pub github_url: Option<String>,
}

/// A collection of models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    /// The name of the collection.
    pub name: String,
    /// The slug of the collection.
    pub slug: String,
    /// A description of the collection.
    pub description: Option<String>,
    /// Models in the collection.
    pub models: Vec<Model>,
}

/// Webhook events filter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEvents {
    /// Immediately on prediction start.
    Start,
    /// Each time a prediction generates an output.
    Output,
    /// Each time log output is generated.
    Logs,
    /// When the prediction reaches a terminal state.
    Completed,
}

impl fmt::Display for WebhookEvents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Start => write!(f, "start"),
            Self::Output => write!(f, "output"),
            Self::Logs => write!(f, "logs"),
            Self::Completed => write!(f, "completed"),
        }
    }
}

/// Input for predictions - a flexible JSON object builder.
#[derive(Debug, Clone, Default)]
pub struct PredictionInput {
    inner: HashMap<String, serde_json::Value>,
}

impl PredictionInput {
    /// Create a new empty input.
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Add a string input.
    pub fn string(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner
            .insert(key.into(), serde_json::Value::String(value.into()));
        self
    }

    /// Add a number input.
    pub fn number(mut self, key: impl Into<String>, value: impl Into<f64>) -> Self {
        self.inner.insert(
            key.into(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(value.into()).unwrap_or_else(|| 0.into()),
            ),
        );
        self
    }

    /// Add a boolean input.
    pub fn bool(mut self, key: impl Into<String>, value: bool) -> Self {
        self.inner
            .insert(key.into(), serde_json::Value::Bool(value));
        self
    }

    /// Add a raw JSON value input.
    pub fn value(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.inner.insert(key.into(), value.into());
        self
    }

    /// Build into a JSON value.
    pub fn build(self) -> serde_json::Value {
        serde_json::Value::Object(self.inner.into_iter().collect())
    }
}

impl From<PredictionInput> for serde_json::Value {
    fn from(input: PredictionInput) -> Self {
        input.build()
    }
}

/// Pagination parameters.
#[derive(Debug, Clone)]
pub struct PaginationParams {
    /// Cursor for pagination.
    pub cursor: Option<String>,
    /// Maximum number of results to return.
    pub per_page: Option<u32>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            cursor: None,
            per_page: Some(100),
        }
    }
}

use std::fmt;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prediction_status() {
        assert!(PredictionStatus::Succeeded.is_terminal());
        assert!(PredictionStatus::Succeeded.is_success());
        assert!(!PredictionStatus::Processing.is_terminal());
        assert!(PredictionStatus::Processing.is_running());
    }

    #[test]
    fn test_prediction_input_builder() {
        let input = PredictionInput::new()
            .string("prompt", "A cat")
            .number("width", 1024.0)
            .bool("hd", true);

        let json = input.build();
        assert_eq!(json["prompt"], "A cat");
        assert_eq!(json["width"], 1024.0);
        assert_eq!(json["hd"], true);
    }

    #[test]
    fn test_webhook_events_display() {
        assert_eq!(WebhookEvents::Start.to_string(), "start");
        assert_eq!(WebhookEvents::Completed.to_string(), "completed");
    }

    #[test]
    fn test_datetime_serialization() {
        let dt = Utc::now();
        let json = serde_json::json!({
            "timestamp": dt.to_rfc3339()
        });
        assert!(json["timestamp"].as_str().is_some());
    }
}

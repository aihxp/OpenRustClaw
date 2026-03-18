//! LangSmith REST API client for trace export.
//!
//! Sends traces directly to LangSmith via REST API, making this
//! framework-agnostic (no langchain/langgraph dependency needed on Rust side).

use chrono::{DateTime, Utc};
use openrustclaw_core::error::{Error, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{debug, warn};
use uuid::Uuid;

/// LangSmith REST API client.
#[derive(Clone)]
pub struct LangSmithClient {
    api_key: String,
    project_name: String,
    endpoint: String,
    client: Client,
    enabled: bool,
}

/// A trace run to send to LangSmith.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceRun {
    pub id: String,
    pub name: String,
    pub run_type: RunType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_run_id: Option<String>,
    pub inputs: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub start_time: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunType {
    Llm,
    Tool,
    Chain,
    Retriever,
    Embedding,
}

/// Feedback to attach to a trace run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    pub key: String,
    pub score: Option<f64>,
    pub value: Option<serde_json::Value>,
    pub comment: Option<String>,
}

impl LangSmithClient {
    /// Create a new LangSmith client. If api_key is empty, tracing is disabled.
    pub fn new(api_key: String, project_name: String, endpoint: Option<String>) -> Self {
        let enabled = !api_key.is_empty();
        Self {
            api_key,
            project_name,
            endpoint: endpoint.unwrap_or_else(|| "https://api.smith.langchain.com".to_string()),
            client: Client::new(),
            enabled,
        }
    }

    /// Create a disabled client (no-op).
    pub fn disabled() -> Self {
        Self {
            api_key: String::new(),
            project_name: String::new(),
            endpoint: String::new(),
            client: Client::new(),
            enabled: false,
        }
    }

    /// Create a client from standard environment variables.
    pub fn from_env(project_name: Option<String>) -> Self {
        let api_key = env::var("LANGSMITH_API_KEY")
            .or_else(|_| env::var("LANGCHAIN_API_KEY"))
            .unwrap_or_default();
        let project_name = project_name
            .or_else(|| env::var("LANGSMITH_PROJECT").ok())
            .unwrap_or_else(|| "openrustclaw".to_string());
        let endpoint = env::var("LANGSMITH_ENDPOINT").ok();
        Self::new(api_key, project_name, endpoint)
    }

    /// Check if tracing is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Create a new trace run (start of an operation).
    pub fn new_run(&self, name: &str, run_type: RunType, inputs: serde_json::Value) -> TraceRun {
        TraceRun {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            run_type,
            parent_run_id: None,
            inputs,
            outputs: None,
            error: None,
            start_time: Utc::now(),
            end_time: None,
            extra: None,
            tags: None,
        }
    }

    /// Create a child trace run.
    pub fn new_child_run(
        &self,
        name: &str,
        run_type: RunType,
        parent_id: &str,
        inputs: serde_json::Value,
    ) -> TraceRun {
        let mut run = self.new_run(name, run_type, inputs);
        run.parent_run_id = Some(parent_id.to_string());
        run
    }

    /// Send a trace run to LangSmith.
    pub async fn trace_run(&self, run: &TraceRun) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let url = format!("{}/api/v1/runs", self.endpoint);
        let payload = serde_json::json!({
            "id": run.id,
            "name": run.name,
            "run_type": run.run_type,
            "parent_run_id": run.parent_run_id,
            "inputs": run.inputs,
            "outputs": run.outputs,
            "error": run.error,
            "start_time": run.start_time,
            "end_time": run.end_time,
            "extra": run.extra,
            "tags": run.tags,
            "session_name": self.project_name,
        });

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::Internal(format!("LangSmith trace error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            warn!("LangSmith trace failed ({}): {}", status, body);
        } else {
            debug!("LangSmith trace sent: {}", run.id);
        }

        Ok(())
    }

    /// Update an existing trace run (e.g., add outputs when complete).
    pub async fn update_run(&self, run: &TraceRun) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let url = format!("{}/api/v1/runs/{}", self.endpoint, run.id);
        let payload = serde_json::json!({
            "outputs": run.outputs,
            "error": run.error,
            "end_time": run.end_time,
            "extra": run.extra,
        });

        let response = self
            .client
            .patch(&url)
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::Internal(format!("LangSmith update error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            warn!("LangSmith update failed ({}): {}", status, body);
        }

        Ok(())
    }

    /// Log feedback for a trace run.
    pub async fn log_feedback(&self, run_id: &str, feedback: &Feedback) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let url = format!("{}/api/v1/feedback", self.endpoint);
        let payload = serde_json::json!({
            "run_id": run_id,
            "key": feedback.key,
            "score": feedback.score,
            "value": feedback.value,
            "comment": feedback.comment,
        });

        self.client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::Internal(format!("LangSmith feedback error: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_client_reports_disabled() {
        let client = LangSmithClient::disabled();
        assert!(!client.is_enabled());
    }

    #[test]
    fn new_run_populates_expected_defaults() {
        let client = LangSmithClient::disabled();
        let run = client.new_run(
            "scheduler_dispatch",
            RunType::Chain,
            serde_json::json!({"job_id":"job-1"}),
        );
        assert_eq!(run.name, "scheduler_dispatch");
        assert!(matches!(run.run_type, RunType::Chain));
        assert_eq!(run.inputs["job_id"], "job-1");
        assert!(run.outputs.is_none());
        assert!(run.error.is_none());
        assert!(run.end_time.is_none());
    }
}

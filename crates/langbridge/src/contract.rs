//! Typed workflow contract helpers for the Rust-sidecar boundary.

use std::collections::HashMap;

use openrustclaw_core::error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::proto::orchestration::WorkflowRequest;

/// Reserved metadata key used to transport typed configurable values.
pub const CONFIGURABLE_METADATA_KEY: &str = "__openrustclaw_configurable";

/// A typed workflow invocation sent from Rust to the Python sidecar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInvocation {
    pub workflow_id: String,
    pub thread_id: String,
    pub input: Value,
    pub metadata: HashMap<String, String>,
    pub configurable: Map<String, Value>,
}

impl WorkflowInvocation {
    /// Create a new workflow invocation with JSON input.
    pub fn new(workflow_id: impl Into<String>, thread_id: impl Into<String>, input: Value) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            thread_id: thread_id.into(),
            input,
            metadata: HashMap::new(),
            configurable: Map::new(),
        }
    }

    /// Attach flat string metadata to the invocation.
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Attach typed configurable values to the invocation.
    pub fn with_configurable(mut self, configurable: Map<String, Value>) -> Self {
        self.configurable = configurable;
        self
    }

    /// Serialize the invocation into the protobuf request shape.
    pub fn into_request(self) -> Result<WorkflowRequest> {
        let mut metadata = self.metadata;
        if !self.configurable.is_empty() {
            metadata.insert(
                CONFIGURABLE_METADATA_KEY.to_string(),
                serde_json::to_string(&self.configurable).map_err(|e| {
                    Error::Sidecar(format!("Failed to encode configurable metadata: {e}"))
                })?,
            );
        }

        Ok(WorkflowRequest {
            workflow_id: self.workflow_id,
            thread_id: self.thread_id,
            input: serde_json::to_string(&self.input)
                .map_err(|e| Error::Sidecar(format!("Failed to encode workflow input: {e}")))?,
            metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_invocation_serializes_typed_configurable_payload() {
        let configurable = Map::from_iter([
            ("job_id".to_string(), Value::String("job-1".to_string())),
            ("limit".to_string(), Value::from(25)),
            (
                "workflow_metadata".to_string(),
                serde_json::json!({
                    "priority": "high",
                    "notify": true,
                    "threshold": 0.8,
                }),
            ),
        ]);

        let request =
            WorkflowInvocation::new("scheduler", "thread-1", serde_json::json!({"ok": true}))
                .with_metadata(HashMap::from([(
                    "job_name".to_string(),
                    "Nightly sync".to_string(),
                )]))
                .with_configurable(configurable)
                .into_request()
                .unwrap();

        assert_eq!(request.workflow_id, "scheduler");
        assert_eq!(request.thread_id, "thread-1");
        assert_eq!(request.metadata.get("job_name").unwrap(), "Nightly sync");

        let encoded = request.metadata.get(CONFIGURABLE_METADATA_KEY).unwrap();
        let decoded: Value = serde_json::from_str(encoded).unwrap();
        assert_eq!(decoded["job_id"], "job-1");
        assert_eq!(decoded["limit"], 25);
        assert_eq!(decoded["workflow_metadata"]["notify"], true);
        assert_eq!(decoded["workflow_metadata"]["threshold"], 0.8);
    }
}

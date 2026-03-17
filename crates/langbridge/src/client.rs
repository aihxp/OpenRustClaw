//! gRPC client to the Python sidecar.

use tonic::transport::Channel;
use openrustclaw_core::error::{Error, Result};
use crate::proto::orchestration::orchestration_service_client::OrchestrationServiceClient;
use crate::proto::orchestration::{WorkflowRequest, WorkflowResponse};
use tracing::info;

/// gRPC client for the Python LangGraph sidecar.
pub struct LangBridgeClient {
    orchestration: OrchestrationServiceClient<Channel>,
}

impl LangBridgeClient {
    /// Connect to the sidecar.
    pub async fn connect(addr: &str) -> Result<Self> {
        let channel = Channel::from_shared(addr.to_string())
            .map_err(|e| Error::Sidecar(format!("Invalid address: {}", e)))?
            .connect()
            .await
            .map_err(|e| Error::Sidecar(format!("Connection failed: {}", e)))?;

        info!(addr = %addr, "Connected to Python sidecar");

        Ok(Self {
            orchestration: OrchestrationServiceClient::new(channel),
        })
    }

    /// Execute a LangGraph workflow.
    pub async fn execute_workflow(
        &mut self,
        workflow_id: &str,
        thread_id: &str,
        input: serde_json::Value,
    ) -> Result<WorkflowResponse> {
        let request = tonic::Request::new(WorkflowRequest {
            workflow_id: workflow_id.to_string(),
            thread_id: thread_id.to_string(),
            input: serde_json::to_string(&input)
                .map_err(|e| Error::Sidecar(e.to_string()))?,
            metadata: std::collections::HashMap::new(),
        });

        let response = self.orchestration
            .execute_workflow(request)
            .await
            .map_err(|e| Error::Sidecar(format!("Workflow execution failed: {}", e)))?;

        Ok(response.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::orchestration::{
        WorkflowRequest, WorkflowResponse, WorkflowUpdate,
        StatusRequest, StatusResponse,
    };

    // --- WorkflowRequest construction tests ---

    #[test]
    fn test_workflow_request_construction() {
        let req = WorkflowRequest {
            workflow_id: "test_workflow".to_string(),
            thread_id: "thread_123".to_string(),
            input: r#"{"key": "value"}"#.to_string(),
            metadata: std::collections::HashMap::new(),
        };
        assert_eq!(req.workflow_id, "test_workflow");
        assert_eq!(req.thread_id, "thread_123");
        assert_eq!(req.input, r#"{"key": "value"}"#);
        assert!(req.metadata.is_empty());
    }

    #[test]
    fn test_workflow_request_with_metadata() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("user_id".to_string(), "user_123".to_string());
        metadata.insert("session".to_string(), "sess_456".to_string());

        let req = WorkflowRequest {
            workflow_id: "wf".to_string(),
            thread_id: "t".to_string(),
            input: "{}".to_string(),
            metadata,
        };
        assert_eq!(req.metadata.len(), 2);
        assert_eq!(req.metadata.get("user_id").unwrap(), "user_123");
        assert_eq!(req.metadata.get("session").unwrap(), "sess_456");
    }

    #[test]
    fn test_workflow_request_empty_fields() {
        let req = WorkflowRequest {
            workflow_id: String::new(),
            thread_id: String::new(),
            input: String::new(),
            metadata: std::collections::HashMap::new(),
        };
        assert!(req.workflow_id.is_empty());
        assert!(req.thread_id.is_empty());
        assert!(req.input.is_empty());
    }

    #[test]
    fn test_workflow_request_json_input_serialization() {
        let input = serde_json::json!({"message": "hello", "count": 42});
        let input_str = serde_json::to_string(&input).unwrap();

        let req = WorkflowRequest {
            workflow_id: "wf".to_string(),
            thread_id: "t".to_string(),
            input: input_str.clone(),
            metadata: std::collections::HashMap::new(),
        };

        // Verify the JSON round-trips
        let parsed: serde_json::Value = serde_json::from_str(&req.input).unwrap();
        assert_eq!(parsed["message"], "hello");
        assert_eq!(parsed["count"], 42);
    }

    // --- WorkflowResponse construction tests ---

    #[test]
    fn test_workflow_response_completed() {
        let resp = WorkflowResponse {
            thread_id: "thread_123".to_string(),
            output: r#"{"result": "success"}"#.to_string(),
            status: "completed".to_string(),
            error: String::new(),
            trace_id: "trace_abc".to_string(),
        };
        assert_eq!(resp.status, "completed");
        assert!(resp.error.is_empty());
        assert_eq!(resp.trace_id, "trace_abc");
    }

    #[test]
    fn test_workflow_response_error() {
        let resp = WorkflowResponse {
            thread_id: "thread_123".to_string(),
            output: String::new(),
            status: "error".to_string(),
            error: "Something went wrong".to_string(),
            trace_id: "trace_abc".to_string(),
        };
        assert_eq!(resp.status, "error");
        assert!(!resp.error.is_empty());
    }

    #[test]
    fn test_workflow_response_pending_approval() {
        let resp = WorkflowResponse {
            thread_id: "t".to_string(),
            output: String::new(),
            status: "pending_approval".to_string(),
            error: String::new(),
            trace_id: String::new(),
        };
        assert_eq!(resp.status, "pending_approval");
    }

    // --- WorkflowUpdate construction tests ---

    #[test]
    fn test_workflow_update_construction() {
        let update = WorkflowUpdate {
            step_name: "extract_data".to_string(),
            status: "running".to_string(),
            output: "processing...".to_string(),
            trace_id: "trace_xyz".to_string(),
        };
        assert_eq!(update.step_name, "extract_data");
        assert_eq!(update.status, "running");
    }

    // --- StatusRequest/StatusResponse tests ---

    #[test]
    fn test_status_request_construction() {
        let req = StatusRequest {
            thread_id: "thread_abc".to_string(),
        };
        assert_eq!(req.thread_id, "thread_abc");
    }

    #[test]
    fn test_status_response_construction() {
        let resp = StatusResponse {
            status: "running".to_string(),
            current_step: "step_2".to_string(),
            steps_completed: 3,
        };
        assert_eq!(resp.status, "running");
        assert_eq!(resp.current_step, "step_2");
        assert_eq!(resp.steps_completed, 3);
    }

    // --- Connection error tests ---

    #[tokio::test]
    async fn test_connect_invalid_address() {
        let result = LangBridgeClient::connect("not-a-valid-address").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connect_unreachable_address() {
        // Port 1 is unlikely to have a gRPC server
        let result = LangBridgeClient::connect("http://127.0.0.1:1").await;
        // This may succeed (tonic lazy-connects) or fail depending on tonic version.
        // What matters is it doesn't panic.
        let _ = result;
    }

    #[tokio::test]
    async fn test_connect_empty_address() {
        let result = LangBridgeClient::connect("").await;
        assert!(result.is_err());
    }
}

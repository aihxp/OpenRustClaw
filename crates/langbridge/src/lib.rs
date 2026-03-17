//! Python sidecar gRPC bridge for OpenRustClaw.
//!
//! Connects to the Python LangGraph/LangSmith sidecar via gRPC (tonic).

pub mod client;
pub mod sidecar;

// Generated protobuf code
pub mod proto {
    pub mod orchestration {
        tonic::include_proto!("openrustclaw.orchestration");
    }
    pub mod tracing {
        tonic::include_proto!("openrustclaw.tracing");
    }
}

pub use client::LangBridgeClient;
pub use sidecar::SidecarManager;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::orchestration::{
        StatusRequest, StatusResponse, WorkflowRequest, WorkflowResponse,
    };
    use crate::proto::tracing::{TraceRequest, TraceResponse};

    // --- Proto type construction tests ---

    #[test]
    fn test_trace_request_construction() {
        let req = TraceRequest {
            run_id: "run_123".to_string(),
            name: "llm_call".to_string(),
            run_type: "llm".to_string(),
            parent_run_id: "parent_456".to_string(),
            inputs: r#"{"prompt": "hello"}"#.to_string(),
            outputs: r#"{"response": "hi"}"#.to_string(),
            error: String::new(),
            start_time: "2026-01-01T00:00:00Z".to_string(),
            end_time: "2026-01-01T00:00:01Z".to_string(),
            project_name: "my_project".to_string(),
        };
        assert_eq!(req.run_id, "run_123");
        assert_eq!(req.name, "llm_call");
        assert_eq!(req.run_type, "llm");
        assert_eq!(req.parent_run_id, "parent_456");
        assert!(req.error.is_empty());
    }

    #[test]
    fn test_trace_request_run_types() {
        for run_type in &["llm", "tool", "chain", "retriever"] {
            let req = TraceRequest {
                run_id: String::new(),
                name: String::new(),
                run_type: run_type.to_string(),
                parent_run_id: String::new(),
                inputs: String::new(),
                outputs: String::new(),
                error: String::new(),
                start_time: String::new(),
                end_time: String::new(),
                project_name: String::new(),
            };
            assert_eq!(req.run_type, *run_type);
        }
    }

    #[test]
    fn test_trace_request_with_error() {
        let req = TraceRequest {
            run_id: "run_err".to_string(),
            name: "failed_call".to_string(),
            run_type: "llm".to_string(),
            parent_run_id: String::new(),
            inputs: "{}".to_string(),
            outputs: String::new(),
            error: "Connection timeout".to_string(),
            start_time: "2026-01-01T00:00:00Z".to_string(),
            end_time: "2026-01-01T00:00:05Z".to_string(),
            project_name: "test".to_string(),
        };
        assert!(!req.error.is_empty());
        assert_eq!(req.error, "Connection timeout");
    }

    #[test]
    fn test_trace_response_success() {
        let resp = TraceResponse {
            success: true,
            error: String::new(),
        };
        assert!(resp.success);
        assert!(resp.error.is_empty());
    }

    #[test]
    fn test_trace_response_failure() {
        let resp = TraceResponse {
            success: false,
            error: "Failed to send trace".to_string(),
        };
        assert!(!resp.success);
        assert_eq!(resp.error, "Failed to send trace");
    }

    #[test]
    fn test_workflow_request_default() {
        let req = WorkflowRequest::default();
        assert!(req.workflow_id.is_empty());
        assert!(req.thread_id.is_empty());
        assert!(req.input.is_empty());
        assert!(req.metadata.is_empty());
    }

    #[test]
    fn test_workflow_response_default() {
        let resp = WorkflowResponse::default();
        assert!(resp.thread_id.is_empty());
        assert!(resp.output.is_empty());
        assert!(resp.status.is_empty());
        assert!(resp.error.is_empty());
        assert!(resp.trace_id.is_empty());
    }

    #[test]
    fn test_status_request_default() {
        let req = StatusRequest::default();
        assert!(req.thread_id.is_empty());
    }

    #[test]
    fn test_status_response_default() {
        let resp = StatusResponse::default();
        assert!(resp.status.is_empty());
        assert!(resp.current_step.is_empty());
        assert_eq!(resp.steps_completed, 0);
    }

    #[test]
    fn test_trace_request_default() {
        let req = TraceRequest::default();
        assert!(req.run_id.is_empty());
        assert!(req.name.is_empty());
        assert!(req.run_type.is_empty());
    }

    #[test]
    fn test_trace_response_default() {
        let resp = TraceResponse::default();
        assert!(!resp.success);
        assert!(resp.error.is_empty());
    }

    // --- SidecarManager re-export test ---

    #[test]
    fn test_sidecar_manager_accessible_from_lib() {
        let manager = SidecarManager::new("python3".to_string(), 50051);
        assert_eq!(manager.grpc_addr(), "http://127.0.0.1:50051");
    }

    // --- Error type tests ---

    #[test]
    fn test_sidecar_error_construction() {
        let err = openrustclaw_core::error::Error::Sidecar("test error".to_string());
        assert!(err.to_string().contains("test error"));
    }

    #[test]
    fn test_sidecar_error_connection_failed() {
        let err = openrustclaw_core::error::Error::Sidecar(
            "Connection failed: connection refused".to_string(),
        );
        assert!(err.to_string().contains("Connection failed"));
    }

    #[test]
    fn test_sidecar_error_invalid_address() {
        let err =
            openrustclaw_core::error::Error::Sidecar("Invalid address: missing scheme".to_string());
        assert!(err.to_string().contains("Invalid address"));
    }
}

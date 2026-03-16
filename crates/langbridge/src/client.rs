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

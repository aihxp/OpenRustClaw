//! Mobile node implementation

use crate::{MobileError, NodeConfig, NodeStatus};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Internal node state
pub struct NodeInner {
    config: NodeConfig,
    #[allow(dead_code)]
    runtime: Arc<Runtime>,
    connected: Arc<AtomicBool>,
    client: Arc<RwLock<Option<Arc<MobileClient>>>>,
    status: Arc<RwLock<NodeStatus>>,
}

/// Simple client for mobile node communication
#[derive(Clone)]
pub struct MobileClient {
    #[allow(dead_code)]
    gateway_url: String,
    #[allow(dead_code)]
    auth_token: String,
    #[allow(dead_code)]
    node_id: String,
}

impl MobileClient {
    /// Create a new mobile client
    pub async fn new(gateway_url: &str, _auth_token: &str) -> Result<Self, MobileError> {
        Ok(Self {
            gateway_url: gateway_url.to_string(),
            auth_token: _auth_token.to_string(),
            node_id: String::new(),
        })
    }

    /// Authenticate with the gateway
    pub async fn authenticate(&self, _token: &str) -> Result<(), MobileError> {
        // Placeholder: actual implementation would authenticate with gateway
        Ok(())
    }

    /// Connect with capabilities
    pub async fn connect_with_capabilities(
        &self,
        _node_id: &str,
        _device_name: &str,
        _capabilities: &[String],
    ) -> Result<(), MobileError> {
        // Placeholder: actual implementation would register with gateway
        Ok(())
    }

    /// Send a message
    pub async fn send(&self, _message: MobileMessage) -> Result<(), MobileError> {
        // Placeholder: actual implementation would send message via gateway
        Ok(())
    }

    /// Send heartbeat
    pub async fn heartbeat(&self) -> Result<(), MobileError> {
        // Placeholder: actual implementation would send heartbeat
        Ok(())
    }
}

/// Simple message structure for mobile nodes
#[derive(Clone, Debug)]
pub struct MobileMessage {
    pub id: String,
    pub source: String,
    pub target: String,
    pub content: Vec<u8>,
    pub content_type: String,
}

impl MobileMessage {
    /// Create a new message
    pub fn new(source: &str, target: &str, content: Vec<u8>, content_type: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source: source.to_string(),
            target: target.to_string(),
            content,
            content_type: content_type.to_string(),
        }
    }
}

impl NodeInner {
    /// Create a new node inner
    pub fn new(config: NodeConfig, runtime: Arc<Runtime>) -> Self {
        let node_id = config.node_id.clone();
        Self {
            config,
            runtime,
            connected: Arc::new(AtomicBool::new(false)),
            client: Arc::new(RwLock::new(None)),
            status: Arc::new(RwLock::new(NodeStatus {
                node_id,
                connected: false,
                peers_count: 0,
                last_sync: None,
                battery_aware: true,
            })),
        }
    }

    /// Start the node
    pub async fn start(&self) -> Result<(), MobileError> {
        info!("Starting mobile node: {}", self.config.node_id);

        // Create mobile client
        let client = Arc::new(
            MobileClient::new(&self.config.gateway_url, &self.config.auth_token)
                .await
                .map_err(|e| MobileError::Network(format!("Failed to create client: {}", e)))?,
        );

        // Authenticate
        client
            .authenticate(&self.config.auth_token)
            .await
            .map_err(|_| MobileError::Auth)?;

        // Connect as a mobile node with capabilities
        client
            .connect_with_capabilities(
                &self.config.node_id,
                &self.config.device_name,
                &self.config.capabilities,
            )
            .await
            .map_err(|e| MobileError::Network(format!("Failed to connect: {}", e)))?;

        *self.client.write().await = Some(client);
        self.connected.store(true, Ordering::SeqCst);

        // Update status
        let mut status = self.status.write().await;
        status.connected = true;

        // Start background tasks
        self.spawn_heartbeat();
        self.spawn_battery_monitor();

        info!("Mobile node {} started successfully", self.config.node_id);
        Ok(())
    }

    /// Stop the node
    pub fn stop(&self) {
        info!("Stopping mobile node: {}", self.config.node_id);
        self.connected.store(false, Ordering::SeqCst);
        // The client connection will be dropped when the handle is dropped
    }

    /// Send a message to a target
    pub async fn send_message(&self, target: &str, content: &str) -> Result<String, MobileError> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or(MobileError::NotConnected)?;

        let message = MobileMessage::new(
            &self.config.node_id,
            target,
            content.as_bytes().to_vec(),
            "text/plain",
        );

        let msg_id = message.id.clone();

        client
            .send(message)
            .await
            .map_err(|e| MobileError::Network(format!("Failed to send message: {}", e)))?;

        Ok(msg_id)
    }

    /// Get current status
    pub fn status(&self) -> NodeStatus {
        // Use blocking read since this may be called from non-async context
        let rt = tokio::runtime::Handle::try_current();
        match rt {
            Ok(_) => {
                // We're in an async context, try async read
                futures::executor::block_on(async { self.status.read().await.clone() })
            }
            Err(_) => {
                // Create a temporary runtime for blocking read
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async { self.status.read().await.clone() })
            }
        }
    }

    fn spawn_heartbeat(&self) {
        let connected = Arc::clone(&self.connected);
        let client = Arc::clone(&self.client);
        let node_id = self.config.node_id.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            while connected.load(Ordering::SeqCst) {
                interval.tick().await;

                let client_guard = client.read().await;
                if let Some(client_ref) = client_guard.as_ref()
                    && let Err(e) = client_ref.heartbeat().await
                {
                    warn!("Heartbeat failed for node {}: {}", node_id, e);
                }
            }
        });
    }

    fn spawn_battery_monitor(&self) {
        // On mobile, monitor battery and adjust sync frequency
        #[cfg(any(target_os = "ios", target_os = "android"))]
        {
            let connected = self.connected.clone();
            let status = self.status.clone();

            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(60));

                while connected.load(Ordering::SeqCst) {
                    interval.tick().await;

                    // Platform-specific battery monitoring would go here
                    // For now, just update the battery_aware flag
                    let mut s = status.write().await;
                    s.battery_aware = true;
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_config() {
        let config = NodeConfig::new("test-node", "ws://localhost:8080", "token123");
        assert_eq!(config.node_id, "test-node");
        assert_eq!(config.gateway_url, "ws://localhost:8080");
        assert_eq!(config.auth_token, "token123");
        assert_eq!(config.device_name, "Mobile Device");
        assert!(config.capabilities.contains(&"mobile".to_string()));
    }

    #[test]
    fn test_node_config_builder() {
        let config = NodeConfig::new("test-node", "ws://localhost:8080", "token123")
            .with_device_name("My Phone")
            .with_capabilities(vec!["mobile".to_string(), "sensor".to_string()]);

        assert_eq!(config.device_name, "My Phone");
        assert_eq!(config.capabilities.len(), 2);
        assert!(config.capabilities.contains(&"sensor".to_string()));
    }

    #[test]
    fn test_mobile_message() {
        let msg = MobileMessage::new("node1", "node2", b"hello".to_vec(), "text/plain");
        assert_eq!(msg.source, "node1");
        assert_eq!(msg.target, "node2");
        assert_eq!(msg.content, b"hello");
        assert_eq!(msg.content_type, "text/plain");
        assert!(!msg.id.is_empty());
    }
}

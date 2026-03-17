//! OpenRustClaw Mobile SDK
//!
//! This crate provides bindings for iOS and Android to use OpenRustClaw
//! as a mobile node in distributed systems.

pub mod node;
pub mod sync;
pub mod notifications;

#[cfg(target_os = "ios")]
pub mod ios;

#[cfg(target_os = "android")]
pub mod android;

// MobileNodeHandle is defined in this module below
pub use sync::{SyncConfig, SyncManager};

use serde::Serialize;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Mobile node handle
pub struct MobileNodeHandle {
    runtime: Arc<Runtime>,
    inner: Arc<node::NodeInner>,
}

impl MobileNodeHandle {
    /// Initialize the mobile node
    pub fn new(config: NodeConfig) -> Result<Self, MobileError> {
        let runtime = Arc::new(Runtime::new()?);
        let inner = Arc::new(node::NodeInner::new(config, runtime.clone()));

        Ok(Self { runtime, inner })
    }

    /// Start the node
    pub fn start(&self) -> Result<(), MobileError> {
        let inner = self.inner.clone();
        self.runtime.block_on(async move { inner.start().await })
    }

    /// Stop the node
    pub fn stop(&self) {
        self.inner.stop();
    }

    /// Send a message to the network
    pub fn send_message(&self, target: &str, content: &str) -> Result<String, MobileError> {
        let inner = self.inner.clone();
        let target = target.to_string();
        let content = content.to_string();

        self.runtime
            .block_on(async move { inner.send_message(&target, &content).await })
    }

    /// Get node status
    pub fn status(&self) -> NodeStatus {
        self.inner.status()
    }
}

/// Node configuration
#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub node_id: String,
    pub gateway_url: String,
    pub auth_token: String,
    pub device_name: String,
    pub capabilities: Vec<String>,
    pub sync_config: SyncConfig,
}

impl NodeConfig {
    /// Create a new node configuration
    pub fn new(
        node_id: impl Into<String>,
        gateway_url: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            gateway_url: gateway_url.into(),
            auth_token: auth_token.into(),
            device_name: "Mobile Device".to_string(),
            capabilities: vec!["mobile".to_string()],
            sync_config: SyncConfig::default(),
        }
    }

    /// Set the device name
    pub fn with_device_name(mut self, name: impl Into<String>) -> Self {
        self.device_name = name.into();
        self
    }

    /// Set the capabilities
    pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Set the sync configuration
    pub fn with_sync_config(mut self, config: SyncConfig) -> Self {
        self.sync_config = config;
        self
    }
}

/// Node status
#[derive(Debug, Clone, Serialize)]
pub struct NodeStatus {
    pub node_id: String,
    pub connected: bool,
    pub peers_count: usize,
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
    pub battery_aware: bool,
}

/// Mobile errors
#[derive(Debug, thiserror::Error)]
pub enum MobileError {
    #[error("Runtime error: {0}")]
    Runtime(#[from] std::io::Error),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Authentication error")]
    Auth,
    #[error("Not connected")]
    NotConnected,
    #[error("Invalid configuration: {0}")]
    Config(String),
}

// impl From<openrustclaw_distributed::error::DistributedError> for MobileError {
//     fn from(err: openrustclaw_distributed::error::DistributedError) -> Self {
//         MobileError::Network(err.to_string())
//     }
// }

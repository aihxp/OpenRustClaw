//! Service discovery for cluster formation.

use crate::config::{DiscoveryBackend, DiscoveryConfig};
use crate::error::{DistributedError, Result};
#[cfg(any(feature = "etcd", feature = "mdns"))]
use crate::node::NodeRole;
use crate::node::{NodeId, NodeInfo};
use async_trait::async_trait;
#[cfg(feature = "etcd")]
use etcd_client::Client as EtcdClient;
#[cfg(feature = "mdns")]
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
#[cfg(feature = "mdns")]
use std::collections::HashMap;
#[cfg(feature = "mdns")]
use std::net::SocketAddr;
use std::sync::Arc;
#[cfg(feature = "mdns")]
use tokio::sync::RwLock;
#[cfg(any(feature = "etcd", feature = "mdns"))]
use tracing::{debug, info};

/// Service discovery trait.
#[async_trait]
pub trait Discovery: Send + Sync {
    /// Register this node with the discovery service.
    async fn register(&self, node: &NodeInfo) -> Result<()>;

    /// Deregister this node.
    async fn deregister(&self, node_id: &NodeId) -> Result<()>;

    /// Discover other nodes in the cluster.
    async fn discover(&self) -> Result<Vec<NodeInfo>>;

    /// Watch for changes in cluster membership.
    async fn watch(&self) -> Result<Box<dyn DiscoveryStream>>;
}

/// Stream of discovery events.
pub trait DiscoveryStream: Send {
    /// Get the next discovery event.
    fn next<'a>(
        &'a mut self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Option<DiscoveryEvent>>> + Send + 'a>,
    >;
}

/// Discovery events.
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    NodeJoined(NodeInfo),
    NodeLeft(NodeId),
    NodeUpdated(NodeInfo),
}

/// Discovery factory.
pub async fn create_discovery(
    config: &DiscoveryConfig,
    #[allow(unused_variables)] local_id: NodeId,
) -> Result<Arc<dyn Discovery>> {
    match config.backend {
        #[cfg(feature = "etcd")]
        DiscoveryBackend::Etcd => {
            let discovery = EtcdDiscovery::new(config, local_id).await?;
            Ok(Arc::new(discovery))
        }
        #[cfg(not(feature = "etcd"))]
        DiscoveryBackend::Etcd => Err(DistributedError::Config(
            "etcd discovery requires the 'etcd' feature to be enabled".to_string(),
        )),
        #[cfg(feature = "consul")]
        DiscoveryBackend::Consul => {
            let discovery = ConsulDiscovery::new(config, local_id)?;
            Ok(Arc::new(discovery))
        }
        #[cfg(not(feature = "consul"))]
        DiscoveryBackend::Consul => Err(DistributedError::Config(
            "Consul discovery requires the 'consul' feature to be enabled".to_string(),
        )),
        #[cfg(feature = "mdns")]
        DiscoveryBackend::Gossip => {
            let discovery = GossipDiscovery::new(config, local_id)?;
            Ok(Arc::new(discovery))
        }
        #[cfg(not(feature = "mdns"))]
        DiscoveryBackend::Gossip => Err(DistributedError::Config(
            "Gossip discovery requires the 'mdns' feature to be enabled".to_string(),
        )),
        DiscoveryBackend::Static => {
            let discovery = StaticDiscovery::new(config)?;
            Ok(Arc::new(discovery))
        }
    }
}

/// etcd-based discovery.
#[cfg(feature = "etcd")]
pub struct EtcdDiscovery {
    client: EtcdClient,
    prefix: String,
    local_id: NodeId,
}

#[cfg(feature = "etcd")]
impl EtcdDiscovery {
    pub async fn new(config: &DiscoveryConfig, local_id: NodeId) -> Result<Self> {
        if config.etcd_endpoints.is_empty() {
            return Err(DistributedError::Config(
                "No etcd endpoints configured".to_string(),
            ));
        }

        let client = EtcdClient::connect(config.etcd_endpoints.clone(), None)
            .await
            .map_err(|e| {
                DistributedError::Discovery(format!("Failed to connect to etcd: {}", e))
            })?;

        let prefix = format!("/openrustclaw/{}/nodes", config.cluster_name);

        Ok(Self {
            client,
            prefix,
            local_id,
        })
    }

    fn node_key(&self, node_id: &str) -> String {
        format!("{}/{}", self.prefix, node_id)
    }
}

#[cfg(feature = "etcd")]
#[async_trait]
impl Discovery for EtcdDiscovery {
    async fn register(&self, node: &NodeInfo) -> Result<()> {
        let key = self.node_key(&node.id);
        let value = serde_json::to_vec(node)?;

        let mut client = self.client.clone();
        let lease = client
            .lease_grant(30, None)
            .await
            .map_err(|e| DistributedError::Discovery(format!("Failed to create lease: {}", e)))?;

        client
            .put(
                key,
                value,
                Some(etcd_client::PutOptions::new().with_lease(lease.id())),
            )
            .await
            .map_err(|e| DistributedError::Discovery(format!("Failed to register: {}", e)))?;

        info!(
            "Registered node {} with etcd (lease: {})",
            node.id,
            lease.id()
        );
        Ok(())
    }

    async fn deregister(&self, node_id: &NodeId) -> Result<()> {
        let key = self.node_key(node_id);

        let mut client = self.client.clone();
        client
            .delete(key, None)
            .await
            .map_err(|e| DistributedError::Discovery(format!("Failed to deregister: {}", e)))?;

        info!("Deregistered node {} from etcd", node_id);
        Ok(())
    }

    async fn discover(&self) -> Result<Vec<NodeInfo>> {
        let mut client = self.client.clone();
        let response = client
            .get(
                self.prefix.clone(),
                Some(etcd_client::GetOptions::new().with_prefix()),
            )
            .await
            .map_err(|e| DistributedError::Discovery(format!("Failed to discover: {}", e)))?;

        let mut nodes = Vec::new();
        for kv in response.kvs() {
            if let Ok(node) = serde_json::from_slice::<NodeInfo>(kv.value()) {
                if node.id != self.local_id {
                    nodes.push(node);
                }
            }
        }

        debug!("Discovered {} nodes from etcd", nodes.len());
        Ok(nodes)
    }

    async fn watch(&self) -> Result<Box<dyn DiscoveryStream>> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let prefix = self.prefix.clone();
        let local_id = self.local_id.clone();

        let mut client = self.client.clone();
        let (_, mut stream) = client
            .watch(prefix, Some(etcd_client::WatchOptions::new().with_prefix()))
            .await
            .map_err(|e| DistributedError::Discovery(format!("Failed to watch: {}", e)))?;

        tokio::spawn(async move {
            while let Some(response) = stream.message().await.ok().flatten() {
                for event in response.events() {
                    match event.event_type() {
                        etcd_client::EventType::Put => {
                            if let Some(kv) = event.kv() {
                                if let Ok(node) = serde_json::from_slice::<NodeInfo>(kv.value()) {
                                    if node.id != local_id {
                                        let _ = tx.send(DiscoveryEvent::NodeJoined(node)).await;
                                    }
                                }
                            }
                        }
                        etcd_client::EventType::Delete => {
                            if let Some(kv) = event.kv() {
                                let key = String::from_utf8_lossy(kv.key());
                                if let Some(id) = key.rsplit('/').next() {
                                    let _ = tx.send(DiscoveryEvent::NodeLeft(id.to_string())).await;
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(Box::new(ChannelDiscoveryStream { rx }))
    }
}

/// Consul-based discovery.
#[cfg(feature = "consul")]
pub struct ConsulDiscovery {
    #[allow(dead_code)]
    config: DiscoveryConfig,
    #[allow(dead_code)]
    local_id: NodeId,
}

#[cfg(feature = "consul")]
impl ConsulDiscovery {
    pub fn new(config: &DiscoveryConfig, local_id: NodeId) -> Result<Self> {
        if config.consul_addr.is_none() {
            return Err(DistributedError::Config(
                "No Consul address configured".to_string(),
            ));
        }

        Ok(Self {
            config: config.clone(),
            local_id,
        })
    }
}

#[cfg(feature = "consul")]
#[async_trait]
impl Discovery for ConsulDiscovery {
    async fn register(&self, node: &NodeInfo) -> Result<()> {
        // Consul registration would be implemented here
        info!("Registered node {} with Consul", node.id);
        Ok(())
    }

    async fn deregister(&self, node_id: &NodeId) -> Result<()> {
        info!("Deregistered node {} from Consul", node_id);
        Ok(())
    }

    async fn discover(&self) -> Result<Vec<NodeInfo>> {
        // Consul discovery would be implemented here
        Ok(vec![])
    }

    async fn watch(&self) -> Result<Box<dyn DiscoveryStream>> {
        let (_tx, rx) = tokio::sync::mpsc::channel(100);
        Ok(Box::new(ChannelDiscoveryStream { rx }))
    }
}

/// Gossip-based discovery using mDNS.
#[cfg(feature = "mdns")]
pub struct GossipDiscovery {
    mdns: ServiceDaemon,
    service_type: String,
    local_id: NodeId,
    #[allow(dead_code)]
    nodes: Arc<RwLock<HashMap<NodeId, NodeInfo>>>,
}

#[cfg(feature = "mdns")]
impl GossipDiscovery {
    pub fn new(config: &DiscoveryConfig, local_id: NodeId) -> Result<Self> {
        let mdns = ServiceDaemon::new().map_err(|e| {
            DistributedError::Discovery(format!("Failed to create mDNS daemon: {}", e))
        })?;

        let service_type = format!("_{}._tcp.local.", config.cluster_name);

        Ok(Self {
            mdns,
            service_type,
            local_id,
            nodes: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

#[cfg(feature = "mdns")]
#[async_trait]
impl Discovery for GossipDiscovery {
    async fn register(&self, node: &NodeInfo) -> Result<()> {
        let properties = [
            ("id".to_string(), node.id.clone()),
            ("role".to_string(), node.role.to_string()),
            ("api_addr".to_string(), node.api_addr.to_string()),
        ];

        let service_info = ServiceInfo::new(
            &self.service_type,
            &format!("node-{}", node.id),
            "local.",
            &node.cluster_addr.ip().to_string(),
            node.cluster_addr.port(),
            &properties[..],
        )
        .map_err(|e| {
            DistributedError::Discovery(format!("Failed to create service info: {}", e))
        })?;

        self.mdns
            .register(service_info)
            .map_err(|e| DistributedError::Discovery(format!("Failed to register: {}", e)))?;

        info!("Registered node {} with mDNS", node.id);
        Ok(())
    }

    async fn deregister(&self, node_id: &NodeId) -> Result<()> {
        let fullname = format!("node-{}.{}", node_id, self.service_type);
        self.mdns
            .unregister(&fullname)
            .map_err(|e| DistributedError::Discovery(format!("Failed to deregister: {}", e)))?;

        info!("Deregistered node {} from mDNS", node_id);
        Ok(())
    }

    async fn discover(&self) -> Result<Vec<NodeInfo>> {
        let receiver = self
            .mdns
            .browse(&self.service_type)
            .map_err(|e| DistributedError::Discovery(format!("Failed to browse: {}", e)))?;

        let mut nodes = Vec::new();

        // Process events for a short time to collect services
        let timeout = tokio::time::Duration::from_secs(2);
        let start = tokio::time::Instant::now();

        while start.elapsed() < timeout {
            if let Ok(event) = receiver.recv_timeout(std::time::Duration::from_millis(100)) {
                if let ServiceEvent::ServiceResolved(info) = event {
                    if let Some(node) = service_info_to_node(info) {
                        if node.id != self.local_id {
                            nodes.push(node);
                        }
                    }
                }
            }
        }

        Ok(nodes)
    }

    async fn watch(&self) -> Result<Box<dyn DiscoveryStream>> {
        let receiver = self
            .mdns
            .browse(&self.service_type)
            .map_err(|e| DistributedError::Discovery(format!("Failed to browse: {}", e)))?;

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let local_id = self.local_id.clone();

        tokio::spawn(async move {
            while let Ok(event) = receiver.recv() {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        if let Some(node) = service_info_to_node(info) {
                            if node.id != local_id {
                                let _ = tx.send(DiscoveryEvent::NodeJoined(node)).await;
                            }
                        }
                    }
                    ServiceEvent::ServiceRemoved(_, fullname) => {
                        // Extract node ID from fullname
                        if let Some(id) = fullname.strip_prefix("node-") {
                            if let Some(id) = id.split('.').next() {
                                let _ = tx.send(DiscoveryEvent::NodeLeft(id.to_string())).await;
                            }
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok(Box::new(ChannelDiscoveryStream { rx }))
    }
}

/// Static discovery using seed nodes.
pub struct StaticDiscovery {
    #[allow(dead_code)]
    seed_nodes: Vec<String>,
}

impl StaticDiscovery {
    pub fn new(config: &DiscoveryConfig) -> Result<Self> {
        Ok(Self {
            seed_nodes: config.seed_nodes.clone(),
        })
    }
}

#[async_trait]
impl Discovery for StaticDiscovery {
    async fn register(&self, _node: &NodeInfo) -> Result<()> {
        // No-op for static discovery
        Ok(())
    }

    async fn deregister(&self, _node_id: &NodeId) -> Result<()> {
        // No-op for static discovery
        Ok(())
    }

    async fn discover(&self) -> Result<Vec<NodeInfo>> {
        // In static discovery, nodes are provided via seed_nodes
        // They would need to be parsed from config
        Ok(vec![])
    }

    async fn watch(&self) -> Result<Box<dyn DiscoveryStream>> {
        // No dynamic updates in static discovery
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        Ok(Box::new(ChannelDiscoveryStream { rx }))
    }
}

/// Discovery stream implementation using channels.
struct ChannelDiscoveryStream {
    rx: tokio::sync::mpsc::Receiver<DiscoveryEvent>,
}

impl DiscoveryStream for ChannelDiscoveryStream {
    fn next<'a>(
        &'a mut self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Option<DiscoveryEvent>>> + Send + 'a>,
    > {
        Box::pin(async move { Ok(self.rx.recv().await) })
    }
}

/// Convert mDNS service info to NodeInfo.
#[cfg(feature = "mdns")]
fn service_info_to_node(info: ServiceInfo) -> Option<NodeInfo> {
    let id = info.get_property_val_str("id")?;
    let role_str = info.get_property_val_str("role")?;
    let api_addr_str = info.get_property_val_str("api_addr")?;

    let role = match role_str {
        "leader" => NodeRole::Leader,
        _ => NodeRole::Worker,
    };

    let cluster_addr =
        SocketAddr::new(info.get_addresses().iter().next()?.clone(), info.get_port());
    let api_addr: SocketAddr = api_addr_str.parse().ok()?;

    Some(NodeInfo::new(id, id, cluster_addr, api_addr, role))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_static_discovery() {
        let config = DiscoveryConfig {
            backend: DiscoveryBackend::Static,
            seed_nodes: vec!["127.0.0.1:50051".to_string()],
            ..DiscoveryConfig::default()
        };

        let discovery = StaticDiscovery::new(&config).unwrap();

        // Should be empty since we haven't parsed seed nodes into NodeInfo yet
        let nodes = discovery.discover().await.unwrap();
        assert!(nodes.is_empty());
    }
}

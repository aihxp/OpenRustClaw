//! Load balancing for request distribution across workers.

use crate::config::{LoadBalanceStrategy, LoadBalancerConfig};
use crate::error::{DistributedError, Result};
use crate::node::{NodeId, NodeInfo};
use dashmap::DashMap;
use siphasher::sip::SipHasher13;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;
use tracing::{debug, info, trace, warn};

#[cfg(test)]
use futures::FutureExt;

/// Load balancer for distributing requests across nodes.
pub struct LoadBalancer {
    config: LoadBalancerConfig,
    /// Ring of virtual nodes for consistent hashing.
    consistent_hash_ring: RwLock<Vec<(u64, NodeId)>>,
    /// Connection counts per node (for least-connections).
    connection_counts: DashMap<NodeId, AtomicU64>,
    /// Response times per node.
    response_times: DashMap<NodeId, RwLock<Vec<f64>>>,
    /// Session affinity mapping (session_id -> node_id).
    pub session_affinity: DashMap<String, NodeId>,
    /// Round-robin counter.
    round_robin_counter: AtomicU64,
}

impl LoadBalancer {
    /// Create a new load balancer.
    pub fn new(config: LoadBalancerConfig) -> Arc<Self> {
        Arc::new(Self {
            config,
            consistent_hash_ring: RwLock::new(Vec::new()),
            connection_counts: DashMap::new(),
            response_times: DashMap::new(),
            session_affinity: DashMap::new(),
            round_robin_counter: AtomicU64::new(0),
        })
    }

    /// Update the set of available nodes.
    pub async fn update_nodes(&self, nodes: &[NodeInfo]) {
        // Clear and rebuild consistent hash ring if enabled
        if self.config.consistent_hashing {
            let mut ring = self.consistent_hash_ring.write().await;
            ring.clear();

            for node in nodes {
                // Create virtual nodes
                for i in 0..self.config.virtual_nodes {
                    let hash = self.hash_virtual_node(&node.id, i);
                    ring.push((hash, node.id.clone()));
                }

                // Initialize connection counter if not exists
                if !self.connection_counts.contains_key(&node.id) {
                    self.connection_counts
                        .insert(node.id.clone(), AtomicU64::new(0));
                }

                // Initialize response times if not exists
                if !self.response_times.contains_key(&node.id) {
                    self.response_times
                        .insert(node.id.clone(), RwLock::new(Vec::new()));
                }
            }

            // Sort the ring by hash value
            ring.sort_by_key(|(hash, _)| *hash);

            debug!(
                "Updated consistent hash ring with {} virtual nodes",
                ring.len()
            );
        }
    }

    /// Select a node for a request.
    pub async fn select_node(
        &self,
        nodes: &[NodeInfo],
        session_id: Option<&str>,
    ) -> Result<NodeInfo> {
        if nodes.is_empty() {
            return Err(DistributedError::Cluster("No available nodes".to_string()));
        }

        // Check session affinity
        if let Some(sid) = session_id
            && self.config.sticky_sessions
            && let Some(node_id) = self.session_affinity.get(sid)
            && let Some(node) = nodes.iter().find(|n| &n.id == node_id.value())
        {
            trace!("Session {} routed to {} (sticky)", sid, node.id);
            return Ok(node.clone());
        }

        let selected = match self.config.strategy {
            LoadBalanceStrategy::RoundRobin => self.round_robin(nodes),
            LoadBalanceStrategy::LeastConnections => self.least_connections(nodes),
            LoadBalanceStrategy::LeastResponseTime => self.least_response_time(nodes),
            LoadBalanceStrategy::ConsistentHash | LoadBalanceStrategy::Random => {
                if let Some(sid) = session_id {
                    self.consistent_hash(nodes, sid)
                } else {
                    self.round_robin(nodes)
                }
            }
            LoadBalanceStrategy::WeightedRoundRobin => self.weighted_round_robin(nodes),
        }?;

        // Record session affinity
        if let Some(sid) = session_id
            && self.config.sticky_sessions
        {
            self.session_affinity
                .insert(sid.to_string(), selected.id.clone());
        }

        // Increment connection count
        if let Some(counter) = self.connection_counts.get(&selected.id) {
            counter.fetch_add(1, Ordering::SeqCst);
        }

        Ok(selected)
    }

    /// Release a connection to a node (decrement count).
    pub fn release_connection(&self, node_id: &NodeId) {
        if let Some(counter) = self.connection_counts.get(node_id) {
            counter.fetch_sub(1, Ordering::SeqCst);
        }
    }

    /// Record response time for a node.
    pub async fn record_response_time(&self, node_id: &NodeId, response_time_ms: f64) {
        if let Some(times) = self.response_times.get(node_id) {
            let mut guard = times.write().await;
            guard.push(response_time_ms);
            // Keep only last 100 measurements
            if guard.len() > 100 {
                guard.remove(0);
            }
        }
    }

    /// Remove a node from the balancer.
    pub async fn remove_node(&self, node_id: &NodeId) {
        self.connection_counts.remove(node_id);
        self.response_times.remove(node_id);

        // Remove from consistent hash ring
        let mut ring = self.consistent_hash_ring.write().await;
        ring.retain(|(_, id)| id != node_id);

        // Remove session affinities for this node
        let sessions_to_remove: Vec<String> = self
            .session_affinity
            .iter()
            .filter(|entry| entry.value() == node_id)
            .map(|entry| entry.key().clone())
            .collect();

        for session in sessions_to_remove {
            self.session_affinity.remove(&session);
        }
    }

    /// Get average response time for a node.
    pub async fn avg_response_time(&self, node_id: &NodeId) -> Option<f64> {
        let times = self.response_times.get(node_id)?;
        let guard = times.read().await;
        if guard.is_empty() {
            return None;
        }
        Some(guard.iter().sum::<f64>() / guard.len() as f64)
    }

    /// Get connection count for a node.
    pub fn connection_count(&self, node_id: &NodeId) -> u64 {
        self.connection_counts
            .get(node_id)
            .map(|c| c.load(Ordering::SeqCst))
            .unwrap_or(0)
    }

    /// Round-robin selection.
    fn round_robin(&self, nodes: &[NodeInfo]) -> Result<NodeInfo> {
        let idx = self.round_robin_counter.fetch_add(1, Ordering::SeqCst) as usize % nodes.len();
        Ok(nodes[idx].clone())
    }

    /// Least connections selection.
    fn least_connections(&self, nodes: &[NodeInfo]) -> Result<NodeInfo> {
        let mut min_connections = u64::MAX;
        let mut selected = None;

        for node in nodes {
            let connections = self.connection_count(&node.id);
            if connections < min_connections {
                min_connections = connections;
                selected = Some(node.clone());
            }
        }

        selected.ok_or_else(|| DistributedError::Cluster("No available nodes".to_string()))
    }

    /// Least response time selection.
    fn least_response_time(&self, nodes: &[NodeInfo]) -> Result<NodeInfo> {
        // For now, use a simple approach - this could be made async
        // For simplicity, falling back to least connections
        self.least_connections(nodes)
    }

    /// Consistent hashing selection.
    fn consistent_hash(&self, nodes: &[NodeInfo], key: &str) -> Result<NodeInfo> {
        let hash = self.hash_key(key);

        let ring = if let Ok(ring) = self.consistent_hash_ring.try_read() {
            ring
        } else if tokio::runtime::Handle::try_current().is_ok() {
            tokio::task::block_in_place(|| self.consistent_hash_ring.blocking_read())
        } else {
            self.consistent_hash_ring.blocking_read()
        };
        if ring.is_empty() {
            // Fallback to round-robin
            return self.round_robin(nodes);
        }

        // Find the first virtual node with hash >= key hash
        let idx = match ring.binary_search_by_key(&hash, |(h, _)| *h) {
            Ok(i) => i,
            Err(i) => i % ring.len(),
        };

        // Get the node ID and find the corresponding node
        let node_id = &ring[idx].1;
        nodes
            .iter()
            .find(|n| &n.id == node_id)
            .cloned()
            .ok_or_else(|| DistributedError::Cluster("Consistent hash node not found".to_string()))
    }

    /// Weighted round-robin selection.
    fn weighted_round_robin(&self, nodes: &[NodeInfo]) -> Result<NodeInfo> {
        // Simplified - could use weights from node metadata
        self.round_robin(nodes)
    }

    /// Hash a key for consistent hashing.
    fn hash_key(&self, key: &str) -> u64 {
        let mut hasher = SipHasher13::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    /// Hash a virtual node.
    fn hash_virtual_node(&self, node_id: &str, replica: usize) -> u64 {
        let key = format!("{}:{}", node_id, replica);
        self.hash_key(&key)
    }

    /// Clear session affinity for a session.
    pub fn clear_session(&self, session_id: &str) {
        self.session_affinity.remove(session_id);
    }

    /// Get all session affinities.
    pub fn session_affinities(&self) -> HashMap<String, NodeId> {
        self.session_affinity
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }
}

/// Session-aware router for WebSocket connections.
pub struct SessionRouter {
    load_balancer: Arc<LoadBalancer>,
    /// Local node ID.
    local_id: NodeId,
    /// Proxy configuration for routing to other nodes (reserved for future use).
    #[allow(dead_code)]
    proxy_config: ProxyConfig,
}

/// Proxy configuration.
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    /// Whether to enable proxying to other nodes.
    pub enabled: bool,
    /// Timeout for proxy connections.
    pub timeout_secs: u64,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout_secs: 30,
        }
    }
}

impl SessionRouter {
    /// Create a new session router.
    pub fn new(
        load_balancer: Arc<LoadBalancer>,
        local_id: NodeId,
        proxy_config: ProxyConfig,
    ) -> Self {
        Self {
            load_balancer,
            local_id,
            proxy_config,
        }
    }

    /// Route a session to the appropriate node.
    pub async fn route_session(
        &self,
        session_id: &str,
        available_nodes: &[NodeInfo],
    ) -> RouteResult {
        match self
            .load_balancer
            .select_node(available_nodes, Some(session_id))
            .await
        {
            Ok(node) => {
                if node.id == self.local_id {
                    RouteResult::Local
                } else {
                    RouteResult::Remote(node)
                }
            }
            Err(e) => {
                warn!("Failed to route session {}: {}", session_id, e);
                RouteResult::Error(e.to_string())
            }
        }
    }

    /// Check if a session is local.
    pub fn is_local_session(&self, session_id: &str) -> bool {
        self.load_balancer
            .session_affinity
            .get(session_id)
            .map(|node_id| *node_id == self.local_id)
            .unwrap_or(true) // Assume local if no affinity
    }

    /// Migrate a session to a different node.
    pub async fn migrate_session(&self, session_id: &str, target_node: &NodeInfo) -> Result<()> {
        self.load_balancer
            .session_affinity
            .insert(session_id.to_string(), target_node.id.clone());

        info!("Migrated session {} to node {}", session_id, target_node.id);

        Ok(())
    }
}

/// Result of routing a session.
#[derive(Debug, Clone)]
pub enum RouteResult {
    /// Handle locally.
    Local,
    /// Forward to remote node.
    Remote(NodeInfo),
    /// Routing error.
    Error(String),
}

/// Load balancer metrics.
#[derive(Debug, Clone, Default)]
pub struct LoadBalancerMetrics {
    pub total_requests: u64,
    pub local_requests: u64,
    pub proxied_requests: u64,
    pub failed_requests: u64,
    pub avg_response_time_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::NodeRole;
    use std::net::SocketAddr;

    fn create_test_node(id: &str) -> NodeInfo {
        let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
        NodeInfo::new(id, format!("node-{}", id), addr, addr, NodeRole::Worker)
    }

    #[tokio::test]
    async fn test_round_robin() {
        let config = LoadBalancerConfig::default();
        let balancer = LoadBalancer::new(config);

        let nodes = vec![
            create_test_node("node-1"),
            create_test_node("node-2"),
            create_test_node("node-3"),
        ];

        let selections: Vec<String> = (0..6)
            .map(|_| {
                balancer
                    .select_node(&nodes, None)
                    .now_or_never()
                    .unwrap()
                    .unwrap()
                    .id
            })
            .collect();

        // Should cycle through nodes
        assert_eq!(selections[0], "node-1");
        assert_eq!(selections[1], "node-2");
        assert_eq!(selections[2], "node-3");
        assert_eq!(selections[3], "node-1");
    }

    #[tokio::test]
    async fn test_consistent_hashing() {
        let config = LoadBalancerConfig {
            strategy: LoadBalanceStrategy::ConsistentHash,
            consistent_hashing: true,
            virtual_nodes: 10,
            ..LoadBalancerConfig::default()
        };
        let balancer = LoadBalancer::new(config);

        let nodes = vec![
            create_test_node("node-1"),
            create_test_node("node-2"),
            create_test_node("node-3"),
        ];

        balancer.update_nodes(&nodes).await;

        // Same session should route to same node
        let session_id = "test-session-123";
        let node1 = balancer
            .select_node(&nodes, Some(session_id))
            .await
            .unwrap();
        let node2 = balancer
            .select_node(&nodes, Some(session_id))
            .await
            .unwrap();

        assert_eq!(node1.id, node2.id);
    }

    #[tokio::test]
    async fn test_session_affinity() {
        let config = LoadBalancerConfig {
            sticky_sessions: true,
            ..LoadBalancerConfig::default()
        };
        let balancer = LoadBalancer::new(config);

        let nodes = vec![create_test_node("node-1"), create_test_node("node-2")];

        balancer.update_nodes(&nodes).await;

        let session_id = "sticky-session";
        let selected = balancer
            .select_node(&nodes, Some(session_id))
            .await
            .unwrap();

        // Second request should go to same node
        let selected2 = balancer
            .select_node(&nodes, Some(session_id))
            .await
            .unwrap();
        assert_eq!(selected.id, selected2.id);

        // Verify affinity is stored
        assert!(balancer.session_affinity.contains_key(session_id));
    }

    #[tokio::test]
    async fn test_least_connections() {
        let config = LoadBalancerConfig {
            strategy: LoadBalanceStrategy::LeastConnections,
            ..LoadBalancerConfig::default()
        };
        let balancer = LoadBalancer::new(config);

        let nodes = vec![create_test_node("node-1"), create_test_node("node-2")];

        balancer.update_nodes(&nodes).await;

        // Add connections to node-1
        if let Some(counter) = balancer.connection_counts.get("node-1") {
            counter.store(5, Ordering::SeqCst);
        }

        // Should select node-2 (0 connections)
        let selected = balancer.select_node(&nodes, None).await.unwrap();
        assert_eq!(selected.id, "node-2");
    }
}

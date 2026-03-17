//! Cluster management for distributed OpenRustClaw.

use crate::config::DistributedConfig;
use crate::error::Result;
use crate::node::{LocalNode, NodeId, NodeInfo, NodeMetrics, NodeRole, NodeState};
use chrono::Utc;
use dashmap::DashMap;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{interval, Duration};
use tracing::{info, warn};
use uuid::Uuid;

/// The state of the entire cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterState {
    /// Cluster is being initialized.
    Initializing,
    /// Cluster is active and operational.
    Active,
    /// Cluster is degraded (some nodes unavailable).
    Degraded,
    /// Network partition detected.
    Partitioned,
    /// Cluster is recovering from a failure.
    Recovering,
    /// Cluster is shutting down.
    ShuttingDown,
}

impl std::fmt::Display for ClusterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClusterState::Initializing => write!(f, "initializing"),
            ClusterState::Active => write!(f, "active"),
            ClusterState::Degraded => write!(f, "degraded"),
            ClusterState::Partitioned => write!(f, "partitioned"),
            ClusterState::Recovering => write!(f, "recovering"),
            ClusterState::ShuttingDown => write!(f, "shutting_down"),
        }
    }
}

/// Events that can occur in the cluster.
#[derive(Debug, Clone)]
pub enum ClusterEvent {
    /// A new node joined the cluster.
    NodeJoined(NodeInfo),
    /// A node left the cluster.
    NodeLeft(NodeId),
    /// A node changed state.
    NodeStateChanged {
        node_id: NodeId,
        old_state: NodeState,
        new_state: NodeState,
    },
    /// A node changed role.
    NodeRoleChanged {
        node_id: NodeId,
        old_role: NodeRole,
        new_role: NodeRole,
    },
    /// Leader has changed.
    LeaderChanged {
        old_leader: Option<NodeId>,
        new_leader: Option<NodeId>,
    },
    /// Cluster state changed.
    ClusterStateChanged {
        old_state: ClusterState,
        new_state: ClusterState,
    },
    /// Node metrics updated.
    MetricsUpdated { node_id: NodeId, metrics: NodeMetrics },
    /// Split brain detected.
    SplitBrainDetected { conflicting_leaders: Vec<NodeId> },
}

/// Cluster membership and management.
pub struct Cluster {
    /// Local node.
    local_node: Arc<LocalNode>,
    /// All nodes in the cluster (including local).
    nodes: DashMap<NodeId, NodeInfo>,
    /// Current cluster state.
    state: RwLock<ClusterState>,
    /// Current leader ID (if known).
    leader_id: RwLock<Option<NodeId>>,
    /// Cluster ID (generated on bootstrap).
    cluster_id: String,
    /// Configuration.
    config: DistributedConfig,
    /// Event broadcaster.
    event_tx: broadcast::Sender<ClusterEvent>,
    /// Node metrics.
    metrics: DashMap<NodeId, NodeMetrics>,
    /// Current term (for Raft).
    current_term: AtomicU64,
}

impl Cluster {
    /// Create a new cluster instance.
    pub fn new(local_node: Arc<LocalNode>, config: DistributedConfig) -> Arc<Self> {
        let (event_tx, _) = broadcast::channel(1024);
        let cluster_id = Uuid::new_v4().to_string();

        let cluster = Arc::new(Self {
            local_node,
            nodes: DashMap::new(),
            state: RwLock::new(ClusterState::Initializing),
            leader_id: RwLock::new(None),
            cluster_id,
            config,
            event_tx,
            metrics: DashMap::new(),
            current_term: AtomicU64::new(0),
        });

        // Add local node to the cluster (blocking read since we're in a non-async context)
        let local_info = cluster.local_node.info_blocking();
        cluster.nodes.insert(local_info.id.clone(), local_info);

        cluster
    }

    /// Get the cluster ID.
    pub fn cluster_id(&self) -> &str {
        &self.cluster_id
    }

    /// Get the local node.
    pub fn local_node(&self) -> Arc<LocalNode> {
        self.local_node.clone()
    }

    /// Get the local node ID.
    pub fn local_id(&self) -> NodeId {
        self.local_node.id()
    }

    /// Check if this node is the leader.
    pub async fn is_leader(&self) -> bool {
        self.local_node.is_leader().await
    }

    /// Get the current leader ID.
    pub async fn leader_id(&self) -> Option<NodeId> {
        self.leader_id.read().await.clone()
    }

    /// Set the leader ID.
    pub async fn set_leader(&self, leader_id: Option<NodeId>) {
        let mut current = self.leader_id.write().await;
        let old_leader = current.clone();
        *current = leader_id.clone();
        
        if old_leader != leader_id {
            info!(
                "Leader changed from {:?} to {:?}",
                old_leader, leader_id
            );
            let _ = self.event_tx.send(ClusterEvent::LeaderChanged {
                old_leader,
                new_leader: leader_id,
            });
        }
    }

    /// Get current cluster state.
    pub async fn state(&self) -> ClusterState {
        *self.state.read().await
    }

    /// Set cluster state.
    pub async fn set_state(&self, new_state: ClusterState) {
        let mut state = self.state.write().await;
        let old_state = *state;
        *state = new_state;
        
        if old_state != new_state {
            info!("Cluster state changed from {} to {}", old_state, new_state);
            let _ = self.event_tx.send(ClusterEvent::ClusterStateChanged {
                old_state,
                new_state,
            });
        }
    }

    /// Get the current term.
    pub fn current_term(&self) -> u64 {
        self.current_term.load(Ordering::SeqCst)
    }

    /// Set the current term.
    pub fn set_term(&self, term: u64) {
        self.current_term.store(term, Ordering::SeqCst);
    }

    /// Increment term and return new value.
    pub fn increment_term(&self) -> u64 {
        self.current_term.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Add or update a node in the cluster.
    pub async fn upsert_node(&self, mut info: NodeInfo) {
        let node_id = info.id.clone();
        
        // Don't modify our own node info
        if node_id == self.local_id() {
            return;
        }

        if let Some(mut existing) = self.nodes.get_mut(&node_id) {
            let old_state = existing.state;
            let old_role = existing.role;
            
            existing.last_heartbeat = Utc::now();
            existing.state = info.state;
            existing.role = info.role;
            existing.metadata = info.metadata.clone();
            existing.term = info.term;
            
            if old_state != info.state {
                let _ = self.event_tx.send(ClusterEvent::NodeStateChanged {
                    node_id: node_id.clone(),
                    old_state,
                    new_state: info.state,
                });
            }
            
            if old_role != info.role {
                let _ = self.event_tx.send(ClusterEvent::NodeRoleChanged {
                    node_id: node_id.clone(),
                    old_role,
                    new_role: info.role,
                });
            }
        } else {
            info.joined_at = Utc::now();
            info.last_heartbeat = Utc::now();
            
            info!("Node {} joined the cluster at {}", node_id, info.cluster_addr);
            
            self.nodes.insert(node_id.clone(), info.clone());
            let _ = self.event_tx.send(ClusterEvent::NodeJoined(info));
        }
    }

    /// Remove a node from the cluster.
    pub async fn remove_node(&self, node_id: &NodeId) {
        if *node_id == self.local_id() {
            return;
        }

        if self.nodes.remove(node_id).is_some() {
            info!("Node {} left the cluster", node_id);
            let _ = self.event_tx.send(ClusterEvent::NodeLeft(node_id.clone()));
            self.metrics.remove(node_id);
        }
    }

    /// Get a node by ID.
    pub fn get_node(&self, node_id: &NodeId) -> Option<NodeInfo> {
        self.nodes.get(node_id).map(|n| n.clone())
    }

    /// Get all nodes in the cluster.
    pub fn all_nodes(&self) -> Vec<NodeInfo> {
        self.nodes.iter().map(|n| n.clone()).collect()
    }

    /// Get healthy nodes (alive and not offline).
    pub fn healthy_nodes(&self) -> Vec<NodeInfo> {
        self.nodes
            .iter()
            .filter(|n| n.is_alive(30)) // 30 second threshold
            .map(|n| n.clone())
            .collect()
    }

    /// Get worker nodes.
    pub fn worker_nodes(&self) -> Vec<NodeInfo> {
        self.nodes
            .iter()
            .filter(|n| matches!(n.role, NodeRole::Worker) && n.is_alive(30))
            .map(|n| n.clone())
            .collect()
    }

    /// Get the number of nodes in the cluster.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the quorum size (majority of nodes).
    pub fn quorum_size(&self) -> usize {
        (self.nodes.len() / 2) + 1
    }

    /// Check if quorum is reached with the given node count.
    pub fn has_quorum(&self, available: usize) -> bool {
        available >= self.quorum_size()
    }

    /// Check if the cluster has quorum with current healthy nodes.
    pub fn has_quorum_now(&self) -> bool {
        self.has_quorum(self.healthy_nodes().len())
    }

    /// Update node metrics.
    pub fn update_metrics(&self, node_id: NodeId, metrics: NodeMetrics) {
        self.metrics.insert(node_id.clone(), metrics.clone());
        let _ = self.event_tx.send(ClusterEvent::MetricsUpdated { node_id, metrics });
    }

    /// Get node metrics.
    pub fn get_metrics(&self, node_id: &NodeId) -> Option<NodeMetrics> {
        self.metrics.get(node_id).map(|m| m.clone())
    }

    /// Get all metrics.
    pub fn all_metrics(&self) -> HashMap<NodeId, NodeMetrics> {
        self.metrics
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Subscribe to cluster events.
    pub fn subscribe(&self) -> broadcast::Receiver<ClusterEvent> {
        self.event_tx.subscribe()
    }

    /// Get the event sender.
    pub fn event_sender(&self) -> broadcast::Sender<ClusterEvent> {
        self.event_tx.clone()
    }

    /// Bootstrap the cluster as leader.
    pub async fn bootstrap(&self) -> Result<()> {
        info!("Bootstrapping cluster as leader");
        
        self.local_node.set_role(NodeRole::Leader).await;
        self.local_node.set_state(NodeState::Healthy).await;
        self.set_leader(Some(self.local_id())).await;
        self.set_state(ClusterState::Active).await;
        
        // Update local node info in nodes map
        let local_info = self.local_node.info().await;
        self.nodes.insert(self.local_id(), local_info);
        
        info!("Cluster {} bootstrapped successfully", self.cluster_id);
        Ok(())
    }

    /// Join an existing cluster.
    pub async fn join_cluster(&self, leader_addr: SocketAddr) -> Result<()> {
        info!("Joining cluster via leader at {}", leader_addr);
        
        self.local_node.set_state(NodeState::Joining).await;
        
        // This will be implemented with actual gRPC call in coordinator
        // For now, just set state
        self.local_node.set_role(NodeRole::Worker).await;
        self.local_node.set_state(NodeState::Healthy).await;
        
        info!("Successfully joined cluster");
        Ok(())
    }

    /// Leave the cluster gracefully.
    pub async fn leave(&self) -> Result<()> {
        info!("Leaving cluster gracefully");
        
        self.set_state(ClusterState::ShuttingDown).await;
        self.local_node.set_state(NodeState::Leaving).await;
        
        // If we're the leader, transfer leadership
        if self.is_leader().await {
            warn!("Leader is leaving - triggering election");
            self.set_leader(None).await;
        }
        
        Ok(())
    }

    /// Start background maintenance tasks.
    pub fn start_maintenance(self: Arc<Self>) {
        tokio::spawn(self.clone().run_maintenance());
    }

    /// Run maintenance tasks.
    async fn run_maintenance(self: Arc<Self>) {
        let mut interval = interval(Duration::from_secs(5));
        
        loop {
            interval.tick().await;
            
            if matches!(self.state().await, ClusterState::ShuttingDown) {
                break;
            }
            
            self.cleanup_dead_nodes().await;
            self.check_cluster_health().await;
        }
    }

    /// Clean up nodes that haven't heartbeated recently.
    async fn cleanup_dead_nodes(&self) {
        let dead_threshold = self.config.health.dead_node_timeout_secs as i64;
        let mut dead_nodes = Vec::new();
        
        for entry in self.nodes.iter() {
            let node = entry.value();
            if node.id == self.local_id() {
                continue;
            }
            
            let elapsed = Utc::now().signed_duration_since(node.last_heartbeat);
            if elapsed.num_seconds() > dead_threshold {
                warn!(
                    "Node {} has been unresponsive for {} seconds, marking as dead",
                    node.id,
                    elapsed.num_seconds()
                );
                dead_nodes.push(node.id.clone());
            }
        }
        
        for node_id in dead_nodes {
            self.remove_node(&node_id).await;
        }
    }

    /// Check overall cluster health.
    async fn check_cluster_health(&self) {
        let healthy_count = self.healthy_nodes().len();
        let total_count = self.node_count();
        
        let current_state = self.state().await;
        let new_state = if healthy_count == total_count {
            ClusterState::Active
        } else if self.has_quorum(healthy_count) {
            ClusterState::Degraded
        } else {
            ClusterState::Partitioned
        };
        
        if current_state != new_state {
            warn!(
                "Cluster health changed: {}/{} nodes healthy, state: {} -> {}",
                healthy_count, total_count, current_state, new_state
            );
            self.set_state(new_state).await;
        }
    }

    /// Get cluster status summary.
    pub async fn status(&self) -> ClusterStatus {
        ClusterStatus {
            cluster_id: self.cluster_id.clone(),
            state: self.state().await,
            leader_id: self.leader_id().await,
            local_id: self.local_id(),
            total_nodes: self.node_count(),
            healthy_nodes: self.healthy_nodes().len(),
            term: self.current_term(),
        }
    }
}

/// Cluster status summary.
#[derive(Debug, Clone)]
pub struct ClusterStatus {
    pub cluster_id: String,
    pub state: ClusterState,
    pub leader_id: Option<NodeId>,
    pub local_id: NodeId,
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub term: u64,
}

impl std::fmt::Display for ClusterStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cluster {}: {} ({} healthy/{}) | Leader: {:?} | Term: {}",
            self.cluster_id,
            self.state,
            self.healthy_nodes,
            self.total_nodes,
            self.leader_id,
            self.term
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    fn create_test_local_node(id: &str) -> Arc<LocalNode> {
        let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
        let info = NodeInfo::new(
            id,
            format!("node-{}", id),
            addr,
            addr,
            NodeRole::Worker,
        );
        Arc::new(LocalNode::new(info))
    }

    fn create_test_config() -> DistributedConfig {
        DistributedConfig::default()
    }

    #[tokio::test]
    async fn test_cluster_creation() {
        let local = create_test_local_node("test-1");
        let config = create_test_config();
        let cluster = Cluster::new(local, config);

        assert_eq!(cluster.node_count(), 1);
        assert_eq!(cluster.local_id(), "test-1");
    }

    #[tokio::test]
    async fn test_quorum_calculation() {
        let local = create_test_local_node("test-1");
        let config = create_test_config();
        let cluster = Cluster::new(local, config);

        // With 1 node, quorum is 1
        assert_eq!(cluster.quorum_size(), 1);
        assert!(cluster.has_quorum_now());
    }

    #[tokio::test]
    async fn test_leader_management() {
        let local = create_test_local_node("test-1");
        let config = create_test_config();
        let cluster = Cluster::new(local, config);

        assert!(cluster.leader_id().await.is_none());

        cluster.set_leader(Some("test-1".to_string())).await;
        assert_eq!(cluster.leader_id().await, Some("test-1".to_string()));
    }
}

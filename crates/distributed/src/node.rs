//! Node types and management for the distributed cluster.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;

/// Unique identifier for a node in the cluster.
pub type NodeId = String;

/// The role of a node in the cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRole {
    /// The leader node manages the cluster and coordinates workers.
    Leader,
    /// A worker node executes tasks and handles sessions.
    Worker,
    /// A candidate node is competing for leadership.
    Candidate,
    /// A learner node is catching up but doesn't vote.
    Learner,
}

impl std::fmt::Display for NodeRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeRole::Leader => write!(f, "leader"),
            NodeRole::Worker => write!(f, "worker"),
            NodeRole::Candidate => write!(f, "candidate"),
            NodeRole::Learner => write!(f, "learner"),
        }
    }
}

/// The current state of a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeState {
    /// Node is starting up.
    Initializing,
    /// Node is healthy and operational.
    Healthy,
    /// Node is experiencing issues but still operational.
    Degraded,
    /// Node is unhealthy but still part of the cluster.
    Unhealthy,
    /// Node is offline or unreachable.
    Offline,
    /// Node is in the process of joining the cluster.
    Joining,
    /// Node is gracefully leaving the cluster.
    Leaving,
}

impl std::fmt::Display for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeState::Initializing => write!(f, "initializing"),
            NodeState::Healthy => write!(f, "healthy"),
            NodeState::Degraded => write!(f, "degraded"),
            NodeState::Unhealthy => write!(f, "unhealthy"),
            NodeState::Offline => write!(f, "offline"),
            NodeState::Joining => write!(f, "joining"),
            NodeState::Leaving => write!(f, "leaving"),
        }
    }
}

/// Information about a node in the cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Unique node identifier.
    pub id: NodeId,
    /// Human-readable node name.
    pub name: String,
    /// Address the node listens on for cluster communication.
    pub cluster_addr: SocketAddr,
    /// Address the node listens on for client API requests.
    pub api_addr: SocketAddr,
    /// Current role of the node.
    pub role: NodeRole,
    /// Current state of the node.
    pub state: NodeState,
    /// Node capabilities and metadata.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// When the node joined the cluster.
    pub joined_at: DateTime<Utc>,
    /// Last heartbeat timestamp.
    pub last_heartbeat: DateTime<Utc>,
    /// Current term in Raft consensus.
    #[serde(default)]
    pub term: u64,
    /// Raft log index (if applicable).
    #[serde(default)]
    pub log_index: u64,
}

impl NodeInfo {
    /// Create new node info with the given parameters.
    pub fn new(
        id: impl Into<NodeId>,
        name: impl Into<String>,
        cluster_addr: SocketAddr,
        api_addr: SocketAddr,
        role: NodeRole,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            name: name.into(),
            cluster_addr,
            api_addr,
            role,
            state: NodeState::Initializing,
            metadata: HashMap::new(),
            joined_at: now,
            last_heartbeat: now,
            term: 0,
            log_index: 0,
        }
    }

    /// Update the heartbeat timestamp.
    pub fn touch(&mut self) {
        self.last_heartbeat = Utc::now();
    }

    /// Check if the node appears to be alive (heartbeat within threshold).
    pub fn is_alive(&self, threshold_secs: i64) -> bool {
        let elapsed = Utc::now().signed_duration_since(self.last_heartbeat);
        elapsed.num_seconds() < threshold_secs
            && !matches!(self.state, NodeState::Offline | NodeState::Leaving)
    }

    /// Get a display string for the node.
    pub fn display(&self) -> String {
        format!(
            "{} ({}) - {} [{}]",
            self.name, self.id, self.role, self.state
        )
    }
}

/// Local node state and management.
pub struct LocalNode {
    /// Stable node ID cache.
    node_id: NodeId,
    /// Node information.
    info: RwLock<NodeInfo>,
    /// Current Raft term.
    current_term: AtomicU64,
    /// Voted for in current term (if any).
    voted_for: RwLock<Option<NodeId>>,
    /// Commit index for Raft log.
    commit_index: AtomicU64,
    /// Last applied log index.
    last_applied: AtomicU64,
}

impl LocalNode {
    /// Create a new local node.
    pub fn new(info: NodeInfo) -> Self {
        Self {
            node_id: info.id.clone(),
            info: RwLock::new(info),
            current_term: AtomicU64::new(0),
            voted_for: RwLock::new(None),
            commit_index: AtomicU64::new(0),
            last_applied: AtomicU64::new(0),
        }
    }

    /// Get the node ID.
    pub fn id(&self) -> NodeId {
        self.node_id.clone()
    }

    /// Get a clone of the node info (async).
    pub async fn info(&self) -> NodeInfo {
        self.info.read().await.clone()
    }

    /// Get a clone of the node info (blocking).
    pub fn info_blocking(&self) -> NodeInfo {
        if let Ok(info) = self.info.try_read() {
            return info.clone();
        }

        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::task::block_in_place(|| self.info.blocking_read().clone())
        } else {
            self.info.blocking_read().clone()
        }
    }

    /// Update node state.
    pub async fn set_state(&self, state: NodeState) {
        self.info.write().await.state = state;
    }

    /// Update node role.
    pub async fn set_role(&self, role: NodeRole) {
        self.info.write().await.role = role;
    }

    /// Update heartbeat timestamp.
    pub async fn touch(&self) {
        self.info.write().await.touch();
    }

    /// Get the current term.
    pub fn current_term(&self) -> u64 {
        self.current_term.load(Ordering::SeqCst)
    }

    /// Increment and return the new term.
    pub fn increment_term(&self) -> u64 {
        self.current_term.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Set the current term.
    pub fn set_term(&self, term: u64) {
        self.current_term.store(term, Ordering::SeqCst);
    }

    /// Get who we voted for this term.
    pub async fn voted_for(&self) -> Option<NodeId> {
        self.voted_for.read().await.clone()
    }

    /// Record a vote for a candidate.
    pub async fn record_vote(&self, candidate_id: NodeId) {
        *self.voted_for.write().await = Some(candidate_id);
    }

    /// Clear vote (used when term changes).
    pub async fn clear_vote(&self) {
        *self.voted_for.write().await = None;
    }

    /// Get commit index.
    pub fn commit_index(&self) -> u64 {
        self.commit_index.load(Ordering::SeqCst)
    }

    /// Set commit index.
    pub fn set_commit_index(&self, index: u64) {
        self.commit_index.store(index, Ordering::SeqCst);
    }

    /// Get last applied index.
    pub fn last_applied(&self) -> u64 {
        self.last_applied.load(Ordering::SeqCst)
    }

    /// Set last applied index.
    pub fn set_last_applied(&self, index: u64) {
        self.last_applied.store(index, Ordering::SeqCst);
    }

    /// Update metadata.
    pub async fn update_metadata(&self, key: impl Into<String>, value: impl Into<String>) {
        self.info
            .write()
            .await
            .metadata
            .insert(key.into(), value.into());
    }

    /// Check if this node is the leader.
    pub async fn is_leader(&self) -> bool {
        self.info.read().await.role == NodeRole::Leader
    }

    /// Check if this node is a worker.
    pub async fn is_worker(&self) -> bool {
        self.info.read().await.role == NodeRole::Worker
    }
}

// Note: NodeInfo derives Clone via serde, no manual impl needed

/// Metrics for a node.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeMetrics {
    /// CPU usage percentage (0-100).
    pub cpu_usage: f64,
    /// Memory usage percentage (0-100).
    pub memory_usage: f64,
    /// Number of active sessions.
    pub active_sessions: u64,
    /// Number of running tasks.
    pub running_tasks: u64,
    /// Total requests handled.
    pub total_requests: u64,
    /// Requests per second.
    pub requests_per_second: f64,
    /// Average response time in milliseconds.
    pub avg_response_time_ms: f64,
    /// Custom metrics.
    #[serde(default)]
    pub custom: HashMap<String, f64>,
}

impl NodeMetrics {
    /// Calculate a health score (0-100, higher is better).
    pub fn health_score(&self) -> f64 {
        let cpu_score = 100.0 - self.cpu_usage;
        let memory_score = 100.0 - self.memory_usage;
        let load_factor = (self.active_sessions as f64 + self.running_tasks as f64) / 100.0;
        let load_score = (100.0 - load_factor * 10.0).max(0.0);

        (cpu_score + memory_score + load_score) / 3.0
    }

    /// Check if node is overloaded.
    pub fn is_overloaded(&self) -> bool {
        self.cpu_usage > 90.0 || self.memory_usage > 90.0 || self.health_score() < 30.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_role_display() {
        assert_eq!(NodeRole::Leader.to_string(), "leader");
        assert_eq!(NodeRole::Worker.to_string(), "worker");
    }

    #[test]
    fn test_node_state_display() {
        assert_eq!(NodeState::Healthy.to_string(), "healthy");
        assert_eq!(NodeState::Offline.to_string(), "offline");
    }

    #[test]
    fn test_node_metrics_health_score() {
        let metrics = NodeMetrics {
            cpu_usage: 50.0,
            memory_usage: 50.0,
            active_sessions: 10,
            running_tasks: 5,
            ..Default::default()
        };
        let score = metrics.health_score();
        assert!(score > 0.0 && score <= 100.0);
    }

    #[test]
    fn test_node_metrics_overloaded() {
        let normal = NodeMetrics {
            cpu_usage: 50.0,
            memory_usage: 50.0,
            ..Default::default()
        };
        assert!(!normal.is_overloaded());

        let overloaded = NodeMetrics {
            cpu_usage: 95.0,
            memory_usage: 50.0,
            ..Default::default()
        };
        assert!(overloaded.is_overloaded());
    }
}

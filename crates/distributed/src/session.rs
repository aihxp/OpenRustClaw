//! Distributed session management.

use crate::cluster::Cluster;
use crate::config::DistributedConfig;
use crate::error::{DistributedError, Result};
use crate::load_balancer::{LoadBalancer, RouteResult, SessionRouter};
use crate::node::{NodeId, NodeInfo, NodeRole};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// A distributed session that can be migrated between nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedSession {
    /// Unique session identifier.
    pub id: String,
    /// User ID associated with the session.
    pub user_id: String,
    /// Workspace ID for multi-tenancy.
    pub workspace_id: Option<String>,
    /// ID of the node currently hosting this session.
    pub assigned_node: NodeId,
    /// Session state.
    pub state: SessionState,
    /// Session creation time.
    pub created_at: DateTime<Utc>,
    /// Last access time.
    pub last_accessed: DateTime<Utc>,
    /// Session metadata.
    pub metadata: HashMap<String, String>,
    /// Serialized session data.
    pub data: Vec<u8>,
}

/// Session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// Session is active and can handle requests.
    Active,
    /// Session is being migrated to another node.
    Migrating,
    /// Session is suspended (no active connection).
    Suspended,
    /// Session is closed.
    Closed,
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionState::Active => write!(f, "active"),
            SessionState::Migrating => write!(f, "migrating"),
            SessionState::Suspended => write!(f, "suspended"),
            SessionState::Closed => write!(f, "closed"),
        }
    }
}

/// Session manager for handling distributed sessions.
pub struct SessionManager {
    /// Local node ID.
    local_id: NodeId,
    /// Cluster reference.
    cluster: Arc<Cluster>,
    /// Load balancer.
    load_balancer: Arc<LoadBalancer>,
    /// Local sessions (sessions hosted on this node).
    local_sessions: DashMap<String, DistributedSession>,
    /// Session router.
    router: SessionRouter,
    /// Configuration.
    config: DistributedConfig,
    /// Session timeout in seconds.
    session_timeout_secs: u64,
}

impl SessionManager {
    /// Create a new session manager.
    pub fn new(
        local_id: NodeId,
        cluster: Arc<Cluster>,
        load_balancer: Arc<LoadBalancer>,
        config: DistributedConfig,
    ) -> Arc<Self> {
        let router = SessionRouter::new(
            load_balancer.clone(),
            local_id.clone(),
            crate::load_balancer::ProxyConfig::default(),
        );

        let manager = Arc::new(Self {
            local_id,
            cluster,
            load_balancer,
            local_sessions: DashMap::new(),
            router,
            session_timeout_secs: config.load_balancer.session_timeout_secs,
            config,
        });

        // Start background cleanup
        manager.clone().start_cleanup_task();

        manager
    }

    /// Create a new session.
    pub async fn create_session(
        &self,
        user_id: impl Into<String>,
        workspace_id: Option<String>,
    ) -> Result<DistributedSession> {
        let session_id = Uuid::new_v4().to_string();
        let user_id = user_id.into();

        // Select a node for the session
        let workers = self.cluster.worker_nodes();
        let target_node = if self.cluster.is_leader().await && workers.is_empty() {
            // If we're the leader and no workers, host locally
            self.cluster.local_node().info().await
        } else if !workers.is_empty() {
            self.load_balancer
                .select_node(&workers, Some(&session_id))
                .await?
        } else {
            return Err(DistributedError::Cluster("No available workers".to_string()));
        };

        let now = Utc::now();
        let session = DistributedSession {
            id: session_id.clone(),
            user_id,
            workspace_id,
            assigned_node: target_node.id.clone(),
            state: SessionState::Active,
            created_at: now,
            last_accessed: now,
            metadata: HashMap::new(),
            data: vec![],
        };

        // If the session is assigned to us, store it locally
        if target_node.id == self.local_id {
            self.local_sessions.insert(session_id.clone(), session.clone());
            info!("Created local session {} for user {}", session_id, session.user_id);
        } else {
            // Otherwise, forward to the target node
            self.forward_create_session(&target_node, &session).await?;
            info!(
                "Created remote session {} on node {} for user {}",
                session_id, target_node.id, session.user_id
            );
        }

        // Record session affinity
        self.load_balancer
            .session_affinity
            .insert(session_id.clone(), target_node.id);

        Ok(session)
    }

    /// Get a session by ID.
    pub async fn get_session(&self, session_id: &str) -> Result<DistributedSession> {
        // First check local sessions
        if let Some(session) = self.local_sessions.get(session_id) {
            let mut session = session.clone();
            session.last_accessed = Utc::now();
            return Ok(session);
        }

        // Check affinity to find which node has the session
        if let Some(node_id) = self.load_balancer.session_affinity.get(session_id) {
            if *node_id.value() != self.local_id {
                // Forward to the node that has the session
                if let Some(node) = self.cluster.get_node(node_id.value()) {
                    return self.forward_get_session(&node, session_id).await;
                }
            }
        }

        Err(DistributedError::SessionNotFound(session_id.to_string()))
    }

    /// Route a session request.
    pub async fn route_session(&self, session_id: &str) -> RouteResult {
        let workers = self.cluster.worker_nodes();
        self.router.route_session(session_id, &workers).await
    }

    /// Update session data.
    pub async fn update_session(
        &self,
        session_id: &str,
        data: Vec<u8>,
    ) -> Result<DistributedSession> {
        // Get the session first to find where it lives
        let mut session = self.get_session(session_id).await?;
        
        session.data = data;
        session.last_accessed = Utc::now();

        // If it's a local session, update it
        if session.assigned_node == self.local_id {
            self.local_sessions.insert(session_id.to_string(), session.clone());
            Ok(session)
        } else {
            // Forward update to the assigned node
            if let Some(node) = self.cluster.get_node(&session.assigned_node) {
                self.forward_update_session(&node, &session).await
            } else {
                Err(DistributedError::NodeNotFound(session.assigned_node))
            }
        }
    }

    /// Migrate a session to a different node.
    pub async fn migrate_session(
        &self,
        session_id: &str,
        target_node_id: &NodeId,
    ) -> Result<DistributedSession> {
        let mut session = self.get_session(session_id).await?;

        if session.assigned_node == target_node_id {
            return Ok(session); // Already on target node
        }

        let target_node = self
            .cluster
            .get_node(target_node_id)
            .ok_or_else(|| DistributedError::NodeNotFound(target_node_id.clone()))?;

        // Mark as migrating
        session.state = SessionState::Migrating;
        if session.assigned_node == self.local_id {
            self.local_sessions.insert(session_id.to_string(), session.clone());
        }

        info!(
            "Migrating session {} from {} to {}",
            session_id, session.assigned_node, target_node_id
        );

        // Transfer session data to target node
        self.forward_create_session(&target_node, &session).await?;

        // Update session assignment
        let old_node_id = session.assigned_node.clone();
        session.assigned_node = target_node_id.to_string();
        session.state = SessionState::Active;
        session.last_accessed = Utc::now();

        // Remove from old node if it was local
        if old_node_id == self.local_id {
            self.local_sessions.remove(session_id);
        } else {
            // Signal old node to remove session
            if let Some(old_node) = self.cluster.get_node(&old_node_id) {
                let _ = self.forward_delete_session(&old_node, session_id).await;
            }
        }

        // Update affinity
        self.load_balancer
            .session_affinity
            .insert(session_id.to_string(), target_node_id.to_string());

        // Store locally if target is us
        if target_node_id == &self.local_id {
            self.local_sessions.insert(session_id.to_string(), session.clone());
        }

        info!("Successfully migrated session {} to {}", session_id, target_node_id);

        Ok(session)
    }

    /// Close a session.
    pub async fn close_session(&self, session_id: &str) -> Result<()> {
        if let Some((_, mut session)) = self.local_sessions.remove(session_id) {
            session.state = SessionState::Closed;
            info!("Closed local session {}", session_id);
        }

        // Clear affinity
        self.load_balancer.clear_session(session_id);

        Ok(())
    }

    /// Get all local sessions.
    pub fn local_sessions(&self) -> Vec<DistributedSession> {
        self.local_sessions
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get local session count.
    pub fn local_session_count(&self) -> usize {
        self.local_sessions.len()
    }

    /// Get sessions by user ID.
    pub fn get_user_sessions(&self, user_id: &str) -> Vec<DistributedSession> {
        self.local_sessions
            .iter()
            .filter(|entry| entry.value().user_id == user_id)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Start background cleanup task.
    fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                self.cleanup_expired_sessions().await;
            }
        });
    }

    /// Clean up expired sessions.
    async fn cleanup_expired_sessions(&self) {
        let now = Utc::now();
        let timeout = chrono::Duration::seconds(self.session_timeout_secs as i64);
        
        let expired: Vec<String> = self
            .local_sessions
            .iter()
            .filter(|entry| {
                let session = entry.value();
                now.signed_duration_since(session.last_accessed) > timeout
            })
            .map(|entry| entry.key().clone())
            .collect();

        for session_id in expired {
            warn!("Session {} expired, closing", session_id);
            let _ = self.close_session(&session_id).await;
        }
    }

    /// Handle a session creation forwarded from another node.
    pub async fn handle_create_session(&self, session: DistributedSession) -> Result<()> {
        self.local_sessions.insert(session.id.clone(), session);
        Ok(())
    }

    /// Handle a session update forwarded from another node.
    pub async fn handle_update_session(&self, session: DistributedSession) -> Result<()> {
        if self.local_sessions.contains_key(&session.id) {
            self.local_sessions.insert(session.id.clone(), session);
            Ok(())
        } else {
            Err(DistributedError::SessionNotFound(session.id))
        }
    }

    /// Handle a session deletion forwarded from another node.
    pub async fn handle_delete_session(&self, session_id: &str) -> Result<()> {
        self.local_sessions.remove(session_id);
        Ok(())
    }

    // Placeholder methods for forwarding to remote nodes
    async fn forward_create_session(&self, node: &NodeInfo, session: &DistributedSession) -> Result<()> {
        // This would use gRPC to forward to the target node
        // For now, just simulate success
        debug!("Forwarding session creation to node {}", node.id);
        Ok(())
    }

    async fn forward_get_session(&self, node: &NodeInfo, session_id: &str) -> Result<DistributedSession> {
        // This would use gRPC to fetch from the target node
        Err(DistributedError::SessionNotFound(session_id.to_string()))
    }

    async fn forward_update_session(&self, node: &NodeInfo, session: &DistributedSession) -> Result<DistributedSession> {
        // This would use gRPC to update on the target node
        Ok(session.clone())
    }

    async fn forward_delete_session(&self, node: &NodeInfo, session_id: &str) -> Result<()> {
        // This would use gRPC to delete on the target node
        Ok(())
    }
}

/// Session replication manager for cross-node session backup.
pub struct SessionReplication {
    cluster: Arc<Cluster>,
    /// Replication factor (how many copies of each session).
    replication_factor: usize,
    /// Primary sessions (this node is primary).
    primary_sessions: DashMap<String, DistributedSession>,
    /// Replica sessions (this node holds a replica).
    replica_sessions: DashMap<String, DistributedSession>,
}

impl SessionReplication {
    /// Create a new session replication manager.
    pub fn new(cluster: Arc<Cluster>, replication_factor: usize) -> Self {
        Self {
            cluster,
            replication_factor,
            primary_sessions: DashMap::new(),
            replica_sessions: DashMap::new(),
        }
    }

    /// Replicate a session to other nodes.
    pub async fn replicate_session(&self, session: &DistributedSession) -> Result<()> {
        let workers = self.cluster.worker_nodes();
        let target_count = self.replication_factor.saturating_sub(1); // Exclude primary
        
        if target_count == 0 || workers.len() <= 1 {
            return Ok(());
        }

        // Select replica nodes (exclude primary)
        let replica_nodes: Vec<_> = workers
            .into_iter()
            .filter(|n| n.id != session.assigned_node)
            .take(target_count)
            .collect();

        for node in replica_nodes {
            debug!(
                "Replicating session {} to node {}",
                session.id, node.id
            );
            // Would use gRPC to send replica to node
        }

        Ok(())
    }

    /// Get a session from replica if primary is unavailable.
    pub async fn get_session_from_replica(&self, session_id: &str) -> Option<DistributedSession> {
        self.replica_sessions.get(session_id).map(|s| s.clone())
    }

    /// Promote a replica to primary.
    pub async fn promote_replica(&self, session_id: &str) -> Result<DistributedSession> {
        if let Some((_, session)) = self.replica_sessions.remove(session_id) {
            let mut session = session;
            session.assigned_node = self.cluster.local_id();
            self.primary_sessions.insert(session_id.to_string(), session.clone());
            info!("Promoted session {} from replica to primary", session_id);
            Ok(session)
        } else {
            Err(DistributedError::SessionNotFound(session_id.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cluster::Cluster;
    use std::net::SocketAddr;

    fn create_test_cluster() -> (Arc<Cluster>, Arc<LoadBalancer>) {
        let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
        let info = crate::node::NodeInfo::new(
            "test-1",
            "test-node",
            addr,
            addr,
            NodeRole::Leader,
        );
        let local = Arc::new(crate::node::LocalNode::new(info));
        let config = DistributedConfig::default();
        let cluster = Cluster::new(local, config.clone());
        let lb = LoadBalancer::new(config.load_balancer.clone());
        (cluster, lb)
    }

    #[tokio::test]
    async fn test_session_creation() {
        let (cluster, lb) = create_test_cluster();
        let config = DistributedConfig::default();
        
        let manager = SessionManager::new(
            "test-1".to_string(),
            cluster,
            lb,
            config,
        );

        let session = manager.create_session("user-1", None).await.unwrap();
        assert_eq!(session.user_id, "user-1");
        assert_eq!(session.state, SessionState::Active);
    }

    #[tokio::test]
    async fn test_local_session_storage() {
        let (cluster, lb) = create_test_cluster();
        let config = DistributedConfig::default();
        
        let manager = SessionManager::new(
            "test-1".to_string(),
            cluster,
            lb,
            config,
        );

        let session = manager.create_session("user-1", None).await.unwrap();
        let retrieved = manager.get_session(&session.id).await.unwrap();
        
        assert_eq!(session.id, retrieved.id);
        assert_eq!(session.user_id, retrieved.user_id);
    }
}

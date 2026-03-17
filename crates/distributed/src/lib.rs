//! OpenRustClaw Distributed Mode
//!
//! This crate provides multi-node distributed capabilities for OpenRustClaw,
//! enabling horizontal scaling across multiple machines.
//!
//! # Architecture
//!
//! The distributed system consists of:
//! - **Leader Node**: Manages the cluster, distributes work, and coordinates workers
//! - **Worker Nodes**: Execute tasks, handle sessions, and process requests
//! - **Raft Consensus**: Ensures leader election and state machine replication
//! - **Service Discovery**: Automatic node discovery via etcd, Consul, or gossip
//! - **Distributed Memory**: Shared state via Redis, etcd, or gossip replication
//!
//! # Example Usage
//!
//! ```rust
//! use openrustclaw_distributed::{ClusterManager, NodeConfig, NodeRole};
//!
//! async fn start_cluster() {
//!     let config = NodeConfig {
//!         node_id: Some("node-1".to_string()),
//!         listen_addr: "0.0.0.0:50051".parse().unwrap(),
//!         role: NodeRole::Leader,
//!         ..Default::default()
//!     };
//!
//!     let manager = ClusterManager::new(config).await.unwrap();
//!     manager.start().await.unwrap();
//! }
//! ```

#![doc = include_str!("../README.md")]

pub mod cluster;
pub mod config;
pub mod consensus;
pub mod coordinator;
pub mod discovery;
pub mod error;
pub mod load_balancer;
pub mod memory;
pub mod messaging;
pub mod node;
pub mod session;
pub mod task;
pub mod worker;

// Re-export main types from their defining modules
pub use cluster::{Cluster, ClusterEvent, ClusterState, ClusterStatus};
pub use config::{ConsensusConfig, DiscoveryBackend, DiscoveryConfig, DistributedConfig, GossipConfig, HealthConfig, LoadBalanceStrategy, LoadBalancerConfig, MemoryBackend, MemoryConfig};
pub use consensus::{RaftNode, RaftRole};
pub use coordinator::{Coordinator, CoordinatorStatus};
pub use discovery::{create_discovery, Discovery, DiscoveryEvent, DiscoveryStream};
pub use error::{DistributedError, Result};
pub use load_balancer::{LoadBalancer, SessionRouter};
pub use memory::{create_memory, DistributedMemory};
pub use messaging::{GrpcClientPool, GrpcServer};
pub use node::{LocalNode, NodeId, NodeInfo, NodeMetrics, NodeRole, NodeState};
pub use session::{DistributedSession, SessionManager, SessionState};
pub use task::{Task, TaskExecutor, TaskManager, TaskPriority, TaskResult, TaskState};
pub use worker::{Worker, WorkerStatus};

// NodeConfig is defined in this module (lib.rs), not in config.rs

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

/// The main cluster manager that orchestrates distributed operations.
pub struct ClusterManager {
    /// Local node configuration.
    config: NodeConfig,
    /// Local node.
    local_node: Arc<LocalNode>,
    /// Cluster state.
    cluster: Arc<Cluster>,
    /// Raft consensus.
    raft: Arc<RaftNode>,
    /// Distributed memory.
    memory: Arc<dyn DistributedMemory>,
    /// Discovery service.
    discovery: Arc<dyn Discovery>,
    /// Coordinator (if this node is or becomes leader).
    coordinator: RwLock<Option<Arc<Coordinator>>>,
    /// Worker (if this node is a worker).
    worker: RwLock<Option<Arc<Worker>>>,
    /// gRPC server.
    grpc_server: RwLock<Option<tokio::task::JoinHandle<()>>>,
}

/// Node configuration for the cluster manager.
#[derive(Debug, Clone)]
pub struct NodeConfig {
    /// Node ID (auto-generated if not provided).
    pub node_id: Option<String>,
    /// Address to listen on for cluster communication.
    pub listen_addr: SocketAddr,
    /// Address for client API requests.
    pub api_addr: SocketAddr,
    /// Node role.
    pub role: NodeRole,
    /// Distributed configuration.
    pub distributed: DistributedConfig,
    /// Discovery configuration.
    pub discovery: DiscoveryConfig,
    /// Memory configuration.
    pub memory: MemoryConfig,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_id: None,
            listen_addr: "0.0.0.0:50051".parse().unwrap(),
            api_addr: "0.0.0.0:8080".parse().unwrap(),
            role: NodeRole::Worker,
            distributed: DistributedConfig::default(),
            discovery: DiscoveryConfig::default(),
            memory: MemoryConfig::default(),
        }
    }
}

impl ClusterManager {
    /// Create a new cluster manager.
    pub async fn new(config: NodeConfig) -> Result<Arc<Self>> {
        // Generate node ID if not provided
        let node_id = config
            .node_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // Create local node info
        let node_info = NodeInfo::new(
            &node_id,
            format!("openrustclaw-{}", &node_id[..8]),
            config.listen_addr,
            config.api_addr,
            config.role,
        );

        let local_node = Arc::new(LocalNode::new(node_info));

        // Create distributed memory
        let memory = create_memory(&config.memory).await?;

        // Create cluster
        let cluster = Cluster::new(local_node.clone(), config.distributed.clone());

        // Create Raft node
        let raft = RaftNode::new(
            local_node.clone(),
            cluster.clone(),
            config.distributed.consensus.clone(),
        );

        // Create discovery service
        let discovery = create_discovery(&config.discovery, node_id).await?;

        Ok(Arc::new(Self {
            config,
            local_node,
            cluster,
            raft,
            memory,
            discovery,
            coordinator: RwLock::new(None),
            worker: RwLock::new(None),
            grpc_server: RwLock::new(None),
        }))
    }

    /// Start the cluster manager.
    pub async fn start(self: Arc<Self>) -> Result<()> {
        info!(
            "Starting OpenRustClaw cluster manager (node: {}, role: {:?})",
            self.local_node.id(),
            self.config.role
        );

        // Start gRPC server
        self.start_grpc_server().await?;

        // Start based on role
        match self.config.role {
            NodeRole::Leader => {
                self.start_as_leader().await?;
            }
            NodeRole::Worker => {
                self.start_as_worker().await?;
            }
            NodeRole::Candidate | NodeRole::Learner => {
                // Start as follower, wait for election
                self.start_as_follower().await?;
            }
        }

        info!("Cluster manager started successfully");
        Ok(())
    }

    /// Stop the cluster manager.
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping cluster manager");

        // Stop worker if running
        if let Some(worker) = self.worker.write().await.take() {
            worker.stop().await?;
        }

        // Stop coordinator if running
        if let Some(coordinator) = self.coordinator.write().await.take() {
            coordinator.stop().await?;
        }

        // Deregister from discovery
        let _ = self.discovery.deregister(&self.local_node.id()).await;

        info!("Cluster manager stopped");
        Ok(())
    }

    /// Start as leader node.
    async fn start_as_leader(self: Arc<Self>) -> Result<()> {
        info!("Starting as leader node");

        let coordinator = Coordinator::new(
            self.local_node.clone(),
            self.cluster.clone(),
            self.raft.clone(),
            self.config.distributed.clone(),
            self.memory.clone(),
            self.discovery.clone(),
        )
        .await?;

        let coordinator_clone = coordinator.clone();
        coordinator_clone.start().await?;
        *self.coordinator.write().await = Some(coordinator);

        Ok(())
    }

    /// Start as worker node.
    async fn start_as_worker(self: Arc<Self>) -> Result<()> {
        info!("Starting as worker node");

        let leader_addr = self
            .config
            .distributed
            .leader_addr
            .ok_or_else(|| DistributedError::Config("Leader address required for worker".to_string()))?;

        // Create a simple task executor (would be provided by the application)
        let executor = Arc::new(DefaultTaskExecutor);

        let worker = Worker::new(
            self.local_node.clone(),
            self.cluster.clone(),
            self.raft.clone(),
            self.config.distributed.clone(),
            leader_addr,
            self.memory.clone(),
            executor,
        )
        .await?;

        let worker_clone = worker.clone();
        worker_clone.start().await?;
        *self.worker.write().await = Some(worker);

        Ok(())
    }

    /// Start as follower (candidate/learner).
    async fn start_as_follower(self: Arc<Self>) -> Result<()> {
        info!("Starting as follower, waiting for leader election");

        // Start Raft to participate in elections
        self.raft.clone().start().await?;

        // Wait for leader to be elected
        let mut attempts = 0;
        loop {
            if self.cluster.leader_id().await.is_some() {
                info!("Leader elected, joining as worker");
                return self.start_as_worker().await;
            }

            attempts += 1;
            if attempts > 30 {
                return Err(DistributedError::Timeout(
                    "No leader elected within timeout".to_string(),
                ));
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    /// Start the gRPC server.
    async fn start_grpc_server(&self) -> Result<()> {
        let server = GrpcServer::new(
            self.cluster.clone(),
            self.raft.clone(),
            self.config.listen_addr,
        );

        let handle = tokio::spawn(async move {
            if let Err(e) = server.start().await {
                error!("gRPC server error: {}", e);
            }
        });

        *self.grpc_server.write().await = Some(handle);

        // Wait a moment for server to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(())
    }

    /// Get the local node ID.
    pub fn node_id(&self) -> NodeId {
        self.local_node.id()
    }

    /// Get cluster status.
    pub async fn cluster_status(&self) -> ClusterStatus {
        self.cluster.status().await
    }

    /// Check if this node is the leader.
    pub async fn is_leader(&self) -> bool {
        self.cluster.is_leader().await
    }

    /// Get coordinator status (if leader).
    pub async fn coordinator_status(&self) -> Option<CoordinatorStatus> {
        if let Some(coordinator) = self.coordinator.read().await.as_ref() {
            Some(coordinator.status().await)
        } else {
            None
        }
    }

    /// Get worker status (if worker).
    pub async fn worker_status(&self) -> Option<WorkerStatus> {
        if let Some(worker) = self.worker.read().await.as_ref() {
            Some(worker.status().await)
        } else {
            None
        }
    }

    /// Scale workers (leader only).
    pub async fn scale_workers(&self, target_count: usize) -> Result<()> {
        if let Some(coordinator) = self.coordinator.read().await.as_ref() {
            coordinator.scale_workers(target_count).await
        } else {
            Err(DistributedError::NotLeader(self.cluster.leader_id().await))
        }
    }

    /// Get session manager.
    pub async fn session_manager(&self) -> Option<Arc<SessionManager>> {
        // Try worker first, then coordinator
        if let Some(worker) = self.worker.read().await.as_ref() {
            Some(worker.session_manager())
        } else if let Some(_coordinator) = self.coordinator.read().await.as_ref() {
            // Coordinator doesn't have sessions directly, but could access them
            None
        } else {
            None
        }
    }
}

/// Default task executor for testing.
struct DefaultTaskExecutor;

#[async_trait::async_trait]
impl TaskExecutor for DefaultTaskExecutor {
    async fn execute(&self, task: Task) -> Result<TaskResult> {
        info!("Executing task {} of type {}", task.id, task.task_type);
        
        // Simulate work
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        Ok(TaskResult::success(
            format!("Completed task {}", task.id).into_bytes(),
            100,
            "worker".to_string(),
        ))
    }

    fn can_handle(&self, _task_type: &str) -> bool {
        true // Can handle any task type
    }

    fn supported_types(&self) -> Vec<String> {
        vec!["*".to_string()]
    }
}

/// Builder for cluster manager configuration.
pub struct ClusterManagerBuilder {
    config: NodeConfig,
}

impl ClusterManagerBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            config: NodeConfig::default(),
        }
    }

    /// Set node ID.
    pub fn node_id(mut self, id: impl Into<String>) -> Self {
        self.config.node_id = Some(id.into());
        self
    }

    /// Set listen address.
    pub fn listen_addr(mut self, addr: SocketAddr) -> Self {
        self.config.listen_addr = addr;
        self
    }

    /// Set API address.
    pub fn api_addr(mut self, addr: SocketAddr) -> Self {
        self.config.api_addr = addr;
        self
    }

    /// Set node role.
    pub fn role(mut self, role: NodeRole) -> Self {
        self.config.role = role;
        self
    }

    /// Bootstrap as leader.
    pub fn bootstrap_leader(mut self) -> Self {
        self.config.role = NodeRole::Leader;
        self.config.distributed.bootstrap_leader = true;
        self
    }

    /// Join as worker.
    pub fn join_worker(mut self, leader_addr: SocketAddr) -> Self {
        self.config.role = NodeRole::Worker;
        self.config.distributed.leader_addr = Some(leader_addr);
        self
    }

    /// Use etcd for discovery.
    pub fn with_etcd(mut self, endpoints: Vec<String>) -> Self {
        self.config.discovery.backend = DiscoveryBackend::Etcd;
        self.config.discovery.etcd_endpoints = endpoints;
        self
    }

    /// Use gossip for discovery.
    pub fn with_gossip(mut self) -> Self {
        self.config.discovery.backend = DiscoveryBackend::Gossip;
        self
    }

    /// Use Redis for memory.
    pub fn with_redis(mut self, url: impl Into<String>) -> Self {
        self.config.memory.backend = MemoryBackend::Redis;
        self.config.memory.redis_url = url.into();
        self
    }

    /// Build the cluster manager.
    pub async fn build(self) -> Result<Arc<ClusterManager>> {
        ClusterManager::new(self.config).await
    }
}

impl Default for ClusterManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_manager_builder() {
        let builder = ClusterManagerBuilder::new()
            .node_id("test-node")
            .bootstrap_leader()
            .with_gossip();

        assert_eq!(builder.config.role, NodeRole::Leader);
        assert_eq!(builder.config.discovery.backend, DiscoveryBackend::Gossip);
        assert!(builder.config.distributed.bootstrap_leader);
    }

    #[test]
    fn test_cluster_manager_builder_worker() {
        let leader_addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
        let builder = ClusterManagerBuilder::new()
            .join_worker(leader_addr)
            .with_redis("redis://localhost:6379");

        assert_eq!(builder.config.role, NodeRole::Worker);
        assert_eq!(builder.config.distributed.leader_addr, Some(leader_addr));
    }
}

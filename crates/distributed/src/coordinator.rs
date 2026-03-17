//! Leader coordination logic for the distributed cluster.

use crate::cluster::{Cluster, ClusterEvent, ClusterState};
use crate::config::DistributedConfig;
use crate::consensus::RaftNode;
use crate::discovery::{Discovery, DiscoveryEvent};
use crate::error::Result;
use crate::load_balancer::LoadBalancer;
use crate::memory::DistributedMemory;
use crate::messaging::GrpcClientPool;
use crate::node::{LocalNode, NodeId, NodeInfo, NodeMetrics, NodeRole, NodeState};
use crate::session::SessionManager;
use crate::worker::WorkerManager;
use chrono::Utc;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, sleep, Duration};
use tracing::{debug, error, info, warn};

/// The coordinator (leader) manages the cluster and distributes work.
pub struct Coordinator {
    /// Local node.
    local_node: Arc<LocalNode>,
    /// Cluster state.
    cluster: Arc<Cluster>,
    /// Raft consensus node.
    raft: Arc<RaftNode>,
    /// Configuration.
    config: DistributedConfig,
    /// Load balancer.
    load_balancer: Arc<LoadBalancer>,
    /// Session manager.
    session_manager: Arc<SessionManager>,
    /// Worker manager.
    worker_manager: Arc<WorkerManager>,
    /// Distributed memory.
    memory: Arc<dyn DistributedMemory>,
    /// Discovery service.
    discovery: Arc<dyn Discovery>,
    /// gRPC client pool.
    client_pool: Arc<GrpcClientPool>,
    /// Running flag.
    running: Arc<RwLock<bool>>,
}

impl Coordinator {
    /// Create a new coordinator.
    pub async fn new(
        local_node: Arc<LocalNode>,
        cluster: Arc<Cluster>,
        raft: Arc<RaftNode>,
        config: DistributedConfig,
        memory: Arc<dyn DistributedMemory>,
        discovery: Arc<dyn Discovery>,
    ) -> Result<Arc<Self>> {
        let load_balancer = LoadBalancer::new(config.load_balancer.clone());
        
        let session_manager = SessionManager::new(
            local_node.id(),
            cluster.clone(),
            load_balancer.clone(),
            config.clone(),
        );

        let worker_manager = WorkerManager::new(
            cluster.clone(),
            load_balancer.clone(),
            client_pool.clone(),
        );

        let client_pool = Arc::new(GrpcClientPool::new(5));

        Ok(Arc::new(Self {
            local_node,
            cluster,
            raft,
            config,
            load_balancer,
            session_manager,
            worker_manager,
            memory,
            discovery,
            client_pool,
            running: Arc::new(RwLock::new(false)),
        }))
    }

    /// Start the coordinator.
    pub async fn start(self: Arc<Self>) -> Result<()> {
        info!("Starting coordinator");

        *self.running.write().await = true;

        // Start Raft consensus
        self.raft.start().await?;

        // Bootstrap cluster if configured
        if self.config.bootstrap_leader {
            self.cluster.bootstrap().await?;
        }

        // Register with discovery service
        let local_info = self.local_node.info().await;
        self.discovery.register(&local_info).await?;

        // Start background tasks
        self.clone().start_background_tasks();

        info!("Coordinator started successfully");
        Ok(())
    }

    /// Stop the coordinator.
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping coordinator");
        *self.running.write().await = false;

        // Deregister from discovery
        let _ = self.discovery.deregister(&self.local_node.id()).await;

        // Stop Raft
        self.raft.stop().await;

        // Close memory connection
        self.memory.close().await?;

        info!("Coordinator stopped");
        Ok(())
    }

    /// Start background maintenance tasks.
    fn start_background_tasks(self: Arc<Self>) {
        // Cluster membership watcher
        tokio::spawn(self.clone().watch_discovery());
        
        // Cluster event processor
        tokio::spawn(self.clone().process_cluster_events());
        
        // Health monitor
        tokio::spawn(self.clone().monitor_health());
        
        // Load balancer updater
        tokio::spawn(self.clone().update_load_balancer());
        
        // Metrics collector
        tokio::spawn(self.clone().collect_metrics());
    }

    /// Watch for discovery events.
    async fn watch_discovery(self: Arc<Self>) {
        let mut stream = match self.discovery.watch().await {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to start discovery watch: {}", e);
                return;
            }
        };

        while *self.running.read().await {
            match stream.next().await {
                Ok(Some(event)) => {
                    match event {
                        DiscoveryEvent::NodeJoined(node) => {
                            info!("Discovered new node: {} at {}", node.id, node.cluster_addr);
                            self.cluster.upsert_node(node).await;
                        }
                        DiscoveryEvent::NodeLeft(node_id) => {
                            info!("Node left: {}", node_id);
                            self.cluster.remove_node(&node_id).await;
                            self.load_balancer.remove_node(&node_id).await;
                        }
                        DiscoveryEvent::NodeUpdated(node) => {
                            self.cluster.upsert_node(node).await;
                        }
                    }
                }
                Ok(None) => {
                    break;
                }
                Err(e) => {
                    warn!("Discovery watch error: {}", e);
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Process cluster events.
    async fn process_cluster_events(self: Arc<Self>) {
        let mut rx = self.cluster.subscribe();

        while *self.running.read().await {
            match rx.recv().await {
                Ok(event) => {
                    match event {
                        ClusterEvent::NodeJoined(node) => {
                            info!("Node {} joined the cluster", node.id);
                            self.on_node_joined(node).await;
                        }
                        ClusterEvent::NodeLeft(node_id) => {
                            info!("Node {} left the cluster", node_id);
                            self.on_node_left(node_id).await;
                        }
                        ClusterEvent::NodeStateChanged { node_id, old_state, new_state } => {
                            debug!("Node {} state changed: {} -> {}", node_id, old_state, new_state);
                            self.on_node_state_changed(node_id, old_state, new_state).await;
                        }
                        ClusterEvent::LeaderChanged { old_leader, new_leader } => {
                            info!("Leadership changed: {:?} -> {:?}", old_leader, new_leader);
                            self.on_leader_changed(old_leader, new_leader).await;
                        }
                        ClusterEvent::MetricsUpdated { node_id, metrics } => {
                            self.on_metrics_updated(node_id, metrics).await;
                        }
                        _ => {}
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!("Event receiver lagged by {} events", n);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    }

    /// Handle a node joining.
    async fn on_node_joined(&self, node: NodeInfo) {
        // Rebalance sessions if needed
        self.rebalance_sessions().await;
        
        // Update load balancer
        let workers = self.cluster.worker_nodes();
        self.load_balancer.update_nodes(&workers).await;
    }

    /// Handle a node leaving.
    async fn on_node_left(&self, node_id: NodeId) {
        // Migrate sessions from the departed node
        self.migrate_sessions_from_node(&node_id).await;
        
        // Update load balancer
        self.load_balancer.remove_node(&node_id).await;
        
        // Clear client pool entries
        self.client_pool.remove_client(&node_id).await;
    }

    /// Handle node state change.
    async fn on_node_state_changed(&self, node_id: NodeId, old_state: NodeState, new_state: NodeState) {
        match (old_state, new_state) {
            (_, NodeState::Unhealthy) | (_, NodeState::Offline) => {
                // Node became unhealthy, consider migrating sessions
                warn!("Node {} became unhealthy, considering migration", node_id);
            }
            (NodeState::Unhealthy, NodeState::Healthy) => {
                // Node recovered, can use it again
                info!("Node {} recovered", node_id);
            }
            _ => {}
        }
    }

    /// Handle leader change.
    async fn on_leader_changed(&self, old_leader: Option<NodeId>, new_leader: Option<NodeId>) {
        let local_id = self.local_node.id();
        
        // If we became leader
        if new_leader.as_ref() == Some(&local_id) {
            info!("This node became the leader");
            self.local_node.set_role(NodeRole::Leader).await;
            
            // Take over leadership duties
            self.assume_leadership().await;
        }
        
        // If we lost leadership
        if old_leader.as_ref() == Some(&local_id) && new_leader.as_ref() != Some(&local_id) {
            warn!("This node lost leadership");
            self.local_node.set_role(NodeRole::Worker).await;
        }
    }

    /// Handle metrics update.
    async fn on_metrics_updated(&self, node_id: NodeId, metrics: NodeMetrics) {
        // Check if node is overloaded
        if metrics.is_overloaded() {
            warn!("Node {} is overloaded (health score: {:.1})", node_id, metrics.health_score());
            
            // Could trigger auto-scaling or migration here
            self.handle_overloaded_node(&node_id).await;
        }
    }

    /// Assume leadership responsibilities.
    async fn assume_leadership(&self) {
        info!("Assuming leadership responsibilities");
        
        // Sync cluster state
        let _ = self.sync_cluster_state().await;
        
        // Rebalance if needed
        self.rebalance_sessions().await;
    }

    /// Sync cluster state with other nodes.
    async fn sync_cluster_state(&self) -> Result<()> {
        debug!("Syncing cluster state");
        
        // Get nodes from discovery
        let discovered = self.discovery.discover().await?;
        
        for node in discovered {
            if node.id != self.local_node.id() {
                self.cluster.upsert_node(node).await;
            }
        }
        
        Ok(())
    }

    /// Rebalance sessions across workers.
    async fn rebalance_sessions(&self) {
        let workers = self.cluster.worker_nodes();
        if workers.len() < 2 {
            return;
        }

        debug!("Rebalancing sessions across {} workers", workers.len());
        
        // Simple rebalancing: ensure sessions are distributed
        // More sophisticated algorithms could be implemented
        
        // For now, just update the load balancer with current workers
        self.load_balancer.update_nodes(&workers).await;
    }

    /// Migrate sessions from a departed node.
    async fn migrate_sessions_from_node(&self, node_id: &NodeId) {
        warn!("Migrating sessions from departed node {}", node_id);
        
        // In a real implementation, we'd retrieve session data from distributed memory
        // and redistribute to remaining nodes
        
        // For now, clear affinities for this node
        let affinities = self.load_balancer.session_affinities();
        for (session_id, assigned_node) in affinities {
            if &assigned_node == node_id {
                self.load_balancer.clear_session(&session_id);
            }
        }
    }

    /// Handle an overloaded node.
    async fn handle_overloaded_node(&self, node_id: &NodeId) {
        // Could implement session migration here
        // For now, just log it
        debug!("Handling overloaded node: {}", node_id);
    }

    /// Monitor cluster health.
    async fn monitor_health(self: Arc<Self>) {
        let mut interval = interval(Duration::from_secs(10));

        while *self.running.read().await {
            interval.tick().await;

            // Check if we have quorum
            if !self.cluster.has_quorum_now() {
                warn!(
                    "Cluster does not have quorum: {}/{} nodes healthy",
                    self.cluster.healthy_nodes().len(),
                    self.cluster.node_count()
                );
                
                self.cluster.set_state(ClusterState::Partitioned).await;
            }

            // Check for split brain
            self.check_split_brain().await;
        }
    }

    /// Check for split brain condition.
    async fn check_split_brain(&self) {
        // In a real implementation, this would check for multiple leaders
        // through the consensus protocol
    }

    /// Update load balancer with current nodes.
    async fn update_load_balancer(self: Arc<Self>) {
        let mut interval = interval(Duration::from_secs(30));

        while *self.running.read().await {
            interval.tick().await;

            let workers = self.cluster.worker_nodes();
            self.load_balancer.update_nodes(&workers).await;
        }
    }

    /// Collect metrics from workers.
    async fn collect_metrics(self: Arc<Self>) {
        let mut interval = interval(Duration::from_secs(5));

        while *self.running.read().await {
            interval.tick().await;

            // Only leader collects metrics
            if !self.raft.is_leader().await {
                continue;
            }

            // Collect from all workers
            for node in self.cluster.healthy_nodes() {
                if node.id == self.local_node.id() {
                    continue;
                }

                match self.collect_node_metrics(&node).await {
                    Ok(metrics) => {
                        self.cluster.update_metrics(node.id.clone(), metrics);
                    }
                    Err(e) => {
                        debug!("Failed to collect metrics from {}: {}", node.id, e);
                    }
                }
            }
        }
    }

    /// Collect metrics from a specific node.
    async fn collect_node_metrics(&self, node: &NodeInfo) -> Result<NodeMetrics> {
        // This would use gRPC to collect metrics from the node
        // For now, return default
        Ok(NodeMetrics::default())
    }

    /// Scale workers up or down.
    pub async fn scale_workers(&self, target_count: usize) -> Result<()> {
        info!("Scaling workers to {}", target_count);
        
        let current_workers = self.cluster.worker_nodes().len();
        
        if target_count > current_workers {
            // Scale up - this would trigger provisioning
            info!("Scaling up from {} to {} workers", current_workers, target_count);
        } else if target_count < current_workers {
            // Scale down - drain workers first
            info!("Scaling down from {} to {} workers", current_workers, target_count);
            self.drain_workers(current_workers - target_count).await?;
        }
        
        Ok(())
    }

    /// Drain and remove workers.
    async fn drain_workers(&self, count: usize) -> Result<()> {
        let workers = self.cluster.worker_nodes();
        
        for worker in workers.iter().take(count) {
            info!("Draining worker {}", worker.id);
            
            // Migrate sessions away
            self.migrate_sessions_from_node(&worker.id).await;
            
            // Request graceful shutdown
            // This would use gRPC to signal the worker
        }
        
        Ok(())
    }

    /// Get cluster status summary.
    pub async fn status(&self) -> CoordinatorStatus {
        let cluster_status = self.cluster.status().await;
        
        CoordinatorStatus {
            cluster_status,
            worker_count: self.cluster.worker_nodes().len(),
            session_count: self.session_manager.local_session_count(),
            is_leader: self.raft.is_leader().await,
        }
    }
}

/// Coordinator status.
#[derive(Debug, Clone)]
pub struct CoordinatorStatus {
    pub cluster_status: crate::cluster::ClusterStatus,
    pub worker_count: usize,
    pub session_count: usize,
    pub is_leader: bool,
}

impl std::fmt::Display for CoordinatorStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} | Workers: {} | Sessions: {} | Leader: {}",
            self.cluster_status,
            self.worker_count,
            self.session_count,
            self.is_leader
        )
    }
}

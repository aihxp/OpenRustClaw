//! Worker node logic for the distributed cluster.

use crate::cluster::{Cluster, ClusterEvent};
use crate::config::DistributedConfig;
use crate::consensus::RaftNode;
use crate::error::{DistributedError, Result};
use crate::load_balancer::LoadBalancer;
use crate::memory::DistributedMemory;
use crate::messaging::proto;
use crate::messaging::{GrpcClientPool, datetime_to_timestamp};
use crate::node::{LocalNode, NodeId, NodeMetrics, NodeRole, NodeState};
use crate::session::SessionManager;
use crate::task::{Task, TaskExecutor, TaskManager};
use chrono::Utc;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, interval, sleep};
use tonic::transport::Channel;
use tracing::{debug, error, info, warn};

/// A worker node in the cluster.
pub struct Worker {
    /// Local node.
    local_node: Arc<LocalNode>,
    /// Cluster reference.
    cluster: Arc<Cluster>,
    /// Raft consensus node.
    raft: Arc<RaftNode>,
    /// Configuration (reserved for future use).
    #[allow(dead_code)]
    config: DistributedConfig,
    /// Leader address.
    leader_addr: SocketAddr,
    /// Leader client.
    leader_client: RwLock<Option<proto::cluster_service_client::ClusterServiceClient<Channel>>>,
    /// Load balancer (reserved for future use).
    #[allow(dead_code)]
    load_balancer: Arc<LoadBalancer>,
    /// Session manager.
    session_manager: Arc<SessionManager>,
    /// Task manager.
    task_manager: Arc<TaskManager>,
    /// Distributed memory.
    memory: Arc<dyn DistributedMemory>,
    /// gRPC client pool (reserved for future use).
    #[allow(dead_code)]
    client_pool: Arc<GrpcClientPool>,
    /// Running flag.
    running: Arc<RwLock<bool>>,
    /// Task executor.
    task_executor: Arc<dyn TaskExecutor>,
}

impl Worker {
    /// Create a new worker.
    pub async fn new(
        local_node: Arc<LocalNode>,
        cluster: Arc<Cluster>,
        raft: Arc<RaftNode>,
        config: DistributedConfig,
        leader_addr: SocketAddr,
        memory: Arc<dyn DistributedMemory>,
        task_executor: Arc<dyn TaskExecutor>,
    ) -> Result<Arc<Self>> {
        let load_balancer = LoadBalancer::new(config.load_balancer.clone());

        let session_manager = SessionManager::new(
            local_node.id(),
            cluster.clone(),
            load_balancer.clone(),
            config.clone(),
        );

        let task_manager = TaskManager::new(local_node.id(), memory.clone(), task_executor.clone());

        let client_pool = Arc::new(GrpcClientPool::new(5));

        Ok(Arc::new(Self {
            local_node,
            cluster,
            raft,
            config,
            leader_addr,
            leader_client: RwLock::new(None),
            load_balancer,
            session_manager,
            task_manager,
            memory,
            client_pool,
            running: Arc::new(RwLock::new(false)),
            task_executor,
        }))
    }

    /// Start the worker.
    pub async fn start(self: Arc<Self>) -> Result<()> {
        info!("Starting worker node");
        *self.running.write().await = true;

        // Start Raft (as follower)
        self.raft.clone().start().await?;

        // Join the cluster
        self.join_cluster().await?;

        // Start background tasks
        self.clone().start_background_tasks();

        info!("Worker started successfully");
        Ok(())
    }

    /// Stop the worker.
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping worker");
        *self.running.write().await = false;

        // Leave the cluster gracefully
        self.leave_cluster().await?;

        // Stop Raft
        self.raft.stop().await;

        // Stop task manager
        self.task_manager.stop().await?;

        // Close memory connection
        self.memory.close().await?;

        info!("Worker stopped");
        Ok(())
    }

    /// Join the cluster.
    async fn join_cluster(&self) -> Result<()> {
        info!("Joining cluster at leader {}", self.leader_addr);

        // Create connection to leader
        let addr = format!("http://{}", self.leader_addr);
        let channel = Channel::from_shared(addr.clone())
            .map_err(|e| DistributedError::Network(format!("Invalid leader address: {}", e)))?
            .connect()
            .await
            .map_err(|e| {
                DistributedError::Network(format!("Failed to connect to leader: {}", e))
            })?;

        let mut client = proto::cluster_service_client::ClusterServiceClient::new(channel.clone());

        // Send join request
        let local_info = self.local_node.info().await;
        let request = proto::JoinRequest {
            node_id: local_info.id.clone(),
            node_addr: local_info.cluster_addr.to_string(),
            node_type: proto::NodeType::Worker as i32,
            metadata: local_info.metadata.clone(),
        };

        let response = client.join(request).await?.into_inner();

        if response.success {
            info!("Successfully joined cluster");

            // Update local state
            self.local_node.set_role(NodeRole::Worker).await;
            self.local_node.set_state(NodeState::Healthy).await;

            // Add discovered nodes to cluster
            for node_proto in response.nodes {
                if let Some(node) = node_from_proto(node_proto)
                    && node.id != local_info.id
                {
                    self.cluster.upsert_node(node).await;
                }
            }

            // Store leader client
            *self.leader_client.write().await = Some(client);

            // Set cluster leader
            self.cluster.set_leader(Some(response.leader_id)).await;
        } else {
            return Err(DistributedError::Cluster(format!(
                "Failed to join cluster: {}",
                response.error
            )));
        }

        Ok(())
    }

    /// Leave the cluster gracefully.
    async fn leave_cluster(&self) -> Result<()> {
        info!("Leaving cluster gracefully");

        if let Some(client) = self.leader_client.write().await.as_mut() {
            let request = proto::LeaveRequest {
                node_id: self.local_node.id(),
                reason: "Graceful shutdown".to_string(),
            };

            let _ = client.leave(request).await;
        }

        self.local_node.set_state(NodeState::Leaving).await;

        Ok(())
    }

    /// Start background tasks.
    fn start_background_tasks(self: Arc<Self>) {
        // Heartbeat sender
        tokio::spawn(self.clone().send_heartbeats());

        // Cluster event processor
        tokio::spawn(self.clone().process_cluster_events());

        // Task processor
        tokio::spawn(self.clone().process_tasks());

        // Metrics reporter
        tokio::spawn(self.clone().report_metrics());
    }

    /// Send heartbeats to leader.
    async fn send_heartbeats(self: Arc<Self>) {
        let mut interval = interval(Duration::from_secs(5));

        while *self.running.read().await {
            interval.tick().await;

            let leader_id = self.cluster.leader_id().await;

            if leader_id.is_none() {
                // Try to reconnect to leader
                if let Err(e) = self.reconnect_leader().await {
                    debug!("Failed to reconnect to leader: {}", e);
                }
                continue;
            }

            // Send heartbeat
            if let Some(_client) = self.leader_client.write().await.as_mut() {
                let _heartbeat = proto::HeartbeatMessage {
                    node_id: self.local_node.id(),
                    term: self.raft.current_term(),
                    state: proto::NodeState::Healthy as i32,
                    metrics: self.collect_metrics().await,
                    timestamp: datetime_to_timestamp(Utc::now()),
                };

                // For streaming heartbeat, we'd need to maintain a long-lived stream
                // For now, this is simplified
                debug!("Sending heartbeat to leader");
            }
        }
    }

    /// Reconnect to the leader.
    async fn reconnect_leader(&self) -> Result<()> {
        let addr = format!("http://{}", self.leader_addr);
        let channel = Channel::from_shared(addr)
            .map_err(|e| DistributedError::Network(format!("Invalid address: {}", e)))?
            .connect()
            .await
            .map_err(|e| DistributedError::Network(e.to_string()))?;

        let client = proto::cluster_service_client::ClusterServiceClient::new(channel);
        *self.leader_client.write().await = Some(client);

        Ok(())
    }

    /// Process cluster events.
    async fn process_cluster_events(self: Arc<Self>) {
        let mut rx = self.cluster.subscribe();

        while *self.running.read().await {
            match rx.recv().await {
                Ok(event) => {
                    match event {
                        ClusterEvent::LeaderChanged { new_leader, .. } => {
                            if let Some(leader_id) = new_leader
                                && leader_id != self.cluster.local_id()
                            {
                                info!("New leader elected: {}", leader_id);
                                // Update leader connection if needed
                            }
                        }
                        ClusterEvent::NodeLeft(node_id) => {
                            if node_id == self.cluster.local_id() {
                                warn!("We were removed from the cluster");
                            }
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

    /// Process assigned tasks.
    async fn process_tasks(self: Arc<Self>) {
        while *self.running.read().await {
            match self.task_manager.get_next_task().await {
                Ok(Some(task)) => {
                    info!("Processing task {}", task.id);

                    match self.task_executor.execute(task.clone()).await {
                        Ok(result) => {
                            if let Err(e) = self.task_manager.complete_task(&task.id, result).await
                            {
                                error!("Failed to complete task {}: {}", task.id, e);
                            }
                        }
                        Err(e) => {
                            error!("Task {} failed: {}", task.id, e);
                            let _ = self.task_manager.fail_task(&task.id, e.to_string()).await;
                        }
                    }
                }
                Ok(None) => {
                    // No tasks available, sleep briefly
                    sleep(Duration::from_millis(100)).await;
                }
                Err(e) => {
                    error!("Error getting next task: {}", e);
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// Report metrics to leader.
    async fn report_metrics(self: Arc<Self>) {
        let mut interval = interval(Duration::from_secs(10));

        while *self.running.read().await {
            interval.tick().await;

            let metrics = self.gather_metrics().await;
            self.cluster.update_metrics(self.local_node.id(), metrics);
        }
    }

    /// Collect current metrics.
    async fn collect_metrics(&self) -> HashMap<String, proto::MetricValue> {
        let mut metrics = HashMap::new();

        let local_metrics = self.gather_metrics().await;

        metrics.insert(
            "cpu_usage".to_string(),
            proto::MetricValue {
                value: Some(proto::metric_value::Value::FloatValue(
                    local_metrics.cpu_usage,
                )),
            },
        );

        metrics.insert(
            "memory_usage".to_string(),
            proto::MetricValue {
                value: Some(proto::metric_value::Value::FloatValue(
                    local_metrics.memory_usage,
                )),
            },
        );

        metrics.insert(
            "active_sessions".to_string(),
            proto::MetricValue {
                value: Some(proto::metric_value::Value::IntValue(
                    local_metrics.active_sessions as i64,
                )),
            },
        );

        metrics.insert(
            "running_tasks".to_string(),
            proto::MetricValue {
                value: Some(proto::metric_value::Value::IntValue(
                    local_metrics.running_tasks as i64,
                )),
            },
        );

        metrics
    }

    /// Gather detailed metrics.
    async fn gather_metrics(&self) -> NodeMetrics {
        // In a real implementation, this would collect actual system metrics
        NodeMetrics {
            cpu_usage: 30.0,
            memory_usage: 50.0,
            active_sessions: self.session_manager.local_session_count() as u64,
            running_tasks: self.task_manager.running_task_count().await as u64,
            total_requests: 0,
            requests_per_second: 0.0,
            avg_response_time_ms: 0.0,
            custom: HashMap::new(),
        }
    }

    /// Get the session manager.
    pub fn session_manager(&self) -> Arc<SessionManager> {
        self.session_manager.clone()
    }

    /// Get the task manager.
    pub fn task_manager(&self) -> Arc<TaskManager> {
        self.task_manager.clone()
    }

    /// Get worker status.
    pub async fn status(&self) -> WorkerStatus {
        WorkerStatus {
            node_id: self.local_node.id(),
            state: self.local_node.info().await.state,
            leader_id: self.cluster.leader_id().await,
            active_sessions: self.session_manager.local_session_count(),
            running_tasks: self.task_manager.running_task_count().await,
            is_healthy: self.local_node.info().await.state == NodeState::Healthy,
        }
    }
}

/// Worker status.
#[derive(Debug, Clone)]
pub struct WorkerStatus {
    pub node_id: NodeId,
    pub state: NodeState,
    pub leader_id: Option<NodeId>,
    pub active_sessions: usize,
    pub running_tasks: usize,
    pub is_healthy: bool,
}

impl std::fmt::Display for WorkerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Worker {} [{}] | Leader: {:?} | Sessions: {} | Tasks: {} | Healthy: {}",
            self.node_id,
            self.state,
            self.leader_id,
            self.active_sessions,
            self.running_tasks,
            self.is_healthy
        )
    }
}

/// Worker manager (runs on leader to manage workers).
pub struct WorkerManager {
    cluster: Arc<Cluster>,
    load_balancer: Arc<LoadBalancer>,
    #[allow(dead_code)]
    client_pool: Arc<GrpcClientPool>,
}

impl WorkerManager {
    /// Create a new worker manager.
    pub fn new(
        cluster: Arc<Cluster>,
        load_balancer: Arc<LoadBalancer>,
        client_pool: Arc<GrpcClientPool>,
    ) -> Arc<Self> {
        Arc::new(Self {
            cluster,
            load_balancer,
            client_pool,
        })
    }

    /// Assign a task to a worker.
    pub async fn assign_task(&self, task: &Task) -> Result<NodeId> {
        let workers = self.cluster.worker_nodes();

        if workers.is_empty() {
            return Err(DistributedError::Cluster(
                "No available workers".to_string(),
            ));
        }

        // Select worker using load balancer
        let selected = self
            .load_balancer
            .select_node(&workers, Some(&task.session_id))
            .await?;

        debug!("Assigned task {} to worker {}", task.id, selected.id);

        Ok(selected.id)
    }

    /// Get worker capacity.
    pub async fn get_worker_capacity(&self, worker_id: &NodeId) -> Result<WorkerCapacity> {
        let metrics = self
            .cluster
            .get_metrics(worker_id)
            .ok_or_else(|| DistributedError::NodeNotFound(worker_id.clone()))?;

        let capacity = WorkerCapacity {
            available_slots: (100.0 - metrics.cpu_usage) as usize,
            can_accept_tasks: !metrics.is_overloaded(),
            health_score: metrics.health_score(),
        };

        Ok(capacity)
    }
}

/// Worker capacity information.
#[derive(Debug, Clone)]
pub struct WorkerCapacity {
    pub available_slots: usize,
    pub can_accept_tasks: bool,
    pub health_score: f64,
}

/// Convert proto NodeInfo to NodeInfo.
fn node_from_proto(proto: proto::NodeInfo) -> Option<crate::node::NodeInfo> {
    use std::net::SocketAddr;

    let role = match proto::NodeType::try_from(proto.node_type) {
        Ok(proto::NodeType::Leader) => NodeRole::Leader,
        Ok(proto::NodeType::Worker) => NodeRole::Worker,
        _ => NodeRole::Worker,
    };

    let cluster_addr: SocketAddr = proto.node_addr.parse().ok()?;
    let node_id = proto.node_id.clone();

    Some(crate::node::NodeInfo::new(
        node_id,
        proto.node_id,
        cluster_addr,
        cluster_addr,
        role,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_status_display() {
        let status = WorkerStatus {
            node_id: "worker-1".to_string(),
            state: NodeState::Healthy,
            leader_id: Some("leader-1".to_string()),
            active_sessions: 5,
            running_tasks: 3,
            is_healthy: true,
        };

        let display = format!("{}", status);
        assert!(display.contains("worker-1"));
        assert!(display.contains("Healthy"));
    }
}

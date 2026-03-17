//! Inter-node communication via gRPC.

pub mod proto {
    tonic::include_proto!("openrustclaw.distributed");
}

use crate::cluster::Cluster;
use crate::consensus::{AppendEntriesRequest, RaftNode, VoteRequest};
use crate::error::DistributedError;
use crate::node::{NodeId, NodeInfo, NodeRole, NodeState};
use chrono::Utc;
use proto::cluster_service_client::ClusterServiceClient;
use proto::cluster_service_server::{ClusterService, ClusterServiceServer};
use proto::raft_service_client::RaftServiceClient;
use proto::raft_service_server::{RaftService, RaftServiceServer};
use proto::session_service_client::SessionServiceClient;
use proto::task_service_client::TaskServiceClient;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};
use tonic::transport::{Channel, Server};
use tonic::{Request, Response, Status, Streaming};
use tracing::{info, warn};

/// gRPC client pool for connecting to other nodes.
pub struct GrpcClientPool {
    clients: RwLock<HashMap<NodeId, ClusterServiceClient<Channel>>>,
    raft_clients: RwLock<HashMap<NodeId, RaftServiceClient<Channel>>>,
    session_clients: RwLock<HashMap<NodeId, SessionServiceClient<Channel>>>,
    task_clients: RwLock<HashMap<NodeId, TaskServiceClient<Channel>>>,
    connect_timeout: Duration,
}

impl GrpcClientPool {
    /// Create a new client pool.
    pub fn new(connect_timeout_secs: u64) -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
            raft_clients: RwLock::new(HashMap::new()),
            session_clients: RwLock::new(HashMap::new()),
            task_clients: RwLock::new(HashMap::new()),
            connect_timeout: Duration::from_secs(connect_timeout_secs),
        }
    }

    /// Get or create a cluster service client for a node.
    pub async fn get_cluster_client(
        &self,
        node: &NodeInfo,
    ) -> crate::error::Result<ClusterServiceClient<Channel>> {
        let mut clients = self.clients.write().await;
        
        if let Some(client) = clients.get(&node.id) {
            return Ok(client.clone());
        }

        let addr = format!("http://{}", node.cluster_addr);
        let endpoint = Channel::from_shared(addr.clone())
            .map_err(|e| DistributedError::Network(format!("Invalid address: {}", e)))?
            .connect_timeout(self.connect_timeout);

        let channel = timeout(self.connect_timeout, endpoint.connect())
            .await
            .map_err(|_| DistributedError::Timeout("Connection timeout".to_string()))?
            .map_err(|e| DistributedError::Network(format!("Connection failed: {}", e)))?;

        let client = ClusterServiceClient::new(channel);
        clients.insert(node.id.clone(), client.clone());
        
        info!("Created gRPC client for node {} at {}", node.id, addr);
        Ok(client)
    }

    /// Get or create a Raft service client for a node.
    pub async fn get_raft_client(&self, node: &NodeInfo) -> crate::error::Result<RaftServiceClient<Channel>> {
        let mut clients = self.raft_clients.write().await;
        
        if let Some(client) = clients.get(&node.id) {
            return Ok(client.clone());
        }

        let addr = format!("http://{}", node.cluster_addr);
        let endpoint = Channel::from_shared(addr)
            .map_err(|e| DistributedError::Network(format!("Invalid address: {}", e)))?
            .connect_timeout(self.connect_timeout);

        let channel = timeout(self.connect_timeout, endpoint.connect())
            .await
            .map_err(|_| DistributedError::Timeout("Connection timeout".to_string()))?
            .map_err(|e| DistributedError::Network(format!("Connection failed: {}", e)))?;

        let client = RaftServiceClient::new(channel);
        clients.insert(node.id.clone(), client.clone());
        
        Ok(client)
    }

    /// Remove a client from the pool.
    pub async fn remove_client(&self, node_id: &NodeId) {
        self.clients.write().await.remove(node_id);
        self.raft_clients.write().await.remove(node_id);
        self.session_clients.write().await.remove(node_id);
        self.task_clients.write().await.remove(node_id);
    }

    /// Clear all clients.
    pub async fn clear(&self) {
        self.clients.write().await.clear();
        self.raft_clients.write().await.clear();
        self.session_clients.write().await.clear();
        self.task_clients.write().await.clear();
    }
}

/// Cluster service implementation.
pub struct ClusterServiceImpl {
    cluster: Arc<Cluster>,
}

impl ClusterServiceImpl {
    pub fn new(cluster: Arc<Cluster>) -> Self {
        Self { cluster }
    }
}

#[tonic::async_trait]
impl ClusterService for ClusterServiceImpl {
    async fn join(&self, request: Request<proto::JoinRequest>) -> Result<Response<proto::JoinResponse>, Status> {
        let req = request.into_inner();
        
        info!("Node {} attempting to join from {}", req.node_id, req.node_addr);

        // Only leader should handle joins
        if !self.cluster.is_leader().await {
            let leader_id = self.cluster.leader_id().await;
            return Ok(Response::new(proto::JoinResponse {
                success: false,
                leader_id: leader_id.clone().unwrap_or_default(),
                leader_addr: self.cluster.get_node(&leader_id.unwrap_or_default())
                    .map(|n| n.cluster_addr.to_string())
                    .unwrap_or_default(),
                nodes: vec![],
                error: "Not leader".to_string(),
            }));
        }

        // Parse node address
        let cluster_addr: SocketAddr = req.node_addr
            .parse()
            .map_err(|e| Status::invalid_argument(format!("Invalid address: {}", e)))?;

        // Create node info
        let node_type = proto::NodeType::try_from(req.node_type)
            .map_err(|_| Status::invalid_argument("Invalid node type"))?;
        
        let role = match node_type {
            proto::NodeType::Leader => NodeRole::Leader,
            proto::NodeType::Worker => NodeRole::Worker,
            _ => NodeRole::Worker,
        };

        let node_info = NodeInfo::new(
            req.node_id.clone(),
            req.node_id.clone(),
            cluster_addr,
            cluster_addr, // Use same for now
            role,
        );

        // Add to cluster
        self.cluster.upsert_node(node_info.clone()).await;

        // Build response with all nodes
        let nodes: Vec<proto::NodeInfo> = self.cluster.all_nodes()
            .into_iter()
            .map(|n| node_info_to_proto(n))
            .collect();

        let local_info = self.cluster.local_node().info().await;
        
        Ok(Response::new(proto::JoinResponse {
            success: true,
            leader_id: local_info.id,
            leader_addr: local_info.cluster_addr.to_string(),
            nodes,
            error: String::new(),
        }))
    }

    async fn leave(&self, request: Request<proto::LeaveRequest>) -> Result<Response<proto::LeaveResponse>, Status> {
        let req = request.into_inner();
        
        info!("Node {} leaving: {}", req.node_id, req.reason);
        
        self.cluster.remove_node(&req.node_id).await;
        
        Ok(Response::new(proto::LeaveResponse {
            success: true,
        }))
    }

    async fn get_cluster_status(
        &self,
        _request: Request<proto::ClusterStatusRequest>,
    ) -> Result<Response<proto::ClusterStatusResponse>, Status> {
        let status = self.cluster.status().await;
        
        let proto_state = match status.state {
            crate::cluster::ClusterState::Active => proto::ClusterState::Active,
            crate::cluster::ClusterState::Degraded => proto::ClusterState::Degraded,
            crate::cluster::ClusterState::Partitioned => proto::ClusterState::Partitioned,
            _ => proto::ClusterState::Initializing,
        };

        let nodes: Vec<proto::NodeInfo> = self.cluster.all_nodes()
            .into_iter()
            .map(|n| node_info_to_proto(n))
            .collect();

        Ok(Response::new(proto::ClusterStatusResponse {
            cluster_id: status.cluster_id,
            leader_id: status.leader_id.unwrap_or_default(),
            nodes,
            state: proto_state as i32,
            term: status.term,
        }))
    }

    type HeartbeatStream = Pin<Box<dyn Stream<Item = Result<proto::HeartbeatAck, Status>> + Send>>;

    async fn heartbeat(
        &self,
        request: Request<Streaming<proto::HeartbeatMessage>>,
    ) -> Result<Response<Self::HeartbeatStream>, Status> {
        let mut stream = request.into_inner();
        let cluster = self.cluster.clone();
        
        let (tx, rx) = mpsc::channel(100);
        
        tokio::spawn(async move {
            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(heartbeat) => {
                        // Update node info
                        if let Some(mut node) = cluster.get_node(&heartbeat.node_id) {
                            node.last_heartbeat = Utc::now();
                            node.term = heartbeat.term;
                            cluster.upsert_node(node).await;
                        }

                        // Send ack
                        let leader_id = cluster.leader_id().await.unwrap_or_default();
                        let ack = proto::HeartbeatAck {
                            leader_id,
                            term: cluster.current_term(),
                            accepted: true,
                        };
                        
                        if tx.send(Ok(ack)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("Heartbeat stream error: {}", e);
                        break;
                    }
                }
            }
        });

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(output_stream) as Self::HeartbeatStream))
    }
}

/// Raft service implementation.
pub struct RaftServiceImpl {
    raft: Arc<RaftNode>,
}

impl RaftServiceImpl {
    pub fn new(raft: Arc<RaftNode>) -> Self {
        Self { raft }
    }
}

#[tonic::async_trait]
impl RaftService for RaftServiceImpl {
    async fn request_vote(
        &self,
        request: Request<proto::VoteRequest>,
    ) -> Result<Response<proto::VoteResponse>, Status> {
        let req = request.into_inner();
        
        let vote_request = VoteRequest {
            term: req.term,
            candidate_id: req.candidate_id,
            last_log_index: req.last_log_index,
            last_log_term: req.last_log_term,
        };

        let response = self.raft.handle_vote_request(vote_request).await;
        
        Ok(Response::new(proto::VoteResponse {
            term: response.term,
            vote_granted: response.vote_granted,
            voter_id: response.voter_id,
        }))
    }

    async fn append_entries(
        &self,
        request: Request<proto::AppendEntriesRequest>,
    ) -> Result<Response<proto::AppendEntriesResponse>, Status> {
        let req = request.into_inner();
        
        let entries: Vec<crate::consensus::LogEntry> = req.entries
            .into_iter()
            .map(|e| crate::consensus::LogEntry {
                index: e.index,
                term: e.term,
                data: e.data,
                entry_type: match proto::EntryType::try_from(e.entry_type) {
                    Ok(proto::EntryType::Command) => crate::consensus::EntryType::Command,
                    Ok(proto::EntryType::ConfigChange) => crate::consensus::EntryType::ConfigChange,
                    _ => crate::consensus::EntryType::NoOp,
                },
            })
            .collect();

        let append_request = AppendEntriesRequest {
            term: req.term,
            leader_id: req.leader_id,
            prev_log_index: req.prev_log_index,
            prev_log_term: req.prev_log_term,
            entries,
            leader_commit: req.leader_commit,
        };

        let response = self.raft.handle_append_entries(append_request).await;
        
        Ok(Response::new(proto::AppendEntriesResponse {
            term: response.term,
            success: response.success,
            match_index: response.match_index,
            conflict_index: response.conflict_index,
            conflict_term: response.conflict_term,
        }))
    }

    async fn install_snapshot(
        &self,
        request: Request<Streaming<proto::SnapshotChunk>>,
    ) -> Result<Response<proto::InstallSnapshotResponse>, Status> {
        // TODO: Implement snapshot installation
        let _stream = request.into_inner();
        
        Ok(Response::new(proto::InstallSnapshotResponse {
            term: 0,
            success: true,
        }))
    }
}

/// gRPC server manager.
pub struct GrpcServer {
    cluster: Arc<Cluster>,
    raft: Arc<RaftNode>,
    addr: SocketAddr,
}

impl GrpcServer {
    /// Create a new gRPC server.
    pub fn new(cluster: Arc<Cluster>, raft: Arc<RaftNode>, addr: SocketAddr) -> Self {
        Self {
            cluster,
            raft,
            addr,
        }
    }

    /// Start the gRPC server.
    pub async fn start(&self) -> crate::error::Result<()> {
        let cluster_service = ClusterServiceServer::new(ClusterServiceImpl::new(self.cluster.clone()));
        let raft_service = RaftServiceServer::new(RaftServiceImpl::new(self.raft.clone()));

        info!("Starting gRPC server on {}", self.addr);

        Server::builder()
            .add_service(cluster_service)
            .add_service(raft_service)
            .serve(self.addr)
            .await
            .map_err(|e| DistributedError::Network(format!("Server error: {}", e)))?;

        Ok(())
    }
}

/// Convert DateTime<Utc> to prost_types::Timestamp
pub fn datetime_to_timestamp(dt: chrono::DateTime<chrono::Utc>) -> Option<prost_types::Timestamp> {
    Some(prost_types::Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    })
}

/// Helper function to convert NodeInfo to proto NodeInfo.
fn node_info_to_proto(info: NodeInfo) -> proto::NodeInfo {
    let node_type = match info.role {
        NodeRole::Leader => proto::NodeType::Leader,
        NodeRole::Worker => proto::NodeType::Worker,
        NodeRole::Candidate => proto::NodeType::Candidate,
        _ => proto::NodeType::Unspecified,
    };

    let node_state = match info.state {
        NodeState::Healthy => proto::NodeState::Healthy,
        NodeState::Degraded => proto::NodeState::Degraded,
        NodeState::Unhealthy => proto::NodeState::Unhealthy,
        NodeState::Offline => proto::NodeState::Offline,
        NodeState::Joining => proto::NodeState::Joining,
        NodeState::Leaving => proto::NodeState::Leaving,
        _ => proto::NodeState::Unspecified,
    };

    proto::NodeInfo {
        node_id: info.id,
        node_addr: info.cluster_addr.to_string(),
        node_type: node_type as i32,
        state: node_state as i32,
        metadata: info.metadata,
        joined_at: datetime_to_timestamp(info.joined_at),
        last_heartbeat: datetime_to_timestamp(info.last_heartbeat),
    }
}

/// Send heartbeat to a node.
pub async fn send_heartbeat(
    client: &mut ClusterServiceClient<Channel>,
    node_id: &str,
    term: u64,
    state: NodeState,
) -> crate::error::Result<proto::HeartbeatAck> {
    let (tx, rx) = mpsc::channel(1);
    
    let request = tonic::Request::new(ReceiverStream::new(rx));
    
    let heartbeat = proto::HeartbeatMessage {
        node_id: node_id.to_string(),
        term,
        state: match state {
            NodeState::Healthy => proto::NodeState::Healthy,
            _ => proto::NodeState::Unspecified,
        } as i32,
        metrics: HashMap::new(),
        timestamp: datetime_to_timestamp(Utc::now()),
    };
    
    let _ = tx.send(heartbeat).await;
    
    let mut stream = client.heartbeat(request).await?.into_inner();
    
    if let Some(ack) = stream.next().await {
        ack.map_err(|e| e.into())
    } else {
        Err(DistributedError::Messaging("No heartbeat ack".to_string()))
    }
}

/// Join request to a leader.
pub async fn join_cluster(
    client: &mut ClusterServiceClient<Channel>,
    node_id: &str,
    node_addr: &str,
    role: NodeRole,
) -> crate::error::Result<proto::JoinResponse> {
    let node_type = match role {
        NodeRole::Leader => proto::NodeType::Leader,
        NodeRole::Worker => proto::NodeType::Worker,
        _ => proto::NodeType::Unspecified,
    };

    let request = proto::JoinRequest {
        node_id: node_id.to_string(),
        node_addr: node_addr.to_string(),
        node_type: node_type as i32,
        metadata: HashMap::new(),
    };

    let response = client.join(request).await?;
    Ok(response.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_info_conversion() {
        use std::net::SocketAddr;
        
        let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
        let info = NodeInfo::new(
            "test-1",
            "test-node",
            addr,
            addr,
            NodeRole::Worker,
        );

        let proto = node_info_to_proto(info);
        assert_eq!(proto.node_id, "test-1");
        assert_eq!(proto.node_type, proto::NodeType::Worker as i32);
    }
}

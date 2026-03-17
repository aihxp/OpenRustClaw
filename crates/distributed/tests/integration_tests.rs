//! Integration tests for distributed mode.

use openrustclaw_distributed::{
    ClusterManager, ClusterManagerBuilder, NodeRole, NodeState, Task, TaskPriority,
};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::sleep;

/// Test basic cluster manager creation.
#[tokio::test]
async fn test_cluster_manager_builder() {
    let manager = ClusterManagerBuilder::new()
        .node_id("test-node-1")
        .bootstrap_leader()
        .with_gossip()
        .build()
        .await;

    assert!(manager.is_ok());
    let manager = manager.unwrap();
    assert_eq!(manager.node_id(), "test-node-1");
}

/// Test worker configuration.
#[tokio::test]
async fn test_worker_configuration() {
    let leader_addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
    
    let manager = ClusterManagerBuilder::new()
        .node_id("test-worker-1")
        .join_worker(leader_addr)
        .listen_addr("127.0.0.1:50052".parse().unwrap())
        .build()
        .await;

    assert!(manager.is_ok());
}

/// Test task creation.
#[tokio::test]
async fn test_task_creation() {
    use openrustclaw_distributed::task::TaskScheduler;

    let task = TaskScheduler::create_task(
        "test_task",
        b"test payload".to_vec(),
        "session-1",
        TaskPriority::Normal,
    );

    assert_eq!(task.task_type, "test_task");
    assert_eq!(task.session_id, "session-1");
    assert_eq!(task.priority, TaskPriority::Normal);
}

/// Test node info creation.
#[tokio::test]
async fn test_node_info() {
    use openrustclaw_distributed::node::{NodeInfo, NodeRole};

    let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
    let node = NodeInfo::new(
        "node-1",
        "Test Node",
        addr,
        addr,
        NodeRole::Worker,
    );

    assert_eq!(node.id, "node-1");
    assert_eq!(node.name, "Test Node");
    assert_eq!(node.role, NodeRole::Worker);
    assert_eq!(node.state, NodeState::Initializing);
}

/// Test load balancer strategies.
#[tokio::test]
async fn test_load_balancer_round_robin() {
    use openrustclaw_distributed::load_balancer::{LoadBalancer, LoadBalancerConfig, LoadBalanceStrategy};
    use openrustclaw_distributed::node::{NodeInfo, NodeRole};

    let config = LoadBalancerConfig {
        strategy: LoadBalanceStrategy::RoundRobin,
        ..Default::default()
    };

    let balancer = LoadBalancer::new(config);
    
    let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
    let nodes = vec![
        NodeInfo::new("node-1", "Node 1", addr, addr, NodeRole::Worker),
        NodeInfo::new("node-2", "Node 2", addr, addr, NodeRole::Worker),
    ];

    balancer.update_nodes(&nodes).await;

    let first = balancer.select_node(&nodes, None).await.unwrap();
    let second = balancer.select_node(&nodes, None).await.unwrap();

    // Should alternate between nodes
    assert_ne!(first.id, second.id);
}

/// Test consistent hashing.
#[tokio::test]
async fn test_consistent_hashing() {
    use openrustclaw_distributed::load_balancer::{LoadBalancer, LoadBalancerConfig, LoadBalanceStrategy};
    use openrustclaw_distributed::node::{NodeInfo, NodeRole};

    let config = LoadBalancerConfig {
        strategy: LoadBalanceStrategy::ConsistentHash,
        consistent_hashing: true,
        virtual_nodes: 10,
        ..Default::default()
    };

    let balancer = LoadBalancer::new(config);
    
    let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
    let nodes = vec![
        NodeInfo::new("node-1", "Node 1", addr, addr, NodeRole::Worker),
        NodeInfo::new("node-2", "Node 2", addr, addr, NodeRole::Worker),
        NodeInfo::new("node-3", "Node 3", addr, addr, NodeRole::Worker),
    ];

    balancer.update_nodes(&nodes).await;

    // Same session should always map to same node
    let session_id = "test-session-abc123";
    let first = balancer.select_node(&nodes, Some(session_id)).await.unwrap();
    let second = balancer.select_node(&nodes, Some(session_id)).await.unwrap();
    
    assert_eq!(first.id, second.id);
}

/// Test cluster state transitions.
#[tokio::test]
async fn test_cluster_state() {
    use openrustclaw_distributed::cluster::{Cluster, ClusterState};
    use openrustclaw_distributed::config::DistributedConfig;
    use openrustclaw_distributed::node::{LocalNode, NodeInfo, NodeRole};
    use std::sync::Arc;

    let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
    let info = NodeInfo::new("node-1", "Node 1", addr, addr, NodeRole::Leader);
    let local_node = Arc::new(LocalNode::new(info));
    let config = DistributedConfig::default();
    let cluster = Cluster::new(local_node, config);

    // Initial state should be initializing
    assert_eq!(cluster.state().await, ClusterState::Initializing);

    // Bootstrap should set to active
    cluster.bootstrap().await.unwrap();
    assert_eq!(cluster.state().await, ClusterState::Active);
}

/// Test session affinity.
#[tokio::test]
async fn test_session_affinity() {
    use openrustclaw_distributed::load_balancer::{LoadBalancer, LoadBalancerConfig};
    use openrustclaw_distributed::node::{NodeInfo, NodeRole};

    let config = LoadBalancerConfig {
        sticky_sessions: true,
        ..Default::default()
    };

    let balancer = LoadBalancer::new(config);
    
    let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
    let nodes = vec![
        NodeInfo::new("node-1", "Node 1", addr, addr, NodeRole::Worker),
        NodeInfo::new("node-2", "Node 2", addr, addr, NodeRole::Worker),
    ];

    balancer.update_nodes(&nodes).await;

    let session_id = "sticky-session-1";
    
    // First request establishes affinity
    let node1 = balancer.select_node(&nodes, Some(session_id)).await.unwrap();
    
    // Subsequent requests should go to same node
    let node2 = balancer.select_node(&nodes, Some(session_id)).await.unwrap();
    let node3 = balancer.select_node(&nodes, Some(session_id)).await.unwrap();

    assert_eq!(node1.id, node2.id);
    assert_eq!(node2.id, node3.id);
    
    // Verify affinity is stored
    assert!(balancer.session_affinity.contains_key(session_id));
}

/// Test node health scoring.
#[tokio::test]
async fn test_node_health_scoring() {
    use openrustclaw_distributed::node::NodeMetrics;

    let healthy = NodeMetrics {
        cpu_usage: 30.0,
        memory_usage: 40.0,
        active_sessions: 10,
        running_tasks: 5,
        ..Default::default()
    };

    let overloaded = NodeMetrics {
        cpu_usage: 95.0,
        memory_usage: 90.0,
        active_sessions: 100,
        running_tasks: 50,
        ..Default::default()
    };

    assert!(healthy.health_score() > overloaded.health_score());
    assert!(!healthy.is_overloaded());
    assert!(overloaded.is_overloaded());
}

/// Test Raft role transitions.
#[tokio::test]
async fn test_raft_roles() {
    use openrustclaw_distributed::consensus::RaftRole;

    let follower = RaftRole::Follower;
    let candidate = RaftRole::Candidate;
    let leader = RaftRole::Leader;

    assert_ne!(follower, candidate);
    assert_ne!(candidate, leader);
    assert_ne!(follower, leader);

    // Test display
    assert_eq!(follower.to_string(), "follower");
    assert_eq!(candidate.to_string(), "candidate");
    assert_eq!(leader.to_string(), "leader");
}

/// Test task priority ordering.
#[tokio::test]
async fn test_task_priority() {
    use openrustclaw_distributed::TaskPriority;

    assert!(TaskPriority::Critical.value() > TaskPriority::High.value());
    assert!(TaskPriority::High.value() > TaskPriority::Normal.value());
    assert!(TaskPriority::Normal.value() > TaskPriority::Low.value());
}

/// Test cluster quorum calculations.
#[tokio::test]
async fn test_quorum_calculation() {
    use openrustclaw_distributed::cluster::Cluster;
    use openrustclaw_distributed::config::DistributedConfig;
    use openrustclaw_distributed::node::{LocalNode, NodeInfo, NodeRole};
    use std::sync::Arc;

    let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
    let info = NodeInfo::new("node-1", "Node 1", addr, addr, NodeRole::Leader);
    let local_node = Arc::new(LocalNode::new(info));
    let config = DistributedConfig::default();
    let cluster = Cluster::new(local_node, config);

    // With just 1 node
    assert_eq!(cluster.quorum_size(), 1);
    assert!(cluster.has_quorum_now());
}

/// Test distributed configuration defaults.
#[tokio::test]
async fn test_config_defaults() {
    use openrustclaw_distributed::config::DistributedConfig;

    let config = DistributedConfig::default();

    assert!(!config.enabled);
    assert_eq!(config.listen_addr.port(), 50051);
    assert_eq!(config.api_addr.port(), 8080);
    assert!(!config.bootstrap_leader);
}

/// Test memory backends.
#[tokio::test]
async fn test_memory_backends() {
    use openrustclaw_distributed::memory::{GossipMemory, MemoryConfig, MemoryBackend};

    let config = MemoryConfig::default();
    let memory = GossipMemory::new(&config);
    
    assert!(memory.is_ok());
}

/// Test discovery backends.
#[tokio::test]
async fn test_discovery_backends() {
    use openrustclaw_distributed::discovery::{StaticDiscovery, DiscoveryConfig, DiscoveryBackend};

    let config = DiscoveryConfig {
        backend: DiscoveryBackend::Static,
        seed_nodes: vec!["127.0.0.1:50051".to_string()],
        ..DiscoveryConfig::default()
    };

    let discovery = StaticDiscovery::new(&config);
    assert!(discovery.is_ok());
}

/// Test error types.
#[tokio::test]
async fn test_error_types() {
    use openrustclaw_distributed::DistributedError;

    let err1 = DistributedError::LeaderNotAvailable;
    let err2 = DistributedError::NodeNotFound("node-1".to_string());
    let err3 = DistributedError::Timeout("operation timed out".to_string());

    assert!(err1.to_string().contains("Leader not available"));
    assert!(err2.to_string().contains("node-1"));
    assert!(err3.to_string().contains("timed out"));
}

/// Test session state transitions.
#[tokio::test]
async fn test_session_states() {
    use openrustclaw_distributed::session::SessionState;

    assert_eq!(SessionState::Active.to_string(), "active");
    assert_eq!(SessionState::Migrating.to_string(), "migrating");
    assert_eq!(SessionState::Suspended.to_string(), "suspended");
    assert_eq!(SessionState::Closed.to_string(), "closed");
}

/// Test task state transitions.
#[tokio::test]
async fn test_task_states() {
    use openrustclaw_distributed::task::TaskState;

    assert_eq!(TaskState::Pending.to_string(), "pending");
    assert_eq!(TaskState::Running.to_string(), "running");
    assert_eq!(TaskState::Completed.to_string(), "completed");
    assert_eq!(TaskState::Failed.to_string(), "failed");
}

//! Configuration for distributed mode.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;

/// Distributed mode configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedConfig {
    /// Whether distributed mode is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Unique identifier for this node.
    pub node_id: Option<String>,
    /// Human-readable node name.
    pub node_name: Option<String>,
    /// Address to listen on for cluster communication.
    #[serde(default = "default_cluster_addr")]
    pub listen_addr: SocketAddr,
    /// Address to listen on for client API requests.
    #[serde(default = "default_api_addr")]
    pub api_addr: SocketAddr,
    /// Address of the leader node (for workers joining).
    pub leader_addr: Option<SocketAddr>,
    /// Whether this node should bootstrap as leader.
    #[serde(default)]
    pub bootstrap_leader: bool,
    /// Discovery configuration.
    #[serde(default)]
    pub discovery: DiscoveryConfig,
    /// Consensus configuration.
    #[serde(default)]
    pub consensus: ConsensusConfig,
    /// Health check configuration.
    #[serde(default)]
    pub health: HealthConfig,
    /// Load balancer configuration.
    #[serde(default)]
    pub load_balancer: LoadBalancerConfig,
    /// Distributed memory configuration.
    #[serde(default)]
    pub memory: MemoryConfig,
    /// Gossip protocol configuration.
    #[serde(default)]
    pub gossip: GossipConfig,
}

impl Default for DistributedConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            node_id: None,
            node_name: None,
            listen_addr: default_cluster_addr(),
            api_addr: default_api_addr(),
            leader_addr: None,
            bootstrap_leader: false,
            discovery: DiscoveryConfig::default(),
            consensus: ConsensusConfig::default(),
            health: HealthConfig::default(),
            load_balancer: LoadBalancerConfig::default(),
            memory: MemoryConfig::default(),
            gossip: GossipConfig::default(),
        }
    }
}

fn default_enabled() -> bool {
    false
}

fn default_cluster_addr() -> SocketAddr {
    "0.0.0.0:50051".parse().unwrap()
}

fn default_api_addr() -> SocketAddr {
    "0.0.0.0:8080".parse().unwrap()
}

/// Service discovery configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Discovery backend to use.
    #[serde(default)]
    pub backend: DiscoveryBackend,
    /// Seed nodes for initial cluster formation.
    #[serde(default)]
    pub seed_nodes: Vec<String>,
    /// etcd endpoints (if using etcd).
    #[serde(default)]
    pub etcd_endpoints: Vec<String>,
    /// Consul address (if using consul).
    pub consul_addr: Option<String>,
    /// Consul datacenter.
    #[serde(default = "default_consul_dc")]
    pub consul_datacenter: String,
    /// Gossip bind address.
    #[serde(default = "default_gossip_bind")]
    pub gossip_bind: String,
    /// Gossip advertise address.
    pub gossip_advertise: Option<String>,
    /// Cluster name for service discovery.
    #[serde(default = "default_cluster_name")]
    pub cluster_name: String,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            backend: DiscoveryBackend::default(),
            seed_nodes: Vec::new(),
            etcd_endpoints: Vec::new(),
            consul_addr: None,
            consul_datacenter: default_consul_dc(),
            gossip_bind: default_gossip_bind(),
            gossip_advertise: None,
            cluster_name: default_cluster_name(),
        }
    }
}

/// Discovery backend type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryBackend {
    /// Use gossip protocol (SWIM-based) - default.
    #[default]
    /// Use etcd for service discovery.
    Etcd,
    /// Use Consul for service discovery.
    Consul,
    Gossip,
    /// Static configuration only.
    Static,
}

fn default_consul_dc() -> String {
    "dc1".to_string()
}

fn default_gossip_bind() -> String {
    "0.0.0.0:7946".to_string()
}

fn default_cluster_name() -> String {
    "openrustclaw".to_string()
}

/// Raft consensus configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Election timeout in milliseconds.
    #[serde(default = "default_election_timeout_ms")]
    pub election_timeout_ms: u64,
    /// Heartbeat interval in milliseconds.
    #[serde(default = "default_heartbeat_interval_ms")]
    pub heartbeat_interval_ms: u64,
    /// Minimum election timeout (for randomization).
    #[serde(default = "default_min_election_timeout_ms")]
    pub min_election_timeout_ms: u64,
    /// Maximum election timeout (for randomization).
    #[serde(default = "default_max_election_timeout_ms")]
    pub max_election_timeout_ms: u64,
    /// Max inflight append entries.
    #[serde(default = "default_max_inflight")]
    pub max_inflight: usize,
    /// Snapshot interval in entries.
    #[serde(default = "default_snapshot_interval")]
    pub snapshot_interval: u64,
    /// Applied index check interval in milliseconds.
    #[serde(default = "default_applied_index_check_ms")]
    pub applied_index_check_ms: u64,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            election_timeout_ms: default_election_timeout_ms(),
            heartbeat_interval_ms: default_heartbeat_interval_ms(),
            min_election_timeout_ms: default_min_election_timeout_ms(),
            max_election_timeout_ms: default_max_election_timeout_ms(),
            max_inflight: default_max_inflight(),
            snapshot_interval: default_snapshot_interval(),
            applied_index_check_ms: default_applied_index_check_ms(),
        }
    }
}

impl ConsensusConfig {
    /// Get election timeout as Duration.
    pub fn election_timeout(&self) -> Duration {
        Duration::from_millis(self.election_timeout_ms)
    }

    /// Get heartbeat interval as Duration.
    pub fn heartbeat_interval(&self) -> Duration {
        Duration::from_millis(self.heartbeat_interval_ms)
    }
}

fn default_election_timeout_ms() -> u64 {
    1000
}

fn default_heartbeat_interval_ms() -> u64 {
    100
}

fn default_min_election_timeout_ms() -> u64 {
    500
}

fn default_max_election_timeout_ms() -> u64 {
    1500
}

fn default_max_inflight() -> usize {
    256
}

fn default_snapshot_interval() -> u64 {
    10000
}

fn default_applied_index_check_ms() -> u64 {
    100
}

/// Health check configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Health check interval in seconds.
    #[serde(default = "default_health_interval_secs")]
    pub check_interval_secs: u64,
    /// Unhealthy threshold (consecutive failures before marking unhealthy).
    #[serde(default = "default_unhealthy_threshold")]
    pub unhealthy_threshold: u32,
    /// Healthy threshold (consecutive successes before marking healthy).
    #[serde(default = "default_healthy_threshold")]
    pub healthy_threshold: u32,
    /// Health check timeout in seconds.
    #[serde(default = "default_health_timeout_secs")]
    pub timeout_secs: u64,
    /// Dead node removal timeout in seconds.
    #[serde(default = "default_dead_node_timeout_secs")]
    pub dead_node_timeout_secs: u64,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: default_health_interval_secs(),
            unhealthy_threshold: default_unhealthy_threshold(),
            healthy_threshold: default_healthy_threshold(),
            timeout_secs: default_health_timeout_secs(),
            dead_node_timeout_secs: default_dead_node_timeout_secs(),
        }
    }
}

fn default_health_interval_secs() -> u64 {
    5
}

fn default_unhealthy_threshold() -> u32 {
    3
}

fn default_healthy_threshold() -> u32 {
    2
}

fn default_health_timeout_secs() -> u64 {
    3
}

fn default_dead_node_timeout_secs() -> u64 {
    60
}

/// Load balancer configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    /// Load balancing strategy.
    #[serde(default)]
    pub strategy: LoadBalanceStrategy,
    /// Enable sticky sessions.
    #[serde(default = "default_sticky_sessions")]
    pub sticky_sessions: bool,
    /// Session timeout in seconds.
    #[serde(default = "default_session_timeout_secs")]
    pub session_timeout_secs: u64,
    /// Enable consistent hashing.
    #[serde(default = "default_consistent_hashing")]
    pub consistent_hashing: bool,
    /// Number of virtual nodes per physical node (for consistent hashing).
    #[serde(default = "default_virtual_nodes")]
    pub virtual_nodes: usize,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalanceStrategy::default(),
            sticky_sessions: default_sticky_sessions(),
            session_timeout_secs: default_session_timeout_secs(),
            consistent_hashing: default_consistent_hashing(),
            virtual_nodes: default_virtual_nodes(),
        }
    }
}

/// Load balancing strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalanceStrategy {
    /// Round-robin distribution - default.
    #[default]
    /// Round-robin distribution.
    RoundRobin,
    /// Least connections.
    LeastConnections,
    /// Least response time.
    LeastResponseTime,
    /// Weighted round-robin.
    WeightedRoundRobin,
    /// Consistent hashing (default for sessions).
    ConsistentHash,
    /// Random selection.
    Random,
}

fn default_sticky_sessions() -> bool {
    true
}

fn default_session_timeout_secs() -> u64 {
    3600
}

fn default_consistent_hashing() -> bool {
    true
}

fn default_virtual_nodes() -> usize {
    150
}

/// Distributed memory configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Memory backend to use.
    #[serde(default)]
    pub backend: MemoryBackend,
    /// Redis URL (if using Redis).
    #[serde(default = "default_redis_url")]
    pub redis_url: String,
    /// Redis cluster nodes.
    #[serde(default)]
    pub redis_cluster: Vec<String>,
    /// Cache TTL in seconds.
    #[serde(default = "default_cache_ttl_secs")]
    pub cache_ttl_secs: u64,
    /// Max memory per node in MB.
    #[serde(default = "default_max_memory_mb")]
    pub max_memory_mb: usize,
    /// Enable replication.
    #[serde(default = "default_replication")]
    pub replication: bool,
    /// Replication factor.
    #[serde(default = "default_replication_factor")]
    pub replication_factor: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            backend: MemoryBackend::default(),
            redis_url: default_redis_url(),
            redis_cluster: Vec::new(),
            cache_ttl_secs: default_cache_ttl_secs(),
            max_memory_mb: default_max_memory_mb(),
            replication: default_replication(),
            replication_factor: default_replication_factor(),
        }
    }
}

/// Memory backend type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MemoryBackend {
    /// Use Redis for distributed memory - default.
    #[default]
    /// Use Redis for distributed memory.
    Redis,
    /// Use in-memory with gossip replication.
    Gossip,
    /// Use etcd for distributed state.
    Etcd,
}

fn default_redis_url() -> String {
    "redis://127.0.0.1:6379".to_string()
}

fn default_cache_ttl_secs() -> u64 {
    3600
}

fn default_max_memory_mb() -> usize {
    512
}

fn default_replication() -> bool {
    true
}

fn default_replication_factor() -> usize {
    2
}

/// Gossip protocol configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipConfig {
    /// Gossip interval in milliseconds.
    #[serde(default = "default_gossip_interval_ms")]
    pub interval_ms: u64,
    /// Fanout (number of nodes to gossip to).
    #[serde(default = "default_gossip_fanout")]
    pub fanout: usize,
    /// Suspicion threshold for failure detection.
    #[serde(default = "default_suspicion_threshold")]
    pub suspicion_threshold: u32,
    /// Gossip message size limit in bytes.
    #[serde(default = "default_gossip_message_limit")]
    pub message_limit: usize,
    /// Enable encryption for gossip messages.
    #[serde(default)]
    pub encrypt: bool,
    /// Encryption key (base64 encoded).
    pub encryption_key: Option<String>,
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            interval_ms: default_gossip_interval_ms(),
            fanout: default_gossip_fanout(),
            suspicion_threshold: default_suspicion_threshold(),
            message_limit: default_gossip_message_limit(),
            encrypt: false,
            encryption_key: None,
        }
    }
}

fn default_gossip_interval_ms() -> u64 {
    200
}

fn default_gossip_fanout() -> usize {
    3
}

fn default_suspicion_threshold() -> u32 {
    3
}

fn default_gossip_message_limit() -> usize {
    1024
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DistributedConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.listen_addr.port(), 50051);
        assert_eq!(config.api_addr.port(), 8080);
    }

    #[test]
    fn test_consensus_config_durations() {
        let config = ConsensusConfig::default();
        assert_eq!(config.election_timeout(), Duration::from_millis(1000));
        assert_eq!(config.heartbeat_interval(), Duration::from_millis(100));
    }

    #[test]
    fn test_load_balance_strategy_serde() {
        let strategies = vec![
            LoadBalanceStrategy::RoundRobin,
            LoadBalanceStrategy::LeastConnections,
            LoadBalanceStrategy::ConsistentHash,
        ];
        for strategy in strategies {
            let json = serde_json::to_string(&strategy).unwrap();
            let decoded: LoadBalanceStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(strategy, decoded);
        }
    }
}

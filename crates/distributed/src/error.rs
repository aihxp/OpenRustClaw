//! Error types for the distributed crate.

use thiserror::Error;

/// Errors that can occur in distributed operations.
#[derive(Debug, Error)]
pub enum DistributedError {
    #[error("Cluster error: {0}")]
    Cluster(String),

    #[error("Node error: {0}")]
    Node(String),

    #[error("Consensus error: {0}")]
    Consensus(String),

    #[error("Discovery error: {0}")]
    Discovery(String),

    #[error("Messaging error: {0}")]
    Messaging(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("Lock error: {lock_name}: {message}")]
    Lock { lock_name: String, message: String },

    #[error("Task error: {0}")]
    Task(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("Leader not available")]
    LeaderNotAvailable,

    #[error("Not leader: current leader is {0:?}")]
    NotLeader(Option<String>),

    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Split brain detected: conflicting leader {0}")]
    SplitBrain(String),

    #[error("Quorum not reached: {0} of {1} nodes available")]
    QuorumNotReached(usize, usize),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for distributed operations.
pub type Result<T> = std::result::Result<T, DistributedError>;

impl From<tonic::Status> for DistributedError {
    fn from(status: tonic::Status) -> Self {
        DistributedError::Rpc(format!("gRPC error: {} - {}", status.code(), status.message()))
    }
}

impl From<serde_json::Error> for DistributedError {
    fn from(err: serde_json::Error) -> Self {
        DistributedError::Serialization(err.to_string())
    }
}

#[cfg(feature = "redis")]
impl From<redis::RedisError> for DistributedError {
    fn from(err: redis::RedisError) -> Self {
        DistributedError::Memory(format!("Redis error: {}", err))
    }
}

#[cfg(feature = "etcd")]
impl From<etcd_client::Error> for DistributedError {
    fn from(err: etcd_client::Error) -> Self {
        DistributedError::Discovery(format!("etcd error: {}", err))
    }
}

impl From<std::io::Error> for DistributedError {
    fn from(err: std::io::Error) -> Self {
        DistributedError::Network(err.to_string())
    }
}

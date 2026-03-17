//! Raft consensus implementation for leader election and log replication.

use crate::cluster::{Cluster, ClusterEvent};
use crate::config::ConsensusConfig;
use crate::error::{DistributedError, Result};
use crate::node::{LocalNode, NodeId, NodeInfo, NodeRole};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::{RwLock, oneshot};
use tokio::time::{Instant, interval};
use tracing::{debug, error, info, warn};

/// Raft role for a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaftRole {
    /// Follower accepts log entries from leader.
    Follower,
    /// Candidate is requesting votes for election.
    Candidate,
    /// Leader manages the cluster.
    Leader,
}

impl std::fmt::Display for RaftRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RaftRole::Follower => write!(f, "follower"),
            RaftRole::Candidate => write!(f, "candidate"),
            RaftRole::Leader => write!(f, "leader"),
        }
    }
}

/// A log entry in the Raft log.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub index: u64,
    pub term: u64,
    pub data: Vec<u8>,
    pub entry_type: EntryType,
}

/// Type of log entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryType {
    /// Normal command entry.
    Command,
    /// Cluster membership change.
    ConfigChange,
    /// No-op entry (used after election).
    NoOp,
}

/// Raft state machine.
pub struct RaftNode {
    /// Local node reference.
    local_node: Arc<LocalNode>,
    /// Cluster reference.
    cluster: Arc<Cluster>,
    /// Configuration.
    config: ConsensusConfig,

    // Persistent state
    /// Current term.
    current_term: AtomicU64,
    /// Candidate that received vote in current term.
    voted_for: RwLock<Option<NodeId>>,
    /// Log entries.
    log: RwLock<Vec<LogEntry>>,

    // Volatile state
    /// Current Raft role.
    role: RwLock<RaftRole>,
    /// Commit index (highest log entry known to be committed).
    commit_index: AtomicU64,
    /// Last applied index (highest log entry applied to state machine).
    #[allow(dead_code)]
    last_applied: AtomicU64,

    // Leader state (reinitialized after election)
    /// For each server, index of next log entry to send.
    next_index: RwLock<HashMap<NodeId, u64>>,
    /// For each server, index of highest log entry known to be replicated.
    match_index: RwLock<HashMap<NodeId, u64>>,

    // Control
    /// Running flag.
    running: AtomicBool,
    /// Channel to signal leader step down.
    step_down_tx: RwLock<Option<oneshot::Sender<()>>>,
    /// Last heartbeat received time.
    last_heartbeat: RwLock<Instant>,
}

impl RaftNode {
    /// Create a new Raft node.
    pub fn new(
        local_node: Arc<LocalNode>,
        cluster: Arc<Cluster>,
        config: ConsensusConfig,
    ) -> Arc<Self> {
        Arc::new(Self {
            local_node,
            cluster,
            config,
            current_term: AtomicU64::new(0),
            voted_for: RwLock::new(None),
            log: RwLock::new(Vec::new()),
            role: RwLock::new(RaftRole::Follower),
            commit_index: AtomicU64::new(0),
            last_applied: AtomicU64::new(0),
            next_index: RwLock::new(HashMap::new()),
            match_index: RwLock::new(HashMap::new()),
            running: AtomicBool::new(false),
            step_down_tx: RwLock::new(None),
            last_heartbeat: RwLock::new(Instant::now()),
        })
    }

    /// Start the Raft node.
    pub async fn start(self: Arc<Self>) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!("Starting Raft consensus node");
        self.running.store(true, Ordering::SeqCst);

        // Start the main event loop
        tokio::spawn(self.clone().run_event_loop());

        Ok(())
    }

    /// Stop the Raft node.
    pub async fn stop(&self) {
        info!("Stopping Raft consensus node");
        self.running.store(false, Ordering::SeqCst);

        // Signal leader to step down if we are leader
        if let Some(tx) = self.step_down_tx.write().await.take() {
            let _ = tx.send(());
        }
    }

    /// Get current role.
    pub async fn role(&self) -> RaftRole {
        *self.role.read().await
    }

    /// Get current term.
    pub fn current_term(&self) -> u64 {
        self.current_term.load(Ordering::SeqCst)
    }

    /// Check if we are the leader.
    pub async fn is_leader(&self) -> bool {
        matches!(self.role().await, RaftRole::Leader)
    }

    /// Main event loop for Raft state machine.
    async fn run_event_loop(self: Arc<Self>) {
        let election_timeout = self.config.election_timeout();
        let mut election_timer = interval(election_timeout);

        while self.running.load(Ordering::SeqCst) {
            election_timer.tick().await;

            let role = self.role().await;

            match role {
                RaftRole::Follower | RaftRole::Candidate => {
                    // Check if election timeout has passed
                    let last_hb = *self.last_heartbeat.read().await;
                    let elapsed = last_hb.elapsed();

                    if elapsed >= election_timeout {
                        info!(
                            "Election timeout elapsed ({}ms), starting election",
                            elapsed.as_millis()
                        );
                        if let Err(e) = self.start_election().await {
                            error!("Election failed: {}", e);
                        }
                    }
                }
                RaftRole::Leader => {
                    // Leader sends heartbeats
                    self.send_heartbeats().await;
                }
            }
        }
    }

    /// Start a leader election.
    async fn start_election(&self) -> Result<()> {
        let mut role = self.role.write().await;
        *role = RaftRole::Candidate;
        drop(role);

        // Increment term
        let term = self.current_term.fetch_add(1, Ordering::SeqCst) + 1;
        self.local_node.set_term(term);

        // Vote for self
        *self.voted_for.write().await = Some(self.local_node.id());
        let mut votes_received = 1;
        let votes_needed = self.cluster.quorum_size();

        info!(
            "Starting election for term {} (need {} votes)",
            term, votes_needed
        );

        // Request votes from all other nodes
        let nodes = self.cluster.healthy_nodes();
        let local_id = self.local_node.id();

        let last_log_index = self.last_log_index();
        let last_log_term = self.last_log_term();

        for node in nodes {
            if node.id == local_id {
                continue;
            }

            let request = VoteRequest {
                term,
                candidate_id: local_id.clone(),
                last_log_index,
                last_log_term,
            };

            // Send vote request (will be implemented with gRPC)
            match self.request_vote(&node, request).await {
                Ok(response) => {
                    if response.vote_granted {
                        votes_received += 1;
                        info!("Received vote from {} (total: {})", node.id, votes_received);
                    } else if response.term > term {
                        // Higher term discovered, step down
                        warn!("Higher term {} discovered, stepping down", response.term);
                        self.step_down(response.term).await;
                        return Ok(());
                    }
                }
                Err(e) => {
                    warn!("Failed to request vote from {}: {}", node.id, e);
                }
            }
        }

        // Check if we won
        if votes_received >= votes_needed {
            info!(
                "Won election with {} votes, becoming leader",
                votes_received
            );
            self.become_leader().await?;
        } else {
            debug!(
                "Election lost: {} votes received, {} needed",
                votes_received, votes_needed
            );
        }

        Ok(())
    }

    /// Become the leader.
    async fn become_leader(&self) -> Result<()> {
        let mut role = self.role.write().await;
        *role = RaftRole::Leader;
        drop(role);

        // Update cluster state
        self.cluster.set_leader(Some(self.local_node.id())).await;
        self.local_node.set_role(NodeRole::Leader).await;

        // Initialize leader state
        let mut next_index = self.next_index.write().await;
        let mut match_index = self.match_index.write().await;

        let last_log = self.last_log_index() + 1;
        for node in self.cluster.healthy_nodes() {
            if node.id != self.local_node.id() {
                next_index.insert(node.id.clone(), last_log);
                match_index.insert(node.id.clone(), 0);
            }
        }

        // Create step down channel
        let (tx, _rx) = oneshot::channel();
        *self.step_down_tx.write().await = Some(tx);

        // Append no-op entry
        self.append_entry(EntryType::NoOp, vec![]).await?;

        info!("Became leader for term {}", self.current_term());

        // Notify cluster
        let _ = self
            .cluster
            .event_sender()
            .send(ClusterEvent::LeaderChanged {
                old_leader: None,
                new_leader: Some(self.local_node.id()),
            });

        Ok(())
    }

    /// Step down to follower.
    async fn step_down(&self, new_term: u64) {
        warn!("Stepping down to follower for term {}", new_term);

        let mut role = self.role.write().await;
        *role = RaftRole::Follower;
        drop(role);

        self.current_term.store(new_term, Ordering::SeqCst);
        self.local_node.set_term(new_term);
        self.local_node.set_role(NodeRole::Worker).await;

        // Clear leader state
        self.next_index.write().await.clear();
        self.match_index.write().await.clear();

        // Signal any ongoing leader operations to stop
        if let Some(tx) = self.step_down_tx.write().await.take() {
            let _ = tx.send(());
        }
    }

    /// Handle a vote request from another node.
    pub async fn handle_vote_request(&self, request: VoteRequest) -> VoteResponse {
        let current_term = self.current_term();

        // Reply false if term < currentTerm
        if request.term < current_term {
            return VoteResponse {
                term: current_term,
                vote_granted: false,
                voter_id: self.local_node.id(),
            };
        }

        // If term > currentTerm, update currentTerm and become follower
        if request.term > current_term {
            self.step_down(request.term).await;
        }

        let voted_for = self.voted_for.read().await.clone();
        let last_log_index = self.last_log_index();
        let last_log_term = self.last_log_term();

        // Check if candidate's log is at least as up-to-date
        let log_ok = request.last_log_term > last_log_term
            || (request.last_log_term == last_log_term && request.last_log_index >= last_log_index);

        // Grant vote if we haven't voted or voted for this candidate, and log is ok
        let vote_granted =
            log_ok && (voted_for.is_none() || voted_for == Some(request.candidate_id.clone()));

        if vote_granted {
            *self.voted_for.write().await = Some(request.candidate_id.clone());
            // Reset heartbeat timer
            *self.last_heartbeat.write().await = Instant::now();
            info!(
                "Granted vote to {} for term {}",
                request.candidate_id, request.term
            );
        }

        VoteResponse {
            term: self.current_term(),
            vote_granted,
            voter_id: self.local_node.id(),
        }
    }

    /// Handle append entries (heartbeat or log replication) from leader.
    pub async fn handle_append_entries(
        &self,
        request: AppendEntriesRequest,
    ) -> AppendEntriesResponse {
        let current_term = self.current_term();

        // Reply false if term < currentTerm
        if request.term < current_term {
            return AppendEntriesResponse {
                term: current_term,
                success: false,
                match_index: 0,
                conflict_index: 0,
                conflict_term: 0,
            };
        }

        // Reset heartbeat timer
        *self.last_heartbeat.write().await = Instant::now();

        // If term > currentTerm, become follower
        if request.term > current_term {
            self.step_down(request.term).await;
        } else if matches!(self.role().await, RaftRole::Candidate) {
            // Step down from candidate to follower
            let mut role = self.role.write().await;
            *role = RaftRole::Follower;
        }

        // Update cluster leader if known
        if self.cluster.leader_id().await != Some(request.leader_id.clone()) {
            self.cluster
                .set_leader(Some(request.leader_id.clone()))
                .await;
        }

        // Reply false if log doesn't contain an entry at prevLogIndex with prevLogTerm
        if request.prev_log_index > 0 {
            let log = self.log.read().await;
            if let Some(entry) = log.get((request.prev_log_index - 1) as usize) {
                if entry.term != request.prev_log_term {
                    return AppendEntriesResponse {
                        term: self.current_term(),
                        success: false,
                        match_index: 0,
                        conflict_index: request.prev_log_index,
                        conflict_term: entry.term,
                    };
                }
            } else {
                return AppendEntriesResponse {
                    term: self.current_term(),
                    success: false,
                    match_index: 0,
                    conflict_index: request.prev_log_index,
                    conflict_term: 0,
                };
            }
        }

        // TODO: Append any new entries not already in the log
        // TODO: Update commit index

        AppendEntriesResponse {
            term: self.current_term(),
            success: true,
            match_index: request.prev_log_index + request.entries.len() as u64,
            conflict_index: 0,
            conflict_term: 0,
        }
    }

    /// Send heartbeats to all followers.
    async fn send_heartbeats(&self) {
        let nodes = self.cluster.healthy_nodes();
        let local_id = self.local_node.id();
        let term = self.current_term();
        let last_log_index = self.last_log_index();
        let commit_index = self.commit_index();

        for node in nodes {
            if node.id == local_id {
                continue;
            }

            let next_idx = self
                .next_index
                .read()
                .await
                .get(&node.id)
                .copied()
                .unwrap_or(last_log_index + 1);

            let prev_log_index = next_idx - 1;
            let prev_log_term = if prev_log_index > 0 {
                self.log_term_at(prev_log_index).await.unwrap_or(0)
            } else {
                0
            };

            let request = AppendEntriesRequest {
                term,
                leader_id: local_id.clone(),
                prev_log_index,
                prev_log_term,
                entries: vec![], // Empty for heartbeat
                leader_commit: commit_index,
            };

            // Send heartbeat (will be implemented with gRPC)
            match self.send_append_entries(&node, request).await {
                Ok(response) => {
                    if !response.success {
                        // Decrement next_index and retry
                        if let Some(idx) = self.next_index.write().await.get_mut(&node.id) {
                            *idx = (*idx - 1).max(1);
                        }
                    } else {
                        // Update match_index
                        self.match_index
                            .write()
                            .await
                            .insert(node.id.clone(), response.match_index);
                        self.next_index
                            .write()
                            .await
                            .insert(node.id.clone(), response.match_index + 1);
                    }

                    if response.term > term {
                        self.step_down(response.term).await;
                        return;
                    }
                }
                Err(e) => {
                    debug!("Failed to send heartbeat to {}: {}", node.id, e);
                }
            }
        }
    }

    /// Append a new entry to the log (leader only).
    pub async fn append_entry(&self, entry_type: EntryType, data: Vec<u8>) -> Result<LogEntry> {
        if !self.is_leader().await {
            return Err(DistributedError::NotLeader(self.cluster.leader_id().await));
        }

        let index = self.last_log_index() + 1;
        let term = self.current_term();

        let entry = LogEntry {
            index,
            term,
            data,
            entry_type,
        };

        self.log.write().await.push(entry.clone());

        info!(
            "Appended entry at index {} (term {}), type {:?}",
            index, term, entry_type
        );

        Ok(entry)
    }

    /// Get the last log index.
    pub fn last_log_index(&self) -> u64 {
        self.log.blocking_read().len() as u64
    }

    /// Get the term of the last log entry.
    pub fn last_log_term(&self) -> u64 {
        self.log.blocking_read().last().map(|e| e.term).unwrap_or(0)
    }

    /// Get term at specific log index.
    async fn log_term_at(&self, index: u64) -> Option<u64> {
        self.log
            .read()
            .await
            .get((index - 1) as usize)
            .map(|e| e.term)
    }

    /// Get commit index.
    pub fn commit_index(&self) -> u64 {
        self.commit_index.load(Ordering::SeqCst)
    }

    // Placeholder for gRPC vote request
    async fn request_vote(&self, node: &NodeInfo, request: VoteRequest) -> Result<VoteResponse> {
        // This will be implemented with actual gRPC client
        // For now, return a default response
        Ok(VoteResponse {
            term: request.term,
            vote_granted: true,
            voter_id: node.id.clone(),
        })
    }

    // Placeholder for gRPC append entries
    async fn send_append_entries(
        &self,
        _node: &NodeInfo,
        request: AppendEntriesRequest,
    ) -> Result<AppendEntriesResponse> {
        // This will be implemented with actual gRPC client
        Ok(AppendEntriesResponse {
            term: request.term,
            success: true,
            match_index: request.prev_log_index,
            conflict_index: 0,
            conflict_term: 0,
        })
    }
}

/// Vote request message.
#[derive(Debug, Clone)]
pub struct VoteRequest {
    pub term: u64,
    pub candidate_id: NodeId,
    pub last_log_index: u64,
    pub last_log_term: u64,
}

/// Vote response message.
#[derive(Debug, Clone)]
pub struct VoteResponse {
    pub term: u64,
    pub vote_granted: bool,
    pub voter_id: NodeId,
}

/// Append entries request message.
#[derive(Debug, Clone)]
pub struct AppendEntriesRequest {
    pub term: u64,
    pub leader_id: NodeId,
    pub prev_log_index: u64,
    pub prev_log_term: u64,
    pub entries: Vec<LogEntry>,
    pub leader_commit: u64,
}

/// Append entries response message.
#[derive(Debug, Clone)]
pub struct AppendEntriesResponse {
    pub term: u64,
    pub success: bool,
    pub match_index: u64,
    pub conflict_index: u64,
    pub conflict_term: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cluster::Cluster;
    use crate::config::DistributedConfig;
    use std::net::SocketAddr;

    fn create_test_node(id: &str) -> (Arc<LocalNode>, Arc<Cluster>) {
        let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
        let info = NodeInfo::new(id, format!("node-{}", id), addr, addr, NodeRole::Worker);
        let local = Arc::new(LocalNode::new(info));
        let config = DistributedConfig::default();
        let cluster = Cluster::new(local.clone(), config);
        (local, cluster)
    }

    #[tokio::test]
    async fn test_raft_initial_state() {
        let (local, cluster) = create_test_node("test-1");
        let config = ConsensusConfig::default();
        let raft = RaftNode::new(local, cluster, config);

        assert!(matches!(raft.role().await, RaftRole::Follower));
        assert_eq!(raft.current_term(), 0);
        assert!(!raft.is_leader().await);
    }

    #[tokio::test]
    async fn test_vote_request_handling() {
        let (local, cluster) = create_test_node("test-1");
        let config = ConsensusConfig::default();
        let raft = RaftNode::new(local, cluster, config);

        let request = VoteRequest {
            term: 1,
            candidate_id: "test-2".to_string(),
            last_log_index: 0,
            last_log_term: 0,
        };

        let response = raft.handle_vote_request(request).await;
        assert!(response.vote_granted);
        assert_eq!(response.term, 1);
    }

    #[tokio::test]
    async fn test_handle_higher_term() {
        let (local, cluster) = create_test_node("test-1");
        let config = ConsensusConfig::default();
        let raft = RaftNode::new(local, cluster, config);

        // Become candidate
        raft.current_term.store(2, Ordering::SeqCst);
        *raft.role.write().await = RaftRole::Candidate;

        // Receive append entries with higher term
        let request = AppendEntriesRequest {
            term: 3,
            leader_id: "test-2".to_string(),
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0,
        };

        let response = raft.handle_append_entries(request).await;
        assert!(response.success);
        assert_eq!(raft.current_term(), 3);
        assert!(matches!(raft.role().await, RaftRole::Follower));
    }
}

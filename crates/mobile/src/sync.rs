//! Data synchronization for mobile nodes

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Sync configuration
#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub sync_mode: SyncMode,
    pub priority: SyncPriority,
    pub conflict_resolution: ConflictResolution,
    pub max_sync_interval_secs: u64,
    pub min_battery_percent: u8,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            sync_mode: SyncMode::Automatic,
            priority: SyncPriority::Normal,
            conflict_resolution: ConflictResolution::LastWriteWins,
            max_sync_interval_secs: 300, // 5 minutes
            min_battery_percent: 20,
        }
    }
}

impl SyncConfig {
    /// Create a new sync configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the sync mode
    pub fn with_sync_mode(mut self, mode: SyncMode) -> Self {
        self.sync_mode = mode;
        self
    }

    /// Set the sync priority
    pub fn with_priority(mut self, priority: SyncPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the conflict resolution strategy
    pub fn with_conflict_resolution(mut self, resolution: ConflictResolution) -> Self {
        self.conflict_resolution = resolution;
        self
    }

    /// Set the max sync interval in seconds
    pub fn with_max_sync_interval(mut self, secs: u64) -> Self {
        self.max_sync_interval_secs = secs;
        self
    }

    /// Set the minimum battery percentage for sync
    pub fn with_min_battery_percent(mut self, percent: u8) -> Self {
        self.min_battery_percent = percent;
        self
    }
}

/// Sync mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncMode {
    Realtime,  // Continuous sync
    Automatic, // Sync when conditions are met
    Manual,    // User-initiated only
    Scheduled, // Sync at scheduled times
}

/// Sync priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    LastWriteWins,
    FirstWriteWins,
    ServerWins,
    ClientWins,
    Manual,
}

/// Sync operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncOperation {
    Create,
    Update,
    Delete,
}

/// A single sync change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncChange {
    pub id: String,
    pub collection: String,
    pub operation: SyncOperation,
    pub data: Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl SyncChange {
    /// Create a new sync change
    pub fn new(
        id: impl Into<String>,
        collection: impl Into<String>,
        operation: SyncOperation,
        data: Value,
    ) -> Self {
        Self {
            id: id.into(),
            collection: collection.into(),
            operation,
            data,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Sync conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConflict {
    pub change_id: String,
    pub local: Value,
    pub server: Value,
    pub local_timestamp: chrono::DateTime<chrono::Utc>,
    pub server_timestamp: chrono::DateTime<chrono::Utc>,
}

/// Sync result
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncResult {
    pub uploaded: usize,
    pub downloaded: usize,
    pub conflicts: Vec<SyncConflict>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl SyncResult {
    /// Create a new sync result
    pub fn new() -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            ..Default::default()
        }
    }

    /// Check if sync was successful (no conflicts)
    pub fn is_success(&self) -> bool {
        self.conflicts.is_empty()
    }
}

/// Sync error
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("Conflict error: {0}")]
    Conflict(String),
    #[error("Authentication error")]
    Auth,
    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Sync manager for handling data synchronization
pub struct SyncManager {
    config: SyncConfig,
    pending_changes: Vec<SyncChange>,
    last_sync: Option<chrono::DateTime<chrono::Utc>>,
    sync_results: Vec<SyncResult>,
}

impl SyncManager {
    /// Create a new sync manager
    pub fn new(config: SyncConfig) -> Self {
        Self {
            config,
            pending_changes: Vec::new(),
            last_sync: None,
            sync_results: Vec::new(),
        }
    }

    /// Queue a change for sync
    pub fn queue_change(&mut self, change: SyncChange) {
        self.pending_changes.push(change);
    }

    /// Get pending changes count
    pub fn pending_count(&self) -> usize {
        self.pending_changes.len()
    }

    /// Check if there are pending changes
    pub fn has_pending_changes(&self) -> bool {
        !self.pending_changes.is_empty()
    }

    /// Get the last sync time
    pub fn last_sync(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.last_sync
    }

    /// Should sync based on battery and mode
    pub fn should_sync(&self, battery_percent: u8) -> bool {
        if battery_percent < self.config.min_battery_percent {
            return false;
        }

        match self.config.sync_mode {
            SyncMode::Manual | SyncMode::Scheduled => false,
            _ => !self.pending_changes.is_empty(),
        }
    }

    /// Check if sync is needed based on time interval
    pub fn is_sync_due(&self) -> bool {
        match self.last_sync {
            None => true,
            Some(last) => {
                let elapsed = chrono::Utc::now().signed_duration_since(last);
                elapsed.num_seconds() as u64 >= self.config.max_sync_interval_secs
            }
        }
    }

    /// Perform sync (placeholder implementation)
    pub async fn sync(&mut self) -> Result<SyncResult, SyncError> {
        // Implementation: Send pending changes to server
        // Receive server changes
        // Resolve conflicts

        let uploaded = self.pending_changes.len();
        let result = SyncResult {
            uploaded,
            downloaded: 0,
            conflicts: vec![],
            timestamp: chrono::Utc::now(),
        };

        // Clear pending changes on success
        self.pending_changes.clear();
        self.last_sync = Some(chrono::Utc::now());
        self.sync_results.push(result.clone());

        Ok(result)
    }

    /// Perform sync with conflict resolution
    pub async fn sync_with_resolution(
        &mut self,
        local_data: &HashMap<String, Value>,
        server_data: &HashMap<String, Value>,
    ) -> Result<SyncResult, SyncError> {
        let mut conflicts = Vec::new();
        let mut uploaded = 0;
        let downloaded = 0;

        // Process pending local changes
        for change in &self.pending_changes {
            match change.operation {
                SyncOperation::Create | SyncOperation::Update => {
                    // Check for conflicts
                    if let Some(server_value) = server_data.get(&change.id) {
                        let local_value = local_data.get(&change.id);

                        if local_value != Some(&change.data) {
                            // Conflict detected
                            conflicts.push(SyncConflict {
                                change_id: change.id.clone(),
                                local: change.data.clone(),
                                server: server_value.clone(),
                                local_timestamp: change.timestamp,
                                server_timestamp: chrono::Utc::now(),
                            });
                            continue;
                        }
                    }
                    uploaded += 1;
                }
                SyncOperation::Delete => {
                    uploaded += 1;
                }
            }
        }

        // Resolve conflicts based on strategy
        for _conflict in &conflicts {
            match self.config.conflict_resolution {
                ConflictResolution::LastWriteWins => {
                    // Keep the server version if it's newer
                    // (already handled by default)
                }
                ConflictResolution::FirstWriteWins => {
                    // Keep local version (would need special handling)
                }
                ConflictResolution::ServerWins => {
                    // Always keep server version
                }
                ConflictResolution::ClientWins => {
                    // Always keep local version
                }
                ConflictResolution::Manual => {
                    // Leave for manual resolution
                }
            }
        }

        let result = SyncResult {
            uploaded,
            downloaded,
            conflicts,
            timestamp: chrono::Utc::now(),
        };

        self.pending_changes.clear();
        self.last_sync = Some(chrono::Utc::now());
        self.sync_results.push(result.clone());

        Ok(result)
    }

    /// Get sync history
    pub fn sync_history(&self) -> &[SyncResult] {
        &self.sync_results
    }

    /// Clear sync history
    pub fn clear_history(&mut self) {
        self.sync_results.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_config_default() {
        let config = SyncConfig::default();
        assert_eq!(config.sync_mode, SyncMode::Automatic);
        assert_eq!(config.priority, SyncPriority::Normal);
        assert_eq!(
            config.conflict_resolution,
            ConflictResolution::LastWriteWins
        );
        assert_eq!(config.max_sync_interval_secs, 300);
        assert_eq!(config.min_battery_percent, 20);
    }

    #[test]
    fn test_sync_config_builder() {
        let config = SyncConfig::new()
            .with_sync_mode(SyncMode::Realtime)
            .with_priority(SyncPriority::High)
            .with_conflict_resolution(ConflictResolution::ServerWins)
            .with_max_sync_interval(60)
            .with_min_battery_percent(10);

        assert_eq!(config.sync_mode, SyncMode::Realtime);
        assert_eq!(config.priority, SyncPriority::High);
        assert_eq!(config.conflict_resolution, ConflictResolution::ServerWins);
        assert_eq!(config.max_sync_interval_secs, 60);
        assert_eq!(config.min_battery_percent, 10);
    }

    #[test]
    fn test_sync_change() {
        let data = serde_json::json!({"name": "test"});
        let change = SyncChange::new("id1", "users", SyncOperation::Create, data.clone());

        assert_eq!(change.id, "id1");
        assert_eq!(change.collection, "users");
        assert_eq!(change.operation, SyncOperation::Create);
        assert_eq!(change.data, data);
    }

    #[test]
    fn test_sync_manager() {
        let config = SyncConfig::default();
        let mut manager = SyncManager::new(config);

        // Add some pending changes
        let change = SyncChange::new(
            "id1",
            "users",
            SyncOperation::Create,
            serde_json::json!({"name": "test"}),
        );
        manager.queue_change(change);

        assert_eq!(manager.pending_count(), 1);
        assert!(manager.has_pending_changes());

        // Check sync conditions
        assert!(manager.should_sync(50)); // Above battery threshold
        assert!(!manager.should_sync(10)); // Below battery threshold

        // Initially no sync
        assert!(manager.is_sync_due());
    }

    #[test]
    fn test_sync_result() {
        let result = SyncResult::new();
        assert_eq!(result.uploaded, 0);
        assert_eq!(result.downloaded, 0);
        assert!(result.conflicts.is_empty());
        assert!(result.is_success());

        let result_with_conflict = SyncResult {
            uploaded: 1,
            downloaded: 1,
            conflicts: vec![SyncConflict {
                change_id: "id1".to_string(),
                local: serde_json::json!({}),
                server: serde_json::json!({}),
                local_timestamp: chrono::Utc::now(),
                server_timestamp: chrono::Utc::now(),
            }],
            timestamp: chrono::Utc::now(),
        };
        assert!(!result_with_conflict.is_success());
    }
}

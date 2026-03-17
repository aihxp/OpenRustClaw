//! Session manager.

use openrustclaw_core::error::{Error, GatewayError, Result};
use openrustclaw_core::types::{Platform, Session, SessionType};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::info;

/// Manages active sessions.
pub struct SessionManager {
    sessions: RwLock<HashMap<String, Session>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new session.
    pub async fn create_session(
        &self,
        user_id: &str,
        session_type: SessionType,
        platform: Platform,
    ) -> Result<Session> {
        let session = Session::new(session_type, user_id, platform);
        let id = session.id.to_string();
        self.sessions
            .write()
            .await
            .insert(id.clone(), session.clone());
        info!(session_id = %id, user_id = %user_id, "Session created");
        Ok(session)
    }

    /// Get a session by ID.
    pub async fn get_session(&self, id: &str) -> Result<Session> {
        self.sessions
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| Error::Gateway(GatewayError::SessionNotFound(id.to_string())))
    }

    /// Remove a session.
    pub async fn remove_session(&self, id: &str) -> Result<()> {
        self.sessions.write().await.remove(id);
        info!(session_id = %id, "Session removed");
        Ok(())
    }

    /// Get active session count.
    pub async fn count(&self) -> usize {
        self.sessions.read().await.len()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

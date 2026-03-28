//! Session manager.

use openrustclaw_core::error::{Error, GatewayError, Result};
use openrustclaw_core::types::{Message, Platform, Session, SessionType};
use openrustclaw_db::{PersistedSession, SessionStatus, SqliteSessionStore};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Manages active sessions.
pub struct SessionManager {
    sessions: RwLock<HashMap<String, Session>>,
    store: Option<Arc<SqliteSessionStore>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            store: None,
        }
    }

    pub fn with_store(store: Arc<SqliteSessionStore>) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            store: Some(store),
        }
    }

    /// Create a new session.
    pub async fn create_session(
        &self,
        user_id: &str,
        session_type: SessionType,
        platform: Platform,
    ) -> Result<Session> {
        self.create_session_with_route(user_id, session_type, platform, None)
            .await
    }

    pub async fn create_session_with_route(
        &self,
        user_id: &str,
        session_type: SessionType,
        platform: Platform,
        route_key: Option<&str>,
    ) -> Result<Session> {
        let mut session = Session::new(session_type, user_id, platform);
        if let Some(route_key) = route_key {
            session.metadata["route_key"] = serde_json::Value::String(route_key.to_string());
        }
        apply_assistant_session_metadata(&mut session, platform);
        self.persist_new_session(session, route_key).await
    }

    pub async fn create_session_with_context(
        &self,
        user_id: &str,
        session_type: SessionType,
        platform: Platform,
        route_key: Option<&str>,
        workspace_id: Option<&str>,
        metadata: Option<serde_json::Value>,
    ) -> Result<Session> {
        let mut session = Session::new(session_type, user_id, platform);
        session.workspace_id = workspace_id.map(|value| value.to_string());
        if let Some(route_key) = route_key {
            session.metadata["route_key"] = serde_json::Value::String(route_key.to_string());
        }
        if let Some(metadata) = metadata {
            session.metadata = merge_json(session.metadata, metadata);
        }
        apply_assistant_session_metadata(&mut session, platform);
        self.persist_new_session(session, route_key).await
    }

    async fn persist_new_session(
        &self,
        session: Session,
        route_key: Option<&str>,
    ) -> Result<Session> {
        let id = session.id.to_string();
        self.sessions
            .write()
            .await
            .insert(id.clone(), session.clone());
        if let Some(store) = &self.store {
            store
                .create_or_update(&session, route_key, SessionStatus::Active)
                .await
                .map_err(|error| Error::Gateway(GatewayError::WebSocket(error.to_string())))?;
        }
        info!(session_id = %id, user_id = %session.user_id, "Session created");
        Ok(session)
    }

    pub async fn restore_or_create_session(
        &self,
        user_id: &str,
        session_type: SessionType,
        platform: Platform,
        route_key: Option<&str>,
    ) -> Result<Session> {
        if let (Some(store), Some(route_key)) = (&self.store, route_key)
            && let Some(restored) = store
                .find_active_by_route_key(route_key)
                .await
                .map_err(|error| Error::Gateway(GatewayError::WebSocket(error.to_string())))?
        {
            let session = restored.session;
            self.sessions
                .write()
                .await
                .insert(session.id.to_string(), session.clone());
            return Ok(session);
        }

        self.create_session_with_route(user_id, session_type, platform, route_key)
            .await
    }

    pub async fn restore_or_create_session_with_context(
        &self,
        user_id: &str,
        session_type: SessionType,
        platform: Platform,
        route_key: Option<&str>,
        workspace_id: Option<&str>,
        metadata: Option<serde_json::Value>,
    ) -> Result<Session> {
        if let (Some(store), Some(route_key)) = (&self.store, route_key)
            && let Some(restored) = store
                .find_active_by_route_key(route_key)
                .await
                .map_err(|error| Error::Gateway(GatewayError::WebSocket(error.to_string())))?
        {
            let mut session = restored.session;
            if let Some(workspace_id) = workspace_id {
                session.workspace_id = Some(workspace_id.to_string());
            }
            session.metadata["route_key"] = serde_json::Value::String(route_key.to_string());
            if let Some(metadata) = metadata.clone() {
                session.metadata = merge_json(session.metadata, metadata);
            }
            apply_assistant_session_metadata(&mut session, platform);
            store
                .create_or_update(&session, Some(route_key), SessionStatus::Active)
                .await
                .map_err(|error| Error::Gateway(GatewayError::WebSocket(error.to_string())))?;
            self.sessions
                .write()
                .await
                .insert(session.id.to_string(), session.clone());
            return Ok(session);
        }

        self.create_session_with_context(
            user_id,
            session_type,
            platform,
            route_key,
            workspace_id,
            metadata,
        )
        .await
    }

    /// Get a session by ID.
    pub async fn get_session(&self, id: &str) -> Result<Session> {
        if let Some(session) = self.sessions.read().await.get(id).cloned() {
            return Ok(session);
        }

        if let Some(store) = &self.store
            && let Some(persisted) = store
                .get_session(id)
                .await
                .map_err(|error| Error::Gateway(GatewayError::WebSocket(error.to_string())))?
        {
            let session = persisted.session;
            self.sessions
                .write()
                .await
                .insert(session.id.to_string(), session.clone());
            return Ok(session);
        }

        Err(Error::Gateway(GatewayError::SessionNotFound(
            id.to_string(),
        )))
    }

    pub async fn list_sessions(
        &self,
        status: Option<SessionStatus>,
        limit: usize,
    ) -> Result<Vec<PersistedSession>> {
        let Some(store) = &self.store else {
            let sessions = self
                .sessions
                .read()
                .await
                .values()
                .take(limit)
                .cloned()
                .map(|session| PersistedSession {
                    session,
                    status: SessionStatus::Active,
                    route_key: None,
                    archived_at: None,
                    closed_at: None,
                })
                .collect();
            return Ok(sessions);
        };

        store
            .list_sessions(status, limit)
            .await
            .map_err(|error| Error::Gateway(GatewayError::WebSocket(error.to_string())))
    }

    pub async fn append_message(&self, session_id: &str, message: &Message) -> Result<()> {
        if let Some(store) = &self.store {
            store
                .append_message(session_id, message)
                .await
                .map_err(|error| Error::Gateway(GatewayError::WebSocket(error.to_string())))?;
        }
        Ok(())
    }

    pub async fn list_history(&self, session_id: &str, limit: usize) -> Result<Vec<Message>> {
        let Some(store) = &self.store else {
            return Ok(Vec::new());
        };
        store
            .list_history(session_id, limit)
            .await
            .map_err(|error| Error::Gateway(GatewayError::WebSocket(error.to_string())))
    }

    pub async fn archive_session(&self, id: &str, reason: Option<&str>) -> Result<()> {
        if let Some(store) = &self.store {
            store
                .set_status(id, SessionStatus::Archived, reason)
                .await
                .map_err(|error| Error::Gateway(GatewayError::WebSocket(error.to_string())))?;
        }
        self.sessions.write().await.remove(id);
        Ok(())
    }

    /// Remove a session.
    pub async fn remove_session(&self, id: &str) -> Result<()> {
        self.sessions.write().await.remove(id);
        if let Some(store) = &self.store {
            store
                .set_status(id, SessionStatus::Closed, Some("removed_from_runtime"))
                .await
                .map_err(|error| Error::Gateway(GatewayError::WebSocket(error.to_string())))?;
        }
        info!(session_id = %id, "Session removed");
        Ok(())
    }

    /// Get active session count.
    pub async fn count(&self) -> usize {
        self.sessions.read().await.len()
    }
}

fn apply_assistant_session_metadata(session: &mut Session, platform: Platform) {
    let Some(surface) = assistant_surface_for_platform(platform) else {
        return;
    };
    session.metadata["assistant_identity"] = serde_json::Value::String("primary".to_string());
    session.metadata["assistant_surface"] = serde_json::Value::String(surface.to_string());
    session.metadata["assistant_session_model"] =
        serde_json::Value::String("persisted".to_string());
}

fn assistant_surface_for_platform(platform: Platform) -> Option<&'static str> {
    match platform {
        Platform::Cli => Some("cli"),
        Platform::Api => Some("api"),
        Platform::WebChat => Some("webchat"),
        _ => None,
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

fn merge_json(base: serde_json::Value, overlay: serde_json::Value) -> serde_json::Value {
    let mut object = base.as_object().cloned().unwrap_or_default();
    if let Some(overlay) = overlay.as_object() {
        for (key, value) in overlay {
            object.insert(key.clone(), value.clone());
        }
    }
    serde_json::Value::Object(object)
}

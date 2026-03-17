//! Inter-Agent Communication Tools
//!
//! These tools enable agents to coordinate work across sessions by listing
//! active sessions, retrieving message history, sending messages to other
//! sessions, and spawning new agent instances.

use async_trait::async_trait;
use openrustclaw_core::error::{Error, Result, ToolError};
use openrustclaw_core::traits::{Tool, ToolContext};
use openrustclaw_core::types::{SkillCapability, ToolOutput};
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};

use crate::tools::ToolRegistry;

// =============================================================================
// Placeholder traits - to be implemented by the runtime
// =============================================================================

/// Information about an agent session.
#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    /// Unique session identifier.
    pub id: String,
    /// Workspace the session belongs to.
    pub workspace: String,
    /// Whether the session is currently active.
    pub is_active: bool,
    /// When the session was created.
    pub created_at: String,
    /// Last activity timestamp.
    pub last_activity: String,
    /// Session capabilities.
    pub capabilities: Vec<String>,
}

/// Trait for agent session registry.
#[async_trait]
pub trait AgentRegistry: Send + Sync {
    /// List all active sessions.
    async fn list_sessions(&self) -> Vec<SessionInfo>;
    /// Get a specific session by ID.
    async fn get_session(&self, session_id: &str) -> Option<SessionInfo>;
}

/// Trait for session message storage.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Get message history for a session.
    async fn get_history(&self, session_id: &str, limit: usize) -> Result<Vec<SessionMessage>>;
    /// Get messages before a specific message ID.
    async fn get_history_before(
        &self,
        session_id: &str,
        before_message_id: &str,
        limit: usize,
    ) -> Result<Vec<SessionMessage>>;
}

/// A message in a session's history.
#[derive(Debug, Clone, Serialize)]
pub struct SessionMessage {
    /// Message ID.
    pub id: String,
    /// Role of the message sender.
    pub role: String,
    /// Message content.
    pub content: String,
    /// Timestamp.
    pub timestamp: String,
}

/// Trait for message routing between sessions.
#[async_trait]
pub trait MessageRouter: Send + Sync {
    /// Send a message to a session (fire and forget).
    async fn send(&self, target_session_id: &str, message: &str) -> Result<()>;
    /// Send a message and wait for a reply.
    async fn send_and_wait(
        &self,
        target_session_id: &str,
        message: &str,
        from_session_id: &str,
    ) -> Result<String>;
}

/// Trait for agent factory.
#[async_trait]
pub trait AgentFactory: Send + Sync {
    /// Spawn a new agent session.
    async fn spawn(
        &self,
        workspace: &str,
        initial_prompt: &str,
        model: Option<&str>,
        capabilities: Vec<String>,
    ) -> Result<String>;
}

/// Trait for workspace management.
#[async_trait]
pub trait WorkspaceManager: Send + Sync {
    /// Check if a workspace exists.
    async fn exists(&self, workspace: &str) -> bool;
    /// Get default model for a workspace.
    async fn default_model(&self, workspace: &str) -> Option<String>;
}

// =============================================================================
// Dependencies container
// =============================================================================

/// Dependencies required for inter-agent communication tools.
#[derive(Clone)]
pub struct InterAgentDeps {
    /// Agent registry for listing sessions.
    pub registry: Arc<RwLock<dyn AgentRegistry>>,
    /// Session store for retrieving message history.
    pub session_store: Arc<dyn SessionStore>,
    /// Message router for sending messages between sessions.
    pub router: Arc<dyn MessageRouter>,
    /// Agent factory for spawning new sessions.
    pub factory: Arc<dyn AgentFactory>,
    /// Workspace manager for workspace operations.
    pub workspace_manager: Arc<dyn WorkspaceManager>,
}

impl InterAgentDeps {
    /// Create a new dependencies container.
    pub fn new(
        registry: Arc<RwLock<dyn AgentRegistry>>,
        session_store: Arc<dyn SessionStore>,
        router: Arc<dyn MessageRouter>,
        factory: Arc<dyn AgentFactory>,
        workspace_manager: Arc<dyn WorkspaceManager>,
    ) -> Self {
        Self {
            registry,
            session_store,
            router,
            factory,
            workspace_manager,
        }
    }
}

// =============================================================================
// Tool: sessions_list
// =============================================================================

/// Tool: sessions_list - List active agent sessions
pub struct SessionsListTool {
    agent_registry: Arc<RwLock<dyn AgentRegistry>>,
}

impl SessionsListTool {
    /// Create a new SessionsListTool.
    pub fn new(agent_registry: Arc<RwLock<dyn AgentRegistry>>) -> Self {
        Self { agent_registry }
    }
}

#[async_trait]
impl Tool for SessionsListTool {
    fn name(&self) -> &str {
        "sessions_list"
    }

    fn description(&self) -> &str {
        "List all active agent sessions with their metadata"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "workspace": {
                    "type": "string",
                    "description": "Filter by workspace"
                },
                "include_inactive": {
                    "type": "boolean",
                    "description": "Include recently inactive sessions"
                }
            }
        })
    }

    fn capabilities_required(&self) -> Vec<SkillCapability> {
        vec![] // Session listing is a basic operation
    }

    #[instrument(skip(self, input))]
    async fn execute(&self, input: serde_json::Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let registry = self.agent_registry.read().await;

        let workspace_filter = input.get("workspace").and_then(|w| w.as_str());
        let include_inactive = input
            .get("include_inactive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        debug!(
            workspace_filter = ?workspace_filter,
            include_inactive = include_inactive,
            "Listing sessions"
        );

        let sessions: Vec<SessionInfo> = registry
            .list_sessions()
            .await
            .into_iter()
            .filter(|s| workspace_filter.map(|w| s.workspace == w).unwrap_or(true))
            .filter(|s| include_inactive || s.is_active)
            .collect();

        let count = sessions.len();
        info!(count = count, "Session list retrieved");

        let result = json!({
            "sessions": sessions,
            "count": count
        });

        Ok(ToolOutput {
            tool_call_id: String::new(),
            content: result.to_string(),
            is_error: false,
        })
    }
}

// =============================================================================
// Tool: sessions_history
// =============================================================================

/// Tool: sessions_history - Get session transcript
pub struct SessionsHistoryTool {
    session_store: Arc<dyn SessionStore>,
}

impl SessionsHistoryTool {
    /// Create a new SessionsHistoryTool.
    pub fn new(session_store: Arc<dyn SessionStore>) -> Self {
        Self { session_store }
    }
}

#[async_trait]
impl Tool for SessionsHistoryTool {
    fn name(&self) -> &str {
        "sessions_history"
    }

    fn description(&self) -> &str {
        "Fetch message history/transcript for a specific session"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "ID of the session"
                },
                "limit": {
                    "type": "integer",
                    "description": "Number of messages to retrieve",
                    "default": 50
                },
                "before_message_id": {
                    "type": "string",
                    "description": "Get messages before this ID (pagination)"
                }
            },
            "required": ["session_id"]
        })
    }

    fn capabilities_required(&self) -> Vec<SkillCapability> {
        vec![] // History retrieval is a basic operation
    }

    #[instrument(skip(self, input))]
    async fn execute(&self, input: serde_json::Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let session_id = input
            .get("session_id")
            .and_then(|s| s.as_str())
            .ok_or_else(|| {
                Error::Tool(ToolError::InputValidation {
                    tool: self.name().to_string(),
                    message: "Missing or invalid 'session_id' parameter".to_string(),
                })
            })?;

        let limit = input
            .get("limit")
            .and_then(|l| l.as_u64())
            .map(|v| v as usize)
            .unwrap_or(50);

        debug!(session_id = %session_id, limit = limit, "Fetching session history");

        let history = if let Some(before_id) = input.get("before_message_id").and_then(|b| b.as_str()) {
            self.session_store
                .get_history_before(session_id, before_id, limit)
                .await?
        } else {
            self.session_store.get_history(session_id, limit).await?
        };

        info!(
            session_id = %session_id,
            message_count = history.len(),
            "Session history retrieved"
        );

        // Serialization should not fail for our types, but handle gracefully
        let content = serde_json::to_string(&history).unwrap_or_else(|e| {
            json!({"error": format!("Failed to serialize history: {}", e)}).to_string()
        });

        Ok(ToolOutput {
            tool_call_id: String::new(),
            content,
            is_error: false,
        })
    }
}

// =============================================================================
// Tool: sessions_send
// =============================================================================

/// Options for sending messages between sessions.
#[derive(Debug, Clone)]
pub struct SendOptions {
    /// Whether to wait for a reply.
    pub expect_reply: bool,
    /// Timeout for reply in seconds.
    pub reply_timeout_secs: u64,
    /// Whether to announce this step to the user.
    pub announce_step: bool,
}

/// Tool: sessions_send - Send message to another session
pub struct SessionsSendTool {
    message_router: Arc<dyn MessageRouter>,
}

impl SessionsSendTool {
    /// Create a new SessionsSendTool.
    pub fn new(message_router: Arc<dyn MessageRouter>) -> Self {
        Self { message_router }
    }
}

#[async_trait]
impl Tool for SessionsSendTool {
    fn name(&self) -> &str {
        "sessions_send"
    }

    fn description(&self) -> &str {
        "Send a message to another agent session. Can optionally wait for a reply."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "target_session_id": {
                    "type": "string",
                    "description": "ID of the target session"
                },
                "message": {
                    "type": "string",
                    "description": "Message to send"
                },
                "expect_reply": {
                    "type": "boolean",
                    "description": "Whether to wait for a reply",
                    "default": false
                },
                "reply_timeout_secs": {
                    "type": "integer",
                    "description": "Timeout for reply (seconds)",
                    "default": 60
                },
                "announce_step": {
                    "type": "boolean",
                    "description": "Announce this step to the user",
                    "default": true
                }
            },
            "required": ["target_session_id", "message"]
        })
    }

    fn capabilities_required(&self) -> Vec<SkillCapability> {
        vec![] // Message sending is a basic operation
    }

    #[instrument(skip(self, input, ctx), fields(session_id = %ctx.session_id))]
    async fn execute(&self, input: serde_json::Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let target_id = input
            .get("target_session_id")
            .and_then(|s| s.as_str())
            .ok_or_else(|| {
                Error::Tool(ToolError::InputValidation {
                    tool: self.name().to_string(),
                    message: "Missing or invalid 'target_session_id' parameter".to_string(),
                })
            })?;

        let message = input
            .get("message")
            .and_then(|m| m.as_str())
            .ok_or_else(|| {
                Error::Tool(ToolError::InputValidation {
                    tool: self.name().to_string(),
                    message: "Missing or invalid 'message' parameter".to_string(),
                })
            })?;

        let options = SendOptions {
            expect_reply: input
                .get("expect_reply")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            reply_timeout_secs: input
                .get("reply_timeout_secs")
                .and_then(|v| v.as_u64())
                .unwrap_or(60),
            announce_step: input
                .get("announce_step")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
        };

        debug!(
            target_id = %target_id,
            expect_reply = options.expect_reply,
            "Sending message to session"
        );

        let result = if options.expect_reply {
            // Send and wait for reply
            match tokio::time::timeout(
                Duration::from_secs(options.reply_timeout_secs),
                self.message_router
                    .send_and_wait(target_id, message, &ctx.session_id),
            )
            .await
            {
                Ok(Ok(reply)) => {
                    info!(target_id = %target_id, "Message sent and reply received");
                    json!({
                        "status": "success",
                        "reply": reply,
                        "from_session": target_id
                    })
                }
                Ok(Err(e)) => {
                    warn!(target_id = %target_id, error = %e, "Failed to send message");
                    json!({
                        "status": "error",
                        "error": format!("Failed to send: {}", e)
                    })
                }
                Err(_) => {
                    warn!(target_id = %target_id, "Message reply timeout");
                    json!({
                        "status": "timeout",
                        "error": "No reply received within timeout"
                    })
                }
            }
        } else {
            // Fire and forget
            match self.message_router.send(target_id, message).await {
                Ok(()) => {
                    info!(target_id = %target_id, "Message sent");
                    json!({
                        "status": "sent",
                        "to_session": target_id
                    })
                }
                Err(e) => {
                    warn!(target_id = %target_id, error = %e, "Failed to send message");
                    json!({
                        "status": "error",
                        "error": format!("Failed to send: {}", e)
                    })
                }
            }
        };

        Ok(ToolOutput {
            tool_call_id: String::new(),
            content: result.to_string(),
            is_error: result.get("status") == Some(&json!("error")),
        })
    }
}

// =============================================================================
// Tool: sessions_spawn
// =============================================================================

/// Tool: sessions_spawn - Spawn a new agent session
pub struct SessionsSpawnTool {
    agent_factory: Arc<dyn AgentFactory>,
    workspace_manager: Arc<dyn WorkspaceManager>,
}

impl SessionsSpawnTool {
    /// Create a new SessionsSpawnTool.
    pub fn new(
        agent_factory: Arc<dyn AgentFactory>,
        workspace_manager: Arc<dyn WorkspaceManager>,
    ) -> Self {
        Self {
            agent_factory,
            workspace_manager,
        }
    }
}

#[async_trait]
impl Tool for SessionsSpawnTool {
    fn name(&self) -> &str {
        "sessions_spawn"
    }

    fn description(&self) -> &str {
        "Spawn a new agent session in a workspace with optional initial context"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "workspace": {
                    "type": "string",
                    "description": "Workspace name"
                },
                "initial_prompt": {
                    "type": "string",
                    "description": "Initial system prompt for the agent"
                },
                "model": {
                    "type": "string",
                    "description": "Model to use (optional, defaults to workspace default)"
                },
                "capabilities": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Capabilities to enable"
                }
            },
            "required": ["workspace", "initial_prompt"]
        })
    }

    fn capabilities_required(&self) -> Vec<SkillCapability> {
        vec![] // Session spawning requires basic permissions
    }

    #[instrument(skip(self, input))]
    async fn execute(&self, input: serde_json::Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let workspace = input
            .get("workspace")
            .and_then(|w| w.as_str())
            .ok_or_else(|| {
                Error::Tool(ToolError::InputValidation {
                    tool: self.name().to_string(),
                    message: "Missing or invalid 'workspace' parameter".to_string(),
                })
            })?;

        // Validate workspace exists
        if !self.workspace_manager.exists(workspace).await {
            return Ok(ToolOutput {
                tool_call_id: String::new(),
                content: json!({
                    "status": "error",
                    "error": format!("Workspace '{}' does not exist", workspace)
                })
                .to_string(),
                is_error: true,
            });
        }

        let initial_prompt = input
            .get("initial_prompt")
            .and_then(|p| p.as_str())
            .ok_or_else(|| {
                Error::Tool(ToolError::InputValidation {
                    tool: self.name().to_string(),
                    message: "Missing or invalid 'initial_prompt' parameter".to_string(),
                })
            })?;

        let model = input.get("model").and_then(|m| m.as_str());

        let capabilities: Vec<String> = input
            .get("capabilities")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        debug!(
            workspace = %workspace,
            model = ?model,
            capabilities_count = capabilities.len(),
            "Spawning new agent session"
        );

        match self
            .agent_factory
            .spawn(workspace, initial_prompt, model, capabilities)
            .await
        {
            Ok(session_id) => {
                info!(session_id = %session_id, workspace = %workspace, "Agent session spawned");
                let result = json!({
                    "session_id": session_id,
                    "workspace": workspace,
                    "status": "spawned"
                });
                Ok(ToolOutput {
                    tool_call_id: String::new(),
                    content: result.to_string(),
                    is_error: false,
                })
            }
            Err(e) => {
                warn!(workspace = %workspace, error = %e, "Failed to spawn session");
                Ok(ToolOutput {
                    tool_call_id: String::new(),
                    content: json!({
                        "status": "error",
                        "error": format!("Failed to spawn session: {}", e)
                    })
                    .to_string(),
                    is_error: true,
                })
            }
        }
    }
}

// =============================================================================
// Registration
// =============================================================================

/// Register all inter-agent communication tools with the registry.
pub fn register_inter_agent_tools(registry: &mut ToolRegistry, deps: InterAgentDeps) {
    registry.register(Arc::new(SessionsListTool::new(deps.registry)));
    registry.register(Arc::new(SessionsHistoryTool::new(deps.session_store)));
    registry.register(Arc::new(SessionsSendTool::new(deps.router)));
    registry.register(Arc::new(SessionsSpawnTool::new(
        deps.factory,
        deps.workspace_manager,
    )));
    info!("Registered inter-agent communication tools");
}

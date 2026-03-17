//! Multi-Agent Routing System
//!
//! This module provides workspace isolation and specialized agent routing,
//! enabling sophisticated multi-agent workflows where different agents handle
//! different types of requests based on routing rules.
//!
//! # Example
//! ```rust
//! use openrustclaw_agent::routing::{AgentRouter, AgentId, RoutingRule, KeywordMatcher};
//! use openrustclaw_core::types::Message;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let mut router = AgentRouter::new(AgentId::new("main"));
//!
//! // Add a routing rule for code-related requests
//! router.add_rule(RoutingRule::new(
//!     "Code requests",
//!     100,
//!     Box::new(KeywordMatcher::any(&["code", "program", "function"])),
//!     AgentId::new("dev"),
//! ));
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::fmt;

use async_trait::async_trait;
use openrustclaw_core::types::{Message, SessionType};
use regex::Regex;
use tokio::sync::mpsc;
use tracing::{debug, trace, warn};

/// Unique identifier for an agent
#[derive(Debug, Clone, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    /// Create a new agent ID from a string
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for AgentId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for AgentId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Handle to an agent instance
#[derive(Debug)]
pub struct AgentHandle {
    /// Unique identifier for this agent
    pub id: AgentId,
    /// Human-readable name
    pub name: String,
    /// Workspace this agent belongs to
    pub workspace: String,
    /// Type of session this agent handles
    pub session_type: SessionType,
    /// Capabilities this agent provides (e.g., "code", "git", "sql")
    pub capabilities: Vec<String>,
    /// Channel sender for routing messages to this agent
    pub sender: mpsc::Sender<Message>,
}

impl AgentHandle {
    /// Create a new agent handle
    pub fn new(
        id: impl Into<AgentId>,
        name: impl Into<String>,
        workspace: impl Into<String>,
        session_type: SessionType,
        sender: mpsc::Sender<Message>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            workspace: workspace.into(),
            session_type,
            capabilities: Vec::new(),
            sender,
        }
    }

    /// Add a capability to this agent
    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities.push(capability.into());
        self
    }

    /// Add multiple capabilities
    pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.capabilities = capabilities;
        self
    }
}

/// Routing rule with priority
#[derive(Debug)]
pub struct RoutingRule {
    /// Human-readable name for this rule
    pub name: String,
    /// Priority - higher values are evaluated first
    pub priority: u32,
    /// Matcher that determines if this rule applies
    pub matcher: Box<dyn RouteMatcher + Send + Sync>,
    /// Target agent to route to when this rule matches
    pub target_agent: AgentId,
}

impl RoutingRule {
    /// Create a new routing rule
    pub fn new(
        name: impl Into<String>,
        priority: u32,
        matcher: Box<dyn RouteMatcher + Send + Sync>,
        target_agent: impl Into<AgentId>,
    ) -> Self {
        Self {
            name: name.into(),
            priority,
            matcher,
            target_agent: target_agent.into(),
        }
    }
}

/// Trait for matching routes
#[async_trait]
pub trait RouteMatcher: Send + Sync + fmt::Debug {
    /// Check if this matcher applies to the given message and context
    async fn matches(&self, message: &Message, context: &RoutingContext) -> bool;
}

/// Context for routing decisions
#[derive(Debug, Clone, Default)]
pub struct RoutingContext {
    /// The channel/platform the message came from
    pub channel: String,
    /// ID of the sender
    pub sender_id: String,
    /// Whether this is a direct message
    pub is_dm: bool,
    /// Whether the agent was mentioned
    pub is_mention: bool,
    /// Group/channel ID if applicable
    pub group_id: Option<String>,
    /// Workspace scope
    pub workspace: Option<String>,
    /// Original platform message metadata
    pub metadata: serde_json::Value,
}

impl RoutingContext {
    /// Create a new routing context
    pub fn new(channel: impl Into<String>, sender_id: impl Into<String>) -> Self {
        Self {
            channel: channel.into(),
            sender_id: sender_id.into(),
            is_dm: false,
            is_mention: false,
            group_id: None,
            workspace: None,
            metadata: serde_json::json!({}),
        }
    }

    /// Set DM flag
    pub fn with_dm(mut self, is_dm: bool) -> Self {
        self.is_dm = is_dm;
        self
    }

    /// Set mention flag
    pub fn with_mention(mut self, is_mention: bool) -> Self {
        self.is_mention = is_mention;
        self
    }

    /// Set group ID
    pub fn with_group_id(mut self, group_id: impl Into<String>) -> Self {
        self.group_id = Some(group_id.into());
        self
    }

    /// Set workspace
    pub fn with_workspace(mut self, workspace: impl Into<String>) -> Self {
        self.workspace = Some(workspace.into());
        self
    }
}

/// Sandbox mode for workspace isolation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SandboxMode {
    /// Direct host access (no sandbox)
    #[default]
    None,
    /// Per-session Docker containers
    Docker,
    /// WASM sandbox
    Wasm,
}

impl fmt::Display for SandboxMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SandboxMode::None => write!(f, "none"),
            SandboxMode::Docker => write!(f, "docker"),
            SandboxMode::Wasm => write!(f, "wasm"),
        }
    }
}

/// Isolated workspace for agents
#[derive(Debug, Clone)]
pub struct Workspace {
    /// Name of the workspace
    pub name: String,
    /// Agents assigned to this workspace
    pub agents: Vec<AgentId>,
    /// Whether agents share memory within this workspace
    pub shared_memory: bool,
    /// Sandbox mode for this workspace
    pub sandbox_mode: SandboxMode,
    /// Workspace-specific configuration
    pub config: serde_json::Value,
}

impl Workspace {
    /// Create a new workspace
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            agents: Vec::new(),
            shared_memory: false,
            sandbox_mode: SandboxMode::None,
            config: serde_json::json!({}),
        }
    }

    /// Enable shared memory for this workspace
    pub fn with_shared_memory(mut self, enabled: bool) -> Self {
        self.shared_memory = enabled;
        self
    }

    /// Set sandbox mode
    pub fn with_sandbox_mode(mut self, mode: SandboxMode) -> Self {
        self.sandbox_mode = mode;
        self
    }

    /// Set configuration
    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = config;
        self
    }

    /// Add an agent to this workspace
    pub fn add_agent(&mut self, agent_id: impl Into<AgentId>) {
        self.agents.push(agent_id.into());
    }
}

/// Result of a routing decision
#[derive(Debug, Clone)]
pub struct RoutingResult {
    /// The target agent ID
    pub agent_id: AgentId,
    /// The rule that matched (if any)
    pub matched_rule: Option<String>,
    /// Whether this is the default agent
    pub is_default: bool,
}

/// Agent router that directs messages to appropriate agents
#[derive(Debug)]
pub struct AgentRouter {
    agents: HashMap<AgentId, AgentHandle>,
    routing_rules: Vec<RoutingRule>,
    default_agent: AgentId,
    workspaces: HashMap<String, Workspace>,
}

impl AgentRouter {
    /// Create a new agent router with a default agent
    pub fn new(default_agent: impl Into<AgentId>) -> Self {
        Self {
            agents: HashMap::new(),
            routing_rules: Vec::new(),
            default_agent: default_agent.into(),
            workspaces: HashMap::new(),
        }
    }

    /// Register an agent
    pub fn register_agent(&mut self, handle: AgentHandle) {
        debug!(agent_id = %handle.id, name = %handle.name, "Registering agent");
        self.agents.insert(handle.id.clone(), handle);
    }

    /// Unregister an agent
    pub fn unregister_agent(&mut self, agent_id: &AgentId) -> Option<AgentHandle> {
        debug!(agent_id = %agent_id, "Unregistering agent");
        self.agents.remove(agent_id)
    }

    /// Get an agent by ID
    pub fn get_agent(&self, agent_id: &AgentId) -> Option<&AgentHandle> {
        self.agents.get(agent_id)
    }

    /// Get a mutable reference to an agent
    pub fn get_agent_mut(&mut self, agent_id: &AgentId) -> Option<&mut AgentHandle> {
        self.agents.get_mut(agent_id)
    }

    /// Check if an agent is registered
    pub fn has_agent(&self, agent_id: &AgentId) -> bool {
        self.agents.contains_key(agent_id)
    }

    /// List all registered agents
    pub fn list_agents(&self) -> Vec<&AgentHandle> {
        self.agents.values().collect()
    }

    /// Add a routing rule
    pub fn add_rule(&mut self, rule: RoutingRule) {
        debug!(rule = %rule.name, priority = rule.priority, "Adding routing rule");
        self.routing_rules.push(rule);
        // Sort by priority (highest first)
        self.routing_rules
            .sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Remove a routing rule by name
    pub fn remove_rule(&mut self, name: &str) -> bool {
        let len_before = self.routing_rules.len();
        self.routing_rules.retain(|r| r.name != name);
        self.routing_rules.len() < len_before
    }

    /// List all routing rules
    pub fn list_rules(&self) -> &[RoutingRule] {
        &self.routing_rules
    }

    /// Route a message to the appropriate agent
    pub async fn route(
        &self,
        message: &Message,
        context: &RoutingContext,
    ) -> anyhow::Result<RoutingResult> {
        trace!(content = %message.content, channel = %context.channel, "Routing message");

        // Check rules in priority order
        for rule in &self.routing_rules {
            if rule.matcher.matches(message, context).await {
                if self.agents.contains_key(&rule.target_agent) {
                    debug!(rule = %rule.name, agent = %rule.target_agent, "Routing rule matched");
                    return Ok(RoutingResult {
                        agent_id: rule.target_agent.clone(),
                        matched_rule: Some(rule.name.clone()),
                        is_default: false,
                    });
                } else {
                    warn!(
                        rule = %rule.name,
                        agent = %rule.target_agent,
                        "Rule matched but target agent not found"
                    );
                }
            }
        }

        // Fall back to default agent
        debug!(agent = %self.default_agent, "Using default agent");
        Ok(RoutingResult {
            agent_id: self.default_agent.clone(),
            matched_rule: None,
            is_default: true,
        })
    }

    /// Route and send a message to the appropriate agent
    pub async fn route_and_send(
        &self,
        message: Message,
        context: &RoutingContext,
    ) -> anyhow::Result<()> {
        let result = self.route(&message, context).await?;

        if let Some(agent) = self.agents.get(&result.agent_id) {
            agent.sender.send(message).await?;
            Ok(())
        } else {
            anyhow::bail!("Target agent not found: {}", result.agent_id);
        }
    }

    /// Get or create a workspace
    pub fn workspace(&mut self, name: &str) -> &mut Workspace {
        self.workspaces
            .entry(name.to_string())
            .or_insert_with(|| Workspace::new(name))
    }

    /// Get a workspace if it exists
    pub fn get_workspace(&self, name: &str) -> Option<&Workspace> {
        self.workspaces.get(name)
    }

    /// Check if a workspace exists
    pub fn has_workspace(&self, name: &str) -> bool {
        self.workspaces.contains_key(name)
    }

    /// Remove a workspace
    pub fn remove_workspace(&mut self, name: &str) -> Option<Workspace> {
        self.workspaces.remove(name)
    }

    /// List all workspaces
    pub fn list_workspaces(&self) -> Vec<&Workspace> {
        self.workspaces.values().collect()
    }

    /// Set the default agent
    pub fn set_default_agent(&mut self, agent_id: impl Into<AgentId>) {
        self.default_agent = agent_id.into();
    }

    /// Get the default agent ID
    pub fn default_agent(&self) -> &AgentId {
        &self.default_agent
    }

    /// Find agents by capability
    pub fn find_by_capability(&self, capability: &str) -> Vec<&AgentHandle> {
        self.agents
            .values()
            .filter(|a| {
                a.capabilities
                    .iter()
                    .any(|c| c.eq_ignore_ascii_case(capability))
            })
            .collect()
    }

    /// Find agents in a workspace
    pub fn find_by_workspace(&self, workspace: &str) -> Vec<&AgentHandle> {
        self.agents
            .values()
            .filter(|a| a.workspace == workspace)
            .collect()
    }
}

// ============================================================================
// Built-in Route Matchers
// ============================================================================

/// Regex-based route matcher
#[derive(Debug)]
pub struct RegexMatcher {
    pattern: Regex,
}

impl RegexMatcher {
    /// Create a new regex matcher
    pub fn new(pattern: &str) -> anyhow::Result<Self> {
        Ok(Self {
            pattern: Regex::new(pattern)?,
        })
    }

    /// Create a new regex matcher with a pre-compiled regex
    pub fn with_regex(regex: Regex) -> Self {
        Self { pattern: regex }
    }
}

#[async_trait]
impl RouteMatcher for RegexMatcher {
    async fn matches(&self, message: &Message, _context: &RoutingContext) -> bool {
        self.pattern.is_match(&message.content)
    }
}

/// Keyword-based route matcher
#[derive(Debug)]
pub struct KeywordMatcher {
    keywords: Vec<String>,
    require_all: bool,
    case_sensitive: bool,
}

impl KeywordMatcher {
    /// Create a matcher that requires ANY of the keywords
    pub fn any(keywords: &[impl AsRef<str>]) -> Self {
        Self {
            keywords: keywords.iter().map(|k| k.as_ref().to_string()).collect(),
            require_all: false,
            case_sensitive: false,
        }
    }

    /// Create a matcher that requires ALL of the keywords
    pub fn all(keywords: &[impl AsRef<str>]) -> Self {
        Self {
            keywords: keywords.iter().map(|k| k.as_ref().to_string()).collect(),
            require_all: true,
            case_sensitive: false,
        }
    }

    /// Set case sensitivity
    pub fn case_sensitive(mut self, sensitive: bool) -> Self {
        self.case_sensitive = sensitive;
        self
    }
}

#[async_trait]
impl RouteMatcher for KeywordMatcher {
    async fn matches(&self, message: &Message, _context: &RoutingContext) -> bool {
        let content = if self.case_sensitive {
            message.content.clone()
        } else {
            message.content.to_lowercase()
        };

        if self.require_all {
            self.keywords.iter().all(|k| {
                let keyword = if self.case_sensitive {
                    k.clone()
                } else {
                    k.to_lowercase()
                };
                content.contains(&keyword)
            })
        } else {
            self.keywords.iter().any(|k| {
                let keyword = if self.case_sensitive {
                    k.clone()
                } else {
                    k.to_lowercase()
                };
                content.contains(&keyword)
            })
        }
    }
}

/// Channel-based route matcher
#[derive(Debug)]
pub struct ChannelMatcher {
    channels: Vec<String>,
}

impl ChannelMatcher {
    /// Create a new channel matcher
    pub fn new(channels: Vec<String>) -> Self {
        Self { channels }
    }

    /// Create a matcher for a single channel
    pub fn single(channel: impl Into<String>) -> Self {
        Self {
            channels: vec![channel.into()],
        }
    }
}

#[async_trait]
impl RouteMatcher for ChannelMatcher {
    async fn matches(&self, _message: &Message, context: &RoutingContext) -> bool {
        self.channels.contains(&context.channel)
    }
}

/// Workspace-based route matcher
#[derive(Debug)]
pub struct WorkspaceMatcher {
    workspace: String,
}

impl WorkspaceMatcher {
    /// Create a new workspace matcher
    pub fn new(workspace: impl Into<String>) -> Self {
        Self {
            workspace: workspace.into(),
        }
    }
}

#[async_trait]
impl RouteMatcher for WorkspaceMatcher {
    async fn matches(&self, _message: &Message, context: &RoutingContext) -> bool {
        context.workspace.as_ref() == Some(&self.workspace)
    }
}

/// DM-only route matcher
#[derive(Debug, Default)]
pub struct DmMatcher;

impl DmMatcher {
    /// Create a new DM matcher
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RouteMatcher for DmMatcher {
    async fn matches(&self, _message: &Message, context: &RoutingContext) -> bool {
        context.is_dm
    }
}

/// Mention-only route matcher
#[derive(Debug, Default)]
pub struct MentionMatcher;

impl MentionMatcher {
    /// Create a new mention matcher
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RouteMatcher for MentionMatcher {
    async fn matches(&self, _message: &Message, context: &RoutingContext) -> bool {
        context.is_mention
    }
}

/// Composite matcher that requires ALL of the inner matchers to match
#[derive(Debug)]
pub struct AllMatcher {
    matchers: Vec<Box<dyn RouteMatcher + Send + Sync>>,
}

impl AllMatcher {
    /// Create a new composite matcher
    pub fn new(matchers: Vec<Box<dyn RouteMatcher + Send + Sync>>) -> Self {
        Self { matchers }
    }
}

#[async_trait]
impl RouteMatcher for AllMatcher {
    async fn matches(&self, message: &Message, context: &RoutingContext) -> bool {
        for matcher in &self.matchers {
            if !matcher.matches(message, context).await {
                return false;
            }
        }
        true
    }
}

/// Composite matcher that requires ANY of the inner matchers to match
#[derive(Debug)]
pub struct AnyMatcher {
    matchers: Vec<Box<dyn RouteMatcher + Send + Sync>>,
}

impl AnyMatcher {
    /// Create a new composite matcher
    pub fn new(matchers: Vec<Box<dyn RouteMatcher + Send + Sync>>) -> Self {
        Self { matchers }
    }
}

#[async_trait]
impl RouteMatcher for AnyMatcher {
    async fn matches(&self, message: &Message, context: &RoutingContext) -> bool {
        for matcher in &self.matchers {
            if matcher.matches(message, context).await {
                return true;
            }
        }
        false
    }
}

/// Capability-based matcher that checks agent capabilities
#[derive(Debug)]
pub struct CapabilityMatcher {
    required_capabilities: Vec<String>,
    require_all: bool,
}

impl CapabilityMatcher {
    /// Create a matcher that requires ANY of the capabilities
    pub fn any(capabilities: &[impl AsRef<str>]) -> Self {
        Self {
            required_capabilities: capabilities
                .iter()
                .map(|c| c.as_ref().to_string())
                .collect(),
            require_all: false,
        }
    }

    /// Create a matcher that requires ALL of the capabilities
    pub fn all(capabilities: &[impl AsRef<str>]) -> Self {
        Self {
            required_capabilities: capabilities
                .iter()
                .map(|c| c.as_ref().to_string())
                .collect(),
            require_all: true,
        }
    }
}

#[async_trait]
impl RouteMatcher for CapabilityMatcher {
    async fn matches(&self, message: &Message, _context: &RoutingContext) -> bool {
        // This matcher checks message content for capability indicators
        // For example, code-related keywords indicate "code" capability
        let content_lower = message.content.to_lowercase();

        let matches_iter = self.required_capabilities.iter().map(|cap| {
            let cap_lower = cap.to_lowercase();
            // Check for capability keywords in the message
            match cap_lower.as_str() {
                "code" => {
                    content_lower.contains("code")
                        || content_lower.contains("program")
                        || content_lower.contains("function")
                        || content_lower.contains("bug")
                }
                "git" => {
                    content_lower.contains("git")
                        || content_lower.contains("commit")
                        || content_lower.contains("branch")
                }
                "sql" | "database" => {
                    content_lower.contains("sql")
                        || content_lower.contains("database")
                        || content_lower.contains("query")
                }
                "shell" => {
                    content_lower.contains("shell")
                        || content_lower.contains("command")
                        || content_lower.contains("terminal")
                }
                _ => content_lower.contains(&cap_lower),
            }
        });

        if self.require_all {
            matches_iter.clone().all(|m| m)
        } else {
            matches_iter.clone().any(|m| m)
        }
    }
}

// ============================================================================
// Builder for convenient router construction
// ============================================================================

/// Builder for constructing an AgentRouter
#[derive(Debug)]
pub struct AgentRouterBuilder {
    default_agent: AgentId,
    agents: Vec<AgentHandle>,
    rules: Vec<RoutingRule>,
    workspaces: Vec<Workspace>,
}

impl AgentRouterBuilder {
    /// Create a new builder with a default agent
    pub fn new(default_agent: impl Into<AgentId>) -> Self {
        Self {
            default_agent: default_agent.into(),
            agents: Vec::new(),
            rules: Vec::new(),
            workspaces: Vec::new(),
        }
    }

    /// Add an agent
    pub fn with_agent(mut self, agent: AgentHandle) -> Self {
        self.agents.push(agent);
        self
    }

    /// Add a routing rule
    pub fn with_rule(mut self, rule: RoutingRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Add a workspace
    pub fn with_workspace(mut self, workspace: Workspace) -> Self {
        self.workspaces.push(workspace);
        self
    }

    /// Build the router
    pub fn build(self) -> AgentRouter {
        let mut router = AgentRouter::new(self.default_agent);

        for agent in self.agents {
            router.register_agent(agent);
        }

        for rule in self.rules {
            router.add_rule(rule);
        }

        for workspace in self.workspaces {
            router.workspaces.insert(workspace.name.clone(), workspace);
        }

        router
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openrustclaw_core::types::{Message, Role};

    fn create_test_message(content: &str) -> Message {
        Message::new(Role::User, content)
    }

    #[tokio::test]
    async fn test_keyword_matcher_any() {
        let matcher = KeywordMatcher::any(&["code", "program"]);
        let ctx = RoutingContext::default();

        assert!(
            matcher
                .matches(&create_test_message("I need help with code"), &ctx)
                .await
        );
        assert!(
            matcher
                .matches(&create_test_message("Program something"), &ctx)
                .await
        );
        assert!(
            !matcher
                .matches(&create_test_message("Hello world"), &ctx)
                .await
        );
    }

    #[tokio::test]
    async fn test_keyword_matcher_all() {
        let matcher = KeywordMatcher::all(&["code", "review"]);
        let ctx = RoutingContext::default();

        assert!(
            matcher
                .matches(&create_test_message("code review please"), &ctx)
                .await
        );
        assert!(
            !matcher
                .matches(&create_test_message("code something"), &ctx)
                .await
        );
    }

    #[tokio::test]
    async fn test_regex_matcher() {
        let matcher = RegexMatcher::new(r"\b\w+\.rs\b").unwrap();
        let ctx = RoutingContext::default();

        assert!(
            matcher
                .matches(&create_test_message("Check main.rs file"), &ctx)
                .await
        );
        assert!(
            !matcher
                .matches(&create_test_message("Check main.py file"), &ctx)
                .await
        );
    }

    #[tokio::test]
    async fn test_channel_matcher() {
        let matcher = ChannelMatcher::single("discord");
        let ctx = RoutingContext::new("discord", "user1");
        let ctx2 = RoutingContext::new("slack", "user1");

        assert!(matcher.matches(&create_test_message("Hello"), &ctx).await);
        assert!(!matcher.matches(&create_test_message("Hello"), &ctx2).await);
    }

    #[tokio::test]
    async fn test_workspace_matcher() {
        let matcher = WorkspaceMatcher::new("dev");
        let ctx = RoutingContext::new("web", "user1").with_workspace("dev");
        let ctx2 = RoutingContext::new("web", "user1").with_workspace("prod");

        assert!(matcher.matches(&create_test_message("Hello"), &ctx).await);
        assert!(!matcher.matches(&create_test_message("Hello"), &ctx2).await);
    }

    #[tokio::test]
    async fn test_dm_matcher() {
        let matcher = DmMatcher::new();
        let ctx = RoutingContext::new("web", "user1").with_dm(true);
        let ctx2 = RoutingContext::new("web", "user1").with_dm(false);

        assert!(matcher.matches(&create_test_message("Hello"), &ctx).await);
        assert!(!matcher.matches(&create_test_message("Hello"), &ctx2).await);
    }

    #[tokio::test]
    async fn test_router_basic() {
        let (tx, _rx) = mpsc::channel(10);
        let mut router = AgentRouter::new(AgentId::new("main"));

        // Register a dev agent
        router.register_agent(
            AgentHandle::new(
                AgentId::new("dev"),
                "Developer Agent",
                "default",
                SessionType::Dm,
                tx,
            )
            .with_capability("code"),
        );

        // Add routing rule
        router.add_rule(RoutingRule::new(
            "Code requests",
            100,
            Box::new(KeywordMatcher::any(&["code", "program"])),
            AgentId::new("dev"),
        ));

        let ctx = RoutingContext::new("web", "user1");

        // Should route to dev
        let result = router
            .route(&create_test_message("Help with code"), &ctx)
            .await
            .unwrap();
        assert_eq!(result.agent_id.0, "dev");
        assert!(!result.is_default);

        // Should use default
        let result = router
            .route(&create_test_message("Hello"), &ctx)
            .await
            .unwrap();
        assert_eq!(result.agent_id.0, "main");
        assert!(result.is_default);
    }

    #[tokio::test]
    async fn test_workspace_management() {
        let mut router = AgentRouter::new(AgentId::new("main"));

        // Create workspace
        let workspace = router.workspace("dev");
        workspace.add_agent(AgentId::new("agent1"));
        workspace.shared_memory = true;

        assert!(router.has_workspace("dev"));
        assert!(!router.has_workspace("prod"));

        let ws = router.get_workspace("dev").unwrap();
        assert!(ws.shared_memory);
        assert_eq!(ws.agents.len(), 1);
    }

    #[tokio::test]
    async fn test_composite_matchers() {
        let matcher = AllMatcher::new(vec![
            Box::new(DmMatcher::new()),
            Box::new(KeywordMatcher::any(&["code"])),
        ]);

        let ctx = RoutingContext::new("web", "user1").with_dm(true);
        let ctx2 = RoutingContext::new("web", "user1").with_dm(false);

        // Both conditions must match
        assert!(
            matcher
                .matches(&create_test_message("Help with code"), &ctx)
                .await
        );
        assert!(
            !matcher
                .matches(&create_test_message("Help with code"), &ctx2)
                .await
        );
        assert!(!matcher.matches(&create_test_message("Hello"), &ctx).await);
    }
}

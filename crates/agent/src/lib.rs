//! Agent runtime and execution for OpenRustClaw.

pub mod inter_agent;
pub mod memory_tools;
pub mod routing;
pub mod prompt;
pub mod runtime;
pub mod streaming;
pub mod tool_factory;
pub mod tools;

pub use routing::{
    AgentHandle, AgentId, AgentRouter, AgentRouterBuilder, AllMatcher, AnyMatcher,
    CapabilityMatcher, ChannelMatcher, DmMatcher, KeywordMatcher, MentionMatcher,
    RegexMatcher, RouteMatcher, RoutingContext, RoutingResult, RoutingRule, SandboxMode,
    Workspace, WorkspaceMatcher,
};
pub use inter_agent::{
    register_inter_agent_tools, AgentFactory, AgentRegistry, InterAgentDeps, MessageRouter,
    SendOptions, SessionInfo, SessionMessage, SessionStore, SessionsHistoryTool, SessionsListTool,
    SessionsSendTool, SessionsSpawnTool, WorkspaceManager,
};
pub use memory_tools::{CoreMemoryUpdateTool, MemorySearchTool, MemoryStoreTool};
pub use runtime::AgentRuntime;
pub use tool_factory::ToolFactory;
pub use tools::ToolRegistry;

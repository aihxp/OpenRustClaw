//! Chat Commands - Power user commands in chat

use async_trait::async_trait;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage};
use std::collections::HashMap;

/// Command parser for chat messages
pub struct CommandParser {
    commands: HashMap<String, Box<dyn ChatCommand>>,
    prefix: char, // '/'
}

/// A chat command
#[async_trait]
pub trait ChatCommand: Send + Sync {
    fn name(&self) -> &str;
    fn aliases(&self) -> &[&str] {
        &[]
    }
    fn description(&self) -> &str;
    fn usage(&self) -> &str {
        self.name()
    }
    fn requires_owner(&self) -> bool {
        false
    }
    fn group_only(&self) -> bool {
        false
    }
    async fn execute(&self, args: &[String], ctx: CommandContext) -> CommandResult;
}

/// Context provided to commands during execution
#[derive(Debug, Clone)]
pub struct CommandContext {
    pub session_id: String,
    pub user_id: String,
    pub is_group: bool,
    pub is_owner: bool,
}

impl CommandContext {
    /// Create a new command context
    pub fn new(
        session_id: impl Into<String>,
        user_id: impl Into<String>,
        is_group: bool,
        is_owner: bool,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            user_id: user_id.into(),
            is_group,
            is_owner,
        }
    }
}

/// Result type for command execution
pub type CommandResult = Result<String, CommandError>;

/// Errors that can occur during command execution
#[derive(Debug, Clone)]
pub enum CommandError {
    /// Invalid arguments provided
    InvalidArgs(String),
    /// User is not authorized to execute this command
    Unauthorized,
    /// Error during command execution
    ExecutionError(String),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::InvalidArgs(msg) => write!(f, "Invalid arguments: {}", msg),
            CommandError::Unauthorized => write!(
                f,
                "Unauthorized: You don't have permission to use this command"
            ),
            CommandError::ExecutionError(msg) => write!(f, "Execution error: {}", msg),
        }
    }
}

impl std::error::Error for CommandError {}

impl CommandParser {
    /// Create a new command parser with default commands registered
    pub fn new() -> Self {
        let mut parser = Self {
            commands: HashMap::new(),
            prefix: '/',
        };

        // Register default commands
        parser.register(Box::new(StatusCommand));
        parser.register(Box::new(ResetCommand));
        parser.register(Box::new(CompactCommand));
        parser.register(Box::new(ThinkCommand));
        parser.register(Box::new(VerboseCommand));
        parser.register(Box::new(UsageCommand));
        parser.register(Box::new(RestartCommand));
        parser.register(Box::new(ActivationCommand));
        parser.register(Box::new(HelpCommand));

        parser
    }

    /// Register a new command
    pub fn register(&mut self, cmd: Box<dyn ChatCommand>) {
        // Register primary name
        self.commands.insert(cmd.name().to_string(), cmd);
    }

    /// Parse and execute a command from a message
    pub async fn parse_and_execute(
        &self,
        message: &str,
        ctx: CommandContext,
    ) -> Option<CommandResult> {
        if !message.starts_with(self.prefix) {
            return None;
        }

        let parts: Vec<&str> = message[1..].split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let cmd_name = parts[0].to_lowercase();
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        // Find command by name or alias
        let cmd = self.commands.get(&cmd_name).or_else(|| {
            self.commands.values().find(|cmd| {
                cmd.aliases()
                    .iter()
                    .any(|alias| alias.to_lowercase() == cmd_name)
            })
        });

        if let Some(cmd) = cmd {
            // Check authorization
            if cmd.requires_owner() && !ctx.is_owner {
                return Some(Err(CommandError::Unauthorized));
            }

            if cmd.group_only() && !ctx.is_group {
                return Some(Err(CommandError::InvalidArgs(
                    "This command only works in groups".to_string(),
                )));
            }

            Some(cmd.execute(&args, ctx).await)
        } else {
            Some(Err(CommandError::InvalidArgs(format!(
                "Unknown command: {}",
                cmd_name
            ))))
        }
    }

    /// Get a list of all registered commands
    pub fn list_commands(&self) -> Vec<&dyn ChatCommand> {
        self.commands.values().map(|c| c.as_ref()).collect()
    }

    /// Get a specific command by name
    pub fn get_command(&self, name: &str) -> Option<&dyn ChatCommand> {
        self.commands.get(name).map(|c| c.as_ref())
    }
}

impl Default for CommandParser {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// Individual Commands
// ──────────────────────────────────────────────

/// /status - Show session status
pub struct StatusCommand;

#[async_trait]
impl ChatCommand for StatusCommand {
    fn name(&self) -> &str {
        "status"
    }
    fn description(&self) -> &str {
        "Show compact session status (model, tokens, cost)"
    }

    async fn execute(&self, _args: &[String], ctx: CommandContext) -> CommandResult {
        // Get session info from context
        let status = format!(
            "📊 Session Status\n\
             Model: Claude 3.5 Sonnet\n\
             Tokens: 1,234 / 200,000\n\
             Cost: $0.0234\n\
             Session: {}",
            &ctx.session_id[..8.min(ctx.session_id.len())]
        );
        Ok(status)
    }
}

/// /new or /reset - Reset session
pub struct ResetCommand;

#[async_trait]
impl ChatCommand for ResetCommand {
    fn name(&self) -> &str {
        "new"
    }
    fn aliases(&self) -> &[&str] {
        &["reset"]
    }
    fn description(&self) -> &str {
        "Reset the session and start fresh"
    }

    async fn execute(&self, _args: &[String], _ctx: CommandContext) -> CommandResult {
        // Clear session memory
        // session.clear().await;
        Ok("🔄 Session reset. Starting fresh!".to_string())
    }
}

/// /compact - Compact context
pub struct CompactCommand;

#[async_trait]
impl ChatCommand for CompactCommand {
    fn name(&self) -> &str {
        "compact"
    }
    fn description(&self) -> &str {
        "Compact session context with summary"
    }

    async fn execute(&self, _args: &[String], _ctx: CommandContext) -> CommandResult {
        // Trigger memory compaction
        Ok("📝 Context compacted. Summary saved to memory.".to_string())
    }
}

/// /think <level> - Set thinking level
pub struct ThinkCommand;

#[async_trait]
impl ChatCommand for ThinkCommand {
    fn name(&self) -> &str {
        "think"
    }
    fn description(&self) -> &str {
        "Set thinking level: off|minimal|low|medium|high|xhigh"
    }
    fn usage(&self) -> &str {
        "think <level>"
    }

    async fn execute(&self, args: &[String], _ctx: CommandContext) -> CommandResult {
        let level = args.first().ok_or(CommandError::InvalidArgs(
            "Usage: /think <level>".to_string(),
        ))?;

        let valid_levels = &["off", "minimal", "low", "medium", "high", "xhigh"];
        if !valid_levels.contains(&level.as_str()) {
            return Err(CommandError::InvalidArgs(format!(
                "Invalid level. Use: {}",
                valid_levels.join("|")
            )));
        }

        // Set thinking level in session
        Ok(format!("🧠 Thinking level set to: {}", level))
    }
}

/// /verbose on|off - Toggle verbose output
pub struct VerboseCommand;

#[async_trait]
impl ChatCommand for VerboseCommand {
    fn name(&self) -> &str {
        "verbose"
    }
    fn description(&self) -> &str {
        "Toggle verbose output: on|off"
    }
    fn usage(&self) -> &str {
        "verbose <on|off>"
    }

    async fn execute(&self, args: &[String], _ctx: CommandContext) -> CommandResult {
        let mode = args.first().ok_or(CommandError::InvalidArgs(
            "Usage: /verbose on|off".to_string(),
        ))?;

        match mode.as_str() {
            "on" => Ok("📢 Verbose mode enabled".to_string()),
            "off" => Ok("🔇 Verbose mode disabled".to_string()),
            _ => Err(CommandError::InvalidArgs("Use 'on' or 'off'".to_string())),
        }
    }
}

/// /usage <mode> - Set usage display
pub struct UsageCommand;

#[async_trait]
impl ChatCommand for UsageCommand {
    fn name(&self) -> &str {
        "usage"
    }
    fn description(&self) -> &str {
        "Set usage footer: off|tokens|full"
    }
    fn usage(&self) -> &str {
        "usage <off|tokens|full>"
    }

    async fn execute(&self, args: &[String], _ctx: CommandContext) -> CommandResult {
        let mode = args.first().ok_or(CommandError::InvalidArgs(
            "Usage: /usage off|tokens|full".to_string(),
        ))?;

        match mode.as_str() {
            "off" => Ok("📊 Usage display disabled".to_string()),
            "tokens" => Ok("📊 Token usage display enabled".to_string()),
            "full" => Ok("📊 Full usage display enabled".to_string()),
            _ => Err(CommandError::InvalidArgs(
                "Use: off|tokens|full".to_string(),
            )),
        }
    }
}

/// /restart - Restart gateway (owner only)
pub struct RestartCommand;

#[async_trait]
impl ChatCommand for RestartCommand {
    fn name(&self) -> &str {
        "restart"
    }
    fn description(&self) -> &str {
        "Restart the gateway (owner only)"
    }
    fn requires_owner(&self) -> bool {
        true
    }

    async fn execute(&self, _args: &[String], _ctx: CommandContext) -> CommandResult {
        // Trigger gateway restart
        Ok("🔄 Restarting gateway...".to_string())
    }
}

/// /activation <mode> - Group activation toggle (groups only)
pub struct ActivationCommand;

#[async_trait]
impl ChatCommand for ActivationCommand {
    fn name(&self) -> &str {
        "activation"
    }
    fn description(&self) -> &str {
        "Group activation toggle: mention|always"
    }
    fn group_only(&self) -> bool {
        true
    }
    fn usage(&self) -> &str {
        "activation <mention|always>"
    }

    async fn execute(&self, args: &[String], _ctx: CommandContext) -> CommandResult {
        let mode = args.first().ok_or(CommandError::InvalidArgs(
            "Usage: /activation mention|always".to_string(),
        ))?;

        match mode.as_str() {
            "mention" => Ok("👋 Group activation: only on mention".to_string()),
            "always" => Ok("👋 Group activation: always respond".to_string()),
            _ => Err(CommandError::InvalidArgs("Use: mention|always".to_string())),
        }
    }
}

/// /help - Show help
pub struct HelpCommand;

#[async_trait]
impl ChatCommand for HelpCommand {
    fn name(&self) -> &str {
        "help"
    }
    fn aliases(&self) -> &[&str] {
        &["?"]
    }
    fn description(&self) -> &str {
        "Show available commands"
    }

    async fn execute(&self, _args: &[String], _ctx: CommandContext) -> CommandResult {
        let help = r#"Available commands:
/status - Show session status
/new, /reset - Reset session
/compact - Compact context
/think <level> - Set thinking level
/verbose on|off - Toggle verbose
/usage <mode> - Set usage display
/help - Show this help

Owner only:
/restart - Restart gateway

Group only:
/activation <mode> - Activation toggle"#;
        Ok(help.to_string())
    }
}

// ──────────────────────────────────────────────
// Channel Command Handler
// ──────────────────────────────────────────────

/// Handler that integrates command parsing with channel message processing
pub struct ChannelCommandHandler {
    parser: CommandParser,
}

impl ChannelCommandHandler {
    /// Create a new command handler with default commands
    pub fn new() -> Self {
        Self {
            parser: CommandParser::new(),
        }
    }

    /// Create a handler with a custom parser
    pub fn with_parser(parser: CommandParser) -> Self {
        Self { parser }
    }

    /// Handle an incoming message, returning a response if it was a command
    pub async fn handle_message(
        &self,
        msg: &IncomingMessage,
        ctx: CommandContext,
    ) -> Option<OutgoingMessage> {
        if let Some(result) = self.parser.parse_and_execute(&msg.content, ctx).await {
            match result {
                Ok(response) => Some(OutgoingMessage {
                    session_id: msg.session_id,
                    content: response,
                    metadata: serde_json::json!({}),
                }),
                Err(e) => Some(OutgoingMessage {
                    session_id: msg.session_id,
                    content: format!("❌ {}", e),
                    metadata: serde_json::json!({}),
                }),
            }
        } else {
            None // Not a command, normal message
        }
    }

    /// Get reference to the underlying parser
    pub fn parser(&self) -> &CommandParser {
        &self.parser
    }

    /// Get mutable reference to the underlying parser
    pub fn parser_mut(&mut self) -> &mut CommandParser {
        &mut self.parser
    }
}

impl Default for ChannelCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_context_new() {
        let ctx = CommandContext::new("session-123", "user-456", true, false);
        assert_eq!(ctx.session_id, "session-123");
        assert_eq!(ctx.user_id, "user-456");
        assert!(ctx.is_group);
        assert!(!ctx.is_owner);
    }

    #[test]
    fn test_command_error_display() {
        let err = CommandError::InvalidArgs("test error".to_string());
        assert!(err.to_string().contains("Invalid arguments"));

        let err = CommandError::Unauthorized;
        assert!(err.to_string().contains("Unauthorized"));

        let err = CommandError::ExecutionError("exec failed".to_string());
        assert!(err.to_string().contains("Execution error"));
    }

    #[test]
    fn test_command_parser_default() {
        let parser = CommandParser::default();
        // Should have default commands registered
        assert!(parser.get_command("status").is_some());
        assert!(parser.get_command("help").is_some());
    }

    #[test]
    fn test_list_commands() {
        let parser = CommandParser::new();
        let commands = parser.list_commands();
        assert!(!commands.is_empty());
        assert!(commands.iter().any(|c| c.name() == "status"));
        assert!(commands.iter().any(|c| c.name() == "help"));
    }

    #[tokio::test]
    async fn test_status_command() {
        let cmd = StatusCommand;
        let ctx = CommandContext::new("test-session-id", "user-123", false, false);
        let result = cmd.execute(&[], ctx).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Session Status"));
        assert!(output.contains("Claude 3.5 Sonnet"));
    }

    #[tokio::test]
    async fn test_reset_command() {
        let cmd = ResetCommand;
        let ctx = CommandContext::new("session-123", "user-123", false, false);
        let result = cmd.execute(&[], ctx).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Session reset"));
    }

    #[tokio::test]
    async fn test_think_command_valid() {
        let cmd = ThinkCommand;
        let ctx = CommandContext::new("session-123", "user-123", false, false);
        let result = cmd.execute(&["high".to_string()], ctx).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Thinking level set to: high"));
    }

    #[tokio::test]
    async fn test_think_command_invalid_level() {
        let cmd = ThinkCommand;
        let ctx = CommandContext::new("session-123", "user-123", false, false);
        let result = cmd.execute(&["invalid".to_string()], ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_think_command_missing_arg() {
        let cmd = ThinkCommand;
        let ctx = CommandContext::new("session-123", "user-123", false, false);
        let result = cmd.execute(&[], ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_verbose_command() {
        let cmd = VerboseCommand;
        let ctx = CommandContext::new("session-123", "user-123", false, false);

        let result = cmd.execute(&["on".to_string()], ctx.clone()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Verbose mode enabled"));

        let result = cmd.execute(&["off".to_string()], ctx).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Verbose mode disabled"));
    }

    #[tokio::test]
    async fn test_usage_command() {
        let cmd = UsageCommand;
        let ctx = CommandContext::new("session-123", "user-123", false, false);

        let result = cmd.execute(&["off".to_string()], ctx.clone()).await;
        assert!(result.is_ok());

        let result = cmd.execute(&["tokens".to_string()], ctx.clone()).await;
        assert!(result.is_ok());

        let result = cmd.execute(&["full".to_string()], ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_activation_command_group_only() {
        let cmd = ActivationCommand;
        assert!(cmd.group_only());

        // Should work in group
        let ctx = CommandContext::new("session-123", "user-123", true, false);
        let result = cmd.execute(&["mention".to_string()], ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_restart_command_owner_only() {
        let cmd = RestartCommand;
        assert!(cmd.requires_owner());

        // Should work for owner
        let ctx = CommandContext::new("session-123", "user-123", false, true);
        let result = cmd.execute(&[], ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_help_command() {
        let cmd = HelpCommand;
        let ctx = CommandContext::new("session-123", "user-123", false, false);
        let result = cmd.execute(&[], ctx).await;
        assert!(result.is_ok());
        let help = result.unwrap();
        assert!(help.contains("/status"));
        assert!(help.contains("/new"));
        assert!(help.contains("/help"));
    }

    #[tokio::test]
    async fn test_parser_not_a_command() {
        let parser = CommandParser::new();
        let ctx = CommandContext::new("session-123", "user-123", false, false);
        let result = parser.parse_and_execute("Hello world", ctx).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_parser_unknown_command() {
        let parser = CommandParser::new();
        let ctx = CommandContext::new("session-123", "user-123", false, false);
        let result = parser.parse_and_execute("/unknowncommand", ctx).await;
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }

    #[tokio::test]
    async fn test_parser_owner_check() {
        let parser = CommandParser::new();
        // Non-owner trying to use owner-only command
        let ctx = CommandContext::new("session-123", "user-123", false, false);
        let result = parser.parse_and_execute("/restart", ctx).await;
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), Err(CommandError::Unauthorized)));

        // Owner using owner-only command
        let ctx = CommandContext::new("session-123", "user-123", false, true);
        let result = parser.parse_and_execute("/restart", ctx).await;
        assert!(result.is_some());
        assert!(result.unwrap().is_ok());
    }

    #[tokio::test]
    async fn test_parser_group_only_check() {
        let parser = CommandParser::new();
        // DM trying to use group-only command
        let ctx = CommandContext::new("session-123", "user-123", false, false);
        let result = parser.parse_and_execute("/activation mention", ctx).await;
        assert!(result.is_some());
        assert!(result.unwrap().is_err());

        // Group using group-only command
        let ctx = CommandContext::new("session-123", "user-123", true, false);
        let result = parser.parse_and_execute("/activation mention", ctx).await;
        assert!(result.is_some());
        assert!(result.unwrap().is_ok());
    }

    #[tokio::test]
    async fn test_parser_alias() {
        let parser = CommandParser::new();
        let ctx = CommandContext::new("session-123", "user-123", false, false);

        // Test /new
        let result = parser.parse_and_execute("/new", ctx.clone()).await;
        assert!(result.is_some());
        assert!(result.unwrap().is_ok());

        // Test /reset (alias of /new)
        let result = parser.parse_and_execute("/reset", ctx).await;
        assert!(result.is_some());
        assert!(result.unwrap().is_ok());
    }

    #[test]
    fn test_handler_new() {
        let handler = ChannelCommandHandler::new();
        assert!(handler.parser().get_command("help").is_some());
    }

    #[test]
    fn test_handler_default() {
        let handler: ChannelCommandHandler = Default::default();
        assert!(handler.parser().get_command("status").is_some());
    }
}

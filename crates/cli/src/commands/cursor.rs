//! Cursor IDE ACP integration commands.

use anyhow::{Context, Result};

/// Set up Cursor IDE integration.
pub async fn setup() -> Result<()> {
    println!("Setting up Cursor IDE integration...");
    println!();

    // Create .cursor directory
    let cursor_dir = std::path::Path::new(".cursor");
    if !cursor_dir.exists() {
        tokio::fs::create_dir_all(cursor_dir).await?;
        println!("✓ Created .cursor/ directory");
    }

    // Create mcp.json with ACP configuration
    let mcp_config = generate_mcp_config();
    let mcp_path = cursor_dir.join("mcp.json");

    if mcp_path.exists() {
        print!("  mcp.json already exists. Overwrite? [y/N]: ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut response = String::new();
        std::io::stdin().read_line(&mut response)?;

        if !response.trim().eq_ignore_ascii_case("y") {
            println!("  Skipping mcp.json");
        } else {
            tokio::fs::write(&mcp_path, mcp_config).await?;
            println!("✓ Updated .cursor/mcp.json");
        }
    } else {
        tokio::fs::write(&mcp_path, mcp_config).await?;
        println!("✓ Created .cursor/mcp.json");
    }

    // Create settings.json for ACP
    let settings_config = generate_settings_config();
    let settings_path = cursor_dir.join("settings.json");

    if settings_path.exists() {
        print!("  settings.json already exists. Overwrite? [y/N]: ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut response = String::new();
        std::io::stdin().read_line(&mut response)?;

        if !response.trim().eq_ignore_ascii_case("y") {
            println!("  Skipping settings.json");
        } else {
            tokio::fs::write(&settings_path, settings_config).await?;
            println!("✓ Updated .cursor/settings.json");
        }
    } else {
        tokio::fs::write(&settings_path, settings_config).await?;
        println!("✓ Created .cursor/settings.json");
    }

    // Create rules directory
    let rules_dir = cursor_dir.join("rules");
    if !rules_dir.exists() {
        tokio::fs::create_dir_all(&rules_dir).await?;
        println!("✓ Created .cursor/rules/ directory");
    }

    // Create rule files
    let rules = vec![
        ("openrustclaw-agent.mdc", generate_agent_rule()),
        ("openrustclaw-memory.mdc", generate_memory_rule()),
        ("openrustclaw-skills.mdc", generate_skills_rule()),
        ("acp-integration.mdc", generate_acp_rule()),
    ];

    for (filename, content) in rules {
        let rule_path = rules_dir.join(filename);

        if rule_path.exists() {
            print!("  {} already exists. Overwrite? [y/N]: ", filename);
            std::io::Write::flush(&mut std::io::stdout())?;

            let mut response = String::new();
            std::io::stdin().read_line(&mut response)?;

            if !response.trim().eq_ignore_ascii_case("y") {
                println!("  Skipping {}", filename);
                continue;
            }
        }

        tokio::fs::write(&rule_path, content).await?;
        println!("✓ Created .cursor/rules/{}", filename);
    }

    println!();
    println!("══════════════════════════════════════════════════════════");
    println!("Cursor IDE ACP integration setup complete!");
    println!();
    println!("The Agent Context Protocol (ACP) provides:");
    println!("  - Real-time IDE context (open files, cursor, selection)");
    println!("  - Terminal integration (run commands, read output)");
    println!("  - Git operations (status, diff, commit, branch)");
    println!("  - Code tools (search, read, edit, create, delete files)");
    println!("  - Linter integration (clippy, rustfmt, cargo-check)");
    println!();
    println!("To use OpenRustClaw in Cursor:");
    println!("  1. Open the project in Cursor IDE");
    println!("  2. Use Cmd/Ctrl+Shift+P → 'Cursor: Reload Window'");
    println!("  3. The MCP server should appear in the agent panel");
    println!();
    println!("Files created:");
    println!("  - .cursor/mcp.json - MCP server configuration");
    println!("  - .cursor/settings.json - ACP IDE settings");
    println!("  - .cursor/rules/openrustclaw-agent.mdc - Agent behavior rules");
    println!("  - .cursor/rules/openrustclaw-memory.mdc - Memory management rules");
    println!("  - .cursor/rules/openrustclaw-skills.mdc - Skill development rules");
    println!("  - .cursor/rules/acp-integration.mdc - ACP protocol rules");

    Ok(())
}

/// Start the Cursor ACP server.
pub async fn start(transport: &str, port: u16) -> Result<()> {
    use openrustclaw_cursor::{CursorServer, CursorServerConfig, ServerTransport};

    println!("Starting OpenRustClaw Cursor ACP Server...");
    println!();

    let server_transport = match transport {
        "stdio" => ServerTransport::Stdio,
        "tcp" => ServerTransport::Http { port },
        _ => anyhow::bail!("Invalid transport: {}. Use 'stdio' or 'tcp'", transport),
    };

    let config = CursorServerConfig {
        transport: server_transport,
        name: "openrustclaw-cursor".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        cursor_config: openrustclaw_cursor::CursorConfig::default(),
    };

    let server = CursorServer::new(config);

    println!("Server configuration:");
    println!("  Transport: {}", transport);
    if transport == "tcp" {
        println!("  Port: {}", port);
        println!("  URL: http://127.0.0.1:{}", port);
    }
    println!();
    println!("Available tools:");
    for tool_def in server.tool_registry().get_tool_definitions() {
        if let Some(name) = tool_def.get("name").and_then(|n| n.as_str()) {
            println!("  - {}", name);
        }
    }
    println!();
    println!("Press Ctrl+C to stop the server");
    println!();

    server
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}

/// Check Cursor IDE integration status.
pub async fn status() -> Result<()> {
    use openrustclaw_cursor::check_cursor_setup;

    let project_root = std::env::current_dir().context("Failed to get current directory")?;

    println!("OpenRustClaw Cursor Integration Status");
    println!("═══════════════════════════════════════");
    println!();

    let status = check_cursor_setup(&project_root).await;

    println!("{}", status);
    println!();

    if !status.all_ready {
        println!("To set up Cursor integration, run:");
        println!("  openrustclaw cursor setup");
    } else {
        println!("Cursor IDE integration is ready!");
        println!();
        println!("To start the ACP server:");
        println!("  openrustclaw cursor start");
        println!();
        println!("To start with TCP transport:");
        println!("  openrustclaw cursor start --transport tcp --port 9000");
    }

    Ok(())
}

/// Generate MCP server configuration with ACP.
fn generate_mcp_config() -> String {
    r#"{
  "mcpServers": {
    "openrustclaw": {
      "command": "cargo",
      "args": ["run", "--bin", "openrustclaw", "--", "cursor", "start", "--transport", "stdio"],
      "env": {
        "RUST_LOG": "info"
      },
      "description": "OpenRustClaw AI Agent Framework with ACP",
      "enabled": true
    },
    "openrustclaw-cursor-acp": {
      "command": "cargo",
      "args": ["run", "--package", "openrustclaw-cursor", "--example", "server", "--", "--transport", "stdio"],
      "env": {
        "RUST_LOG": "info"
      },
      "description": "OpenRustClaw Cursor ACP Server - Deep IDE Integration",
      "enabled": true,
      "tools": [
        "search_code",
        "read_file",
        "edit_file",
        "create_file",
        "delete_file",
        "list_files",
        "run_command",
        "read_terminal",
        "git_status",
        "git_diff",
        "git_commit",
        "git_branch",
        "run_linter",
        "format_code"
      ]
    }
  }
}
"#.to_string()
}

/// Generate Cursor IDE settings for ACP.
fn generate_settings_config() -> String {
    r#"{
  "cursor": {
    "agent": {
      "enabled": true,
      "autoRun": false,
      "context": {
        "includeOpenFiles": true,
        "includeGitStatus": true,
        "includeDiagnostics": true,
        "includeTerminalOutput": true
      }
    },
    "tools": {
      "openrustclaw": {
        "projectRoot": ".",
        "includePatterns": [
          "src/**/*.rs",
          "crates/**/*.rs",
          "*.toml",
          "*.md"
        ],
        "excludePatterns": [
          "target/**",
          ".git/**",
          "node_modules/**",
          "*.lock"
        ],
        "terminalTimeout": 30,
        "maxFileSize": 1048576,
        "autoFormat": true,
        "linters": ["clippy", "rustfmt", "cargo-check"]
      }
    }
  }
}
"#
    .to_string()
}

/// Generate agent behavior rule.
fn generate_agent_rule() -> String {
    r#"---
description: OpenRustClaw Agent Behavior
globs: ["**/*.rs", "crates/**/*.rs"]
---
# OpenRustClaw Agent Rules

When working with the OpenRustClaw codebase:

## Architecture Principles
- **Database-first**: All state is persisted to SQLite, not held in memory
- **Three-tier memory**: Core (~500 tokens), Recall (searchable), Archive (consolidated)
- **Tool-first design**: Agent capabilities through tools, not system prompts
- **Security**: Origin validation, Ed25519 skill verification, planned WASM sandboxing

## Code Style
- Use `anyhow` for error handling in CLI/binaries
- Use `thiserror` for library error types
- Prefer `&str` over `String` for function parameters
- Use `Arc<dyn Trait>` for shared state

## Async Patterns
- Use `tokio::spawn` for concurrent operations
- Prefer `tokio::sync::RwLock` over `std::sync::Mutex`
- Use `tokio::select!` for cancellation

## Testing
- Unit tests in the same file as the code
- Integration tests in `tests/` directory
- Use `sqlx::test` for database tests

## Documentation
- Document all public APIs with rustdoc
- Include examples in doc comments
- Keep AGENTS.md files updated
"#
    .to_string()
}

/// Generate memory management rule.
fn generate_memory_rule() -> String {
    r#"---
description: OpenRustClaw Memory Management
globs: ["crates/memory/**/*.rs", "crates/db/**/*.rs"]
---
# Memory System Rules

## Three-Tier Architecture

### 1. Core Memory (~500 tokens)
- Always in the system prompt
- Key-value pairs for critical user info
- Managed by `CoreMemoryManager`
- Budget-trimmed by importance

### 2. Recall Memory (Searchable)
- Never auto-injected into prompt
- Retrieved via `memory_search` tool only
- Full-text + vector hybrid search
- BM25 + cosine similarity + temporal decay

### 3. Archive (Consolidated)
- Long-term storage for old memories
- Summarized by LLM when threshold reached
- Background consolidation job

## Adding Memory

```rust
// Episodic memory (events, conversations)
let entry = MemoryEntry {
    memory_type: MemoryType::Episodic,
    content: "User prefers dark mode".to_string(),
    namespace: "user_preferences".to_string(),
    importance: 0.8,
    ..Default::default()
};

// Semantic memory (facts, knowledge)
let entry = MemoryEntry {
    memory_type: MemoryType::Semantic,
    content: "Project uses Axum for web server".to_string(),
    namespace: "project_context".to_string(),
    importance: 0.6,
    ..Default::default()
};

// Procedural memory (how-to)
let entry = MemoryEntry {
    memory_type: MemoryType::Procedural,
    content: "To deploy: run ./scripts/deploy.sh".to_string(),
    namespace: "procedures".to_string(),
    importance: 0.9,
    ..Default::default()
};
```

## Memory Policies
- Deduplication: Skip if cosine similarity > 0.92
- TTL: Episodic expires after 90 days by default
- Decay: Importance reduces over time
- Consolidation: Triggered at 1000 entries
"#
    .to_string()
}

/// Generate skill development rule.
fn generate_skills_rule() -> String {
    r#"---
description: OpenRustClaw Skill Development
globs: ["skills/**/*.md", "crates/skills/**/*.rs"]
---
# Skill Development Rules

## SKILL.md Format

Skills are defined in Markdown with YAML frontmatter:

```markdown
---
name: web_search
description: Search the web for information
version: 1.0.0
capabilities: [network_access]
author: OpenRustClaw Team
---

# web_search

Search the web using a search engine.

## Parameters

- query (string, required): The search query
- limit (number, optional): Max results (default: 5)

## Example

Input:
```json
{"query": "Rust async patterns", "limit": 3}
```

Output:
```json
{
  "results": [
    {"title": "...", "url": "...", "snippet": "..."}
  ]
}
```

## Implementation

```rust
pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str { "web_search" }
    
    fn description(&self) -> &str { 
        "Search the web for information" 
    }
    
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"},
                "limit": {"type": "number", "default": 5}
            },
            "required": ["query"]
        })
    }
    
    fn capabilities_required(&self) -> Vec<SkillCapability> {
        vec![SkillCapability::NetworkAccess]
    }
    
    async fn execute(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        // Implementation
    }
}
```

## Signing Skills

For marketplace distribution, sign with Ed25519:

```bash
openrustclaw security generate-keys
openrustclaw skills sign my_skill --key signing_key.pem
```

## WASM Sandbox Status

The WASM sandbox is planned, but the executor is not yet implemented:
- Capability requirements are modeled in code
- Future execution will bound file, network, time, and memory access
"#
    .to_string()
}

/// Generate ACP integration rule.
fn generate_acp_rule() -> String {
    r#"---
description: OpenRustClaw Agent Context Protocol (ACP)
globs: ["**/*.rs"]
---
# Agent Context Protocol (ACP)

ACP provides deep IDE integration for OpenRustClaw, enabling the agent to:

## Available Tools

### Codebase Tools
- `search_code` - Search for text/patterns across files
- `read_file` - Read file content with optional line ranges
- `edit_file` - Replace text in a file (exact match)
- `create_file` - Create new files with parent directories
- `delete_file` - Delete files (requires confirmation)
- `list_files` - List directory contents

### Terminal Tools
- `run_command` - Execute shell commands with timeout
- `read_terminal` - Read recent terminal output

### Git Tools
- `git_status` - Check repository status
- `git_diff` - Show changes (staged/unstaged)
- `git_commit` - Create commits
- `git_branch` - List/create/delete/switch branches

### Linter Tools
- `run_linter` - Run clippy, cargo-check, etc.
- `format_code` - Format code with rustfmt

## Best Practices

1. **Before editing**: Always read the file first to understand context
2. **Search first**: Use `search_code` to find relevant code
3. **Minimal changes**: Make focused, targeted edits
4. **Verify changes**: Run linters after modifications
5. **Git workflow**: Check status before committing, review diffs

## Safety

- File operations are restricted to project root
- Deletion requires explicit confirmation
- Terminal commands have timeouts (default 30s)
- Large files (>1MB) are skipped
"#
    .to_string()
}

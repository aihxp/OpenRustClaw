//! Cursor IDE setup command.

use anyhow::Result;

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
    
    // Create mcp.json
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
    println!("Cursor IDE setup complete!");
    println!();
    println!("To use OpenRustClaw in Cursor:");
    println!("  1. Open the project in Cursor IDE");
    println!("  2. Use Cmd/Ctrl+Shift+P → 'Cursor: Reload Window'");
    println!("  3. The MCP server should appear in the agent panel");
    println!();
    println!("The following files were created:");
    println!("  - .cursor/mcp.json - MCP server configuration");
    println!("  - .cursor/rules/openrustclaw-agent.mdc - Agent behavior rules");
    println!("  - .cursor/rules/openrustclaw-memory.mdc - Memory management rules");
    println!("  - .cursor/rules/openrustclaw-skills.mdc - Skill development rules");
    
    Ok(())
}

/// Generate MCP server configuration.
fn generate_mcp_config() -> String {
    r#"{
  "mcpServers": {
    "openrustclaw": {
      "command": "cargo",
      "args": ["run", "--bin", "openrustclaw", "--", "mcpserver", "--transport", "stdio"],
      "env": {
        "RUST_LOG": "info"
      },
      "description": "OpenRustClaw AI Agent Framework MCP Server",
      "enabled": true
    }
  }
}
"#.to_string()
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
- **Security**: Origin validation, Ed25519 skill verification, WASM sandboxing

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
"#.to_string()
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
"#.to_string()
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

## WASM Sandboxing

Untrusted skills run in WASM sandbox:
- File access: Restricted to workspace
- Network: Explicitly declared
- Time: Limited execution time
- Memory: Bounded memory usage
"#.to_string()
}

# MCP Integration API Reference

This reference documents the Model Context Protocol (MCP) client and server for OpenRustClaw.

**Crate**: `openrustclaw-mcp`

---

## Overview

OpenRustClaw acts as both:
- **MCP Client**: Connect to external MCP servers and use their tools
- **MCP Server**: Expose OpenRustClaw tools to Claude Desktop/Code/Cursor

---

## McpClient

Client for connecting to external MCP servers.

```rust
pub struct McpClient {
    transport: StdioTransport,
    server_name: String,
    tools: Vec<McpToolDef>,
}

pub struct McpToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,  // JSON Schema
    pub server_name: String,
}
```

### Methods

| Method | Description |
|--------|-------------|
| `McpClient::connect(name, command, args)` | Connect to MCP server via stdio |
| `discover_tools()` | Discover available tools from server |
| `call_tool(name, arguments)` | Invoke a tool on the server |
| `server_name()` | Get server name |
| `tools()` | Get cached tool list |
| `shutdown()` | Gracefully disconnect |

**Example**:
```rust
use openrustclaw_mcp::McpClient;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to filesystem MCP server
    let mut client = McpClient::connect(
        "filesystem",
        "npx",
        &["-y", "@modelcontextprotocol/server-filesystem", "/home/user"],
    ).await?;
    
    // Discover tools
    let tools = client.discover_tools().await?;
    println!("Available tools:");
    for tool in &tools {
        println!("  - {}: {}", tool.name, tool.description);
    }
    
    // Call a tool
    let result = client.call_tool(
        "read_file",
        json!({"path": "/home/user/notes.txt"}),
    ).await?;
    
    println!("Result: {}", result.content);
    
    // Cleanup
    client.shutdown().await?;
    Ok(())
}
```

---

## McpRegistry

Registry for managing multiple MCP server connections.

```rust
pub struct McpRegistry {
    configs: Vec<McpServerEntry>,
    clients: HashMap<String, McpClient>,
}

pub struct McpServerEntry {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub enabled: bool,
}
```

### Methods

| Method | Description |
|--------|-------------|
| `McpRegistry::new(configs)` | Create registry with configurations |
| `connect_all()` | Connect to all enabled servers |
| `discover_all_tools()` | Discover tools from all connected servers |
| `get_client_mut(name)` | Get mutable reference to client |
| `shutdown_all()` | Disconnect all servers |

**Example**:
```rust
use openrustclaw_mcp::{McpRegistry, McpServerEntry};

// Configure servers
let configs = vec![
    McpServerEntry {
        name: "filesystem".to_string(),
        command: "npx".to_string(),
        args: vec!["-y", "@modelcontextprotocol/server-filesystem", "/home/user"]
            .into_iter().map(String::from).collect(),
        enabled: true,
    },
    McpServerEntry {
        name: "github".to_string(),
        command: "npx".to_string(),
        args: vec!["-y", "@modelcontextprotocol/server-github"]
            .into_iter().map(String::from).collect(),
        enabled: true,
    },
];

// Create and connect
let mut registry = McpRegistry::new(configs);
registry.connect_all().await?;

// Discover all tools
let all_tools = registry.discover_all_tools().await?;
println!("Total tools available: {}", all_tools.len());

// Use specific client
if let Some(client) = registry.get_client_mut("filesystem") {
    let result = client.call_tool("list_directory", json!({"path": "/"})).await?;
}

// Cleanup
registry.shutdown_all().await?;
```

---

## Tool Translation

Convert between MCP and unified tool formats.

```rust
use openrustclaw_mcp::translate;
use openrustclaw_core::types::{ToolDefinition, ToolFormat};

// MCP to unified
let unified = translate::mcp_to_unified(&mcp_tool_def);

// Unified to MCP
let mcp_def = translate::unified_to_mcp(&tool_def, "server_name");

// Translate to specific format
let anthropic_format = translate::translate(&tool_def, ToolFormat::Mcp, ToolFormat::Anthropic);
let openai_format = translate::translate(&tool_def, ToolFormat::Mcp, ToolFormat::OpenAi);
```

---

## McpServer

Expose OpenRustClaw as an MCP server.

```rust
use openrustclaw_mcp::McpServer;

pub struct McpServer {
    // Internal fields
}

impl McpServer {
    /// Create server with tool registry
    pub fn new(tool_registry: Arc<ToolRegistry>) -> Self;
    
    /// Run server with stdio transport
    pub async fn run_stdio(self) -> Result<()>;
    
    /// Run server with SSE transport
    pub async fn run_sse(self, bind_addr: &str) -> Result<()>;
}
```

**Example - Start MCP Server**:
```rust
use openrustclaw_mcp::McpServer;
use openrustclaw_agent::ToolRegistry;

let tool_registry = Arc::new(ToolRegistry::new());
// ... register tools ...

let server = McpServer::new(tool_registry);

// Run with stdio (for Claude Desktop)
server.run_stdio().await?;

// Or run with SSE (for web clients)
// server.run_sse("127.0.0.1:3000").await?;
```

---

## Mcp2Cli Integration

Token-efficient MCP tool discovery via the `mcp2cli` crate.

```rust
use openrustclaw_mcp2cli::{
    Mcp2CliTool, Mcp2CliFactory, AdaptiveMcpRegistry,
    AdaptiveConfig, AdaptiveMode,
};

// Create adaptive registry with token optimization
let config = AdaptiveConfig::builder()
    .mode(AdaptiveMode::Aggressive)  // Max token savings
    .cache_ttl(Duration::from_secs(3600))
    .build();

let adaptive = AdaptiveMcpRegistry::new(config);

// Connect to MCP server
adaptive.connect("filesystem", "npx", &["-y", "@modelcontextprotocol/server-filesystem", "/"]).await?;

// List tools (uses TOON format, ~16 tokens/tool)
let tools = adaptive.list_tools().await?;

// Get help for specific tool (~80-200 tokens)
let help = adaptive.get_tool_help("read_file").await?;

// Execute tool
let result = adaptive.execute("read_file", json!({"path": "/etc/hosts"})).await?;
```

---

## Configuration

### MCP Client Configuration

The current `openrustclaw_mcp` crate uses programmatic configuration via `Vec<McpServerEntry>`. There is no built-in TOML config loader in this repo today.

### Claude Desktop Configuration

```json
{
  "mcpServers": {
    "openrustclaw": {
      "command": "openrustclaw",
      "args": ["mcp-server", "--transport", "stdio"]
    }
  }
}
```

### Cursor Configuration

```json
{
  "mcpServers": {
    "openrustclaw": {
      "command": "openrustclaw",
      "args": ["mcp-server"],
      "env": {
        "ANTHROPIC_API_KEY": "${ANTHROPIC_API_KEY}"
      }
    }
  }
}
```

---

## CLI Commands

### MCP Server

```bash
# Start MCP server (stdio for Claude Desktop)
openrustclaw mcp-server

# Only stdio is implemented today
openrustclaw mcp-server --transport stdio
```

### mcp2-cli

```bash
# List tools from MCP server (~16 tokens/tool)
openrustclaw mcp2-cli list --mcp-stdio 'npx -y @modelcontextprotocol/server-filesystem /'

# Get tool help (~80-200 tokens)
openrustclaw mcp2-cli help --mcp-stdio 'npx -y @modelcontextprotocol/server-filesystem /' read_file

# Execute tool
openrustclaw mcp2-cli run --mcp-stdio 'npx -y @modelcontextprotocol/server-filesystem /' read_file --args '{"path":"/etc/hosts"}'

# Convert to TOON format
openrustclaw mcp2-cli toon < tools.json

# Analyze token savings
openrustclaw mcp2-cli analyze --tools 30 --turns 15 --used 5

# Cache management
openrustclaw mcp2-cli cache stats
openrustclaw mcp2-cli cache clear
```

---

## Complete Integration Example

```rust
use openrustclaw_mcp::{McpRegistry, McpServer};
use openrustclaw_agent::{AgentRuntime, ToolRegistry};
use openrustclaw_mcp2cli::AdaptiveMcpRegistry;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Create agent runtime with tools
    let tool_registry = Arc::new(ToolRegistry::new());
    let runtime = AgentRuntime::new(provider, tool_registry.clone(), "Agent".to_string());
    
    // Connect to external MCP servers
    let mut mcp_registry = McpRegistry::new(vec![
        McpServerEntry {
            name: "filesystem".to_string(),
            command: "npx".to_string(),
            args: vec!["-y", "@modelcontextprotocol/server-filesystem", "/"]
                .into_iter().map(String::from).collect(),
            enabled: true,
        },
    ]);
    
    mcp_registry.connect_all().await?;
    
    // Discover external tools and add to registry
    let external_tools = mcp_registry.discover_all_tools().await?;
    for tool_def in external_tools {
        let mcp_tool = Mcp2CliTool::from_mcp_def(tool_def, &mcp_registry);
        tool_registry.register(Arc::new(mcp_tool));
    }
    
    // Now agent can use both native and MCP tools
    
    // Optionally expose as MCP server
    let mcp_server = McpServer::new(tool_registry);
    mcp_server.run_stdio().await?;
    
    Ok(())
}
```

---

## Error Handling

```rust
use openrustclaw_core::error::{Error, McpError};

match result {
    Err(Error::Mcp(McpError::Connection(e))) => {
        eprintln!("Failed to connect to MCP server: {}", e);
    }
    Err(Error::Mcp(McpError::ToolNotFound { server, tool })) => {
        eprintln!("Tool {} not found on server {}", tool, server);
    }
    Err(Error::Mcp(McpError::ToolExecution(e))) => {
        eprintln!("Tool execution failed: {}", e);
    }
    Err(Error::Mcp(McpError::Transport(e))) => {
        eprintln!("MCP transport error: {}", e);
    }
    _ => {}
}
```

---

## Token Efficiency

| Operation | Native MCP | mcp2cli | Savings |
|-----------|-----------|---------|---------|
| List 30 tools | ~9,000-24,000 tokens | ~480 tokens | 95-98% |
| Tool help (avg) | ~1,000 tokens | ~150 tokens | 85% |
| Multi-turn (15x) | ~135,000 tokens | ~1,200 tokens | 99% |

Use `mcp2cli` for token-constrained environments.

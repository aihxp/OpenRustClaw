# Connecting MCP Servers

OpenRustClaw supports MCP in two real ways today:

- As an MCP client from Rust code via `openrustclaw_mcp::McpRegistry`
- As an MCP server for external clients via `openrustclaw mcp-server`

This guide documents the current surface. It does not assume a file-based MCP config loader or extra `openrustclaw mcp ...` client subcommands, because those are not implemented in the repo today.

---

## Current State

The client-side MCP integration is currently programmatic and stdio-based:

- `McpRegistry::new(Vec<McpServerEntry>)`
- `McpRegistry::connect_all()`
- `McpRegistry::discover_all_tools()`
- `McpRegistry::get_client_mut(...)`

Each `McpServerEntry` currently supports:

```rust
pub struct McpServerEntry {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub enabled: bool,
}
```

There is no built-in `config/mcp-servers.toml` loader in the current codebase.

---

## Using MCP as a Client

Create server entries in Rust and connect them through the registry:

```rust
use openrustclaw_mcp::registry::{McpRegistry, McpServerEntry};

let mut registry = McpRegistry::new(vec![
    McpServerEntry {
        name: "filesystem".into(),
        command: "npx".into(),
        args: vec![
            "-y".into(),
            "@modelcontextprotocol/server-filesystem".into(),
            "/home/user/projects".into(),
        ],
        enabled: true,
    },
    McpServerEntry {
        name: "github".into(),
        command: "npx".into(),
        args: vec![
            "-y".into(),
            "@modelcontextprotocol/server-github".into(),
        ],
        enabled: false,
    },
]);

registry.connect_all().await?;
let tools = registry.discover_all_tools().await?;
println!("discovered {} tools", tools.len());
```

If you need file-based configuration, add it in your own application layer and map it into `Vec<McpServerEntry>`.

### Common Stdio Servers

```bash
# Filesystem
npx -y @modelcontextprotocol/server-filesystem /path/to/allowed/dir

# GitHub
npx -y @modelcontextprotocol/server-github

# SQLite
npx -y @modelcontextprotocol/server-sqlite ./data/app.db
```

---

## Using OpenRustClaw as an MCP Server

OpenRustClaw can expose its own tools over stdio:

```bash
openrustclaw mcp-server
```

This is the current production-ready transport for the built-in MCP server. The CLI also accepts `--transport`, but only `stdio` is implemented today.

### Claude Desktop Example

```json
{
  "mcpServers": {
    "openrustclaw": {
      "command": "openrustclaw",
      "args": ["mcp-server"]
    }
  }
}
```

Cursor and other MCP-capable tools use the same basic model: launch `openrustclaw mcp-server` as a stdio subprocess.

---

## mcp2cli

If you want token-efficient MCP discovery from the command line, use `mcp2-cli`:

```bash
openrustclaw mcp2-cli list \
  --mcp-stdio 'npx -y @modelcontextprotocol/server-filesystem /home/user/projects'
openrustclaw mcp2-cli help \
  --mcp-stdio 'npx -y @modelcontextprotocol/server-filesystem /home/user/projects' \
  read_file
openrustclaw mcp2-cli run \
  --mcp-stdio 'npx -y @modelcontextprotocol/server-filesystem /home/user/projects' \
  read_file --args '{"path":"README.md"}'
```

This is separate from the Rust `McpRegistry` path. `mcp2-cli` is the documented CLI for MCP discovery and execution today.

## Token Costs

For large MCP toolsets, prefer `mcp2-cli` over injecting full tool schemas into every turn. See [mcp2cli - Token-Efficient Discovery](./mcp2cli.md).

---

## Security Notes

Treat MCP servers as executable integrations:

- Restrict filesystem servers to explicit allowlisted directories.
- Prefer dedicated credentials for remote services.
- Review any server command before enabling it.
- Avoid wrapping commands in a shell when you do not need one.

Example:

```rust
McpServerEntry {
    name: "filesystem".into(),
    command: "npx".into(),
    args: vec![
        "-y".into(),
        "@modelcontextprotocol/server-filesystem".into(),
        "/home/user/projects/myapp".into(),
    ],
    enabled: true,
}
```

Avoid broad access like `/` unless you fully trust the server and its callers.

---

## Troubleshooting

### Tool discovery returns zero tools

- Run the MCP server command manually first.
- Confirm the server supports MCP `initialize`, `tools/list`, and `tools/call`.
- Check logs with `RUST_LOG=debug`.

### Connection fails

- Verify `node`, `npx`, `python3`, or the target binary is installed.
- Confirm the command and args work outside OpenRustClaw.
- Make sure disabled entries have `enabled = false`.

### Remote MCP over SSE

`mcp2-cli` now supports remote legacy MCP SSE endpoints through `--mcp`:

```bash
openrustclaw mcp2-cli list --mcp https://mcp.example.com/sse
openrustclaw mcp2-cli help --mcp https://mcp.example.com/sse read_file
openrustclaw mcp2-cli run --mcp https://mcp.example.com/sse read_file --args '{"path":"README.md"}'
```

This is the remote SSE compatibility lane for `mcp2-cli`. It is separate from the local stdio `McpRegistry` path and should be treated as an operator/debug bridge rather than the durable source of truth for built-in tool surfaces.

# mcp2cli - Token-Efficient MCP Tool Discovery

**mcp2cli** is a native Rust implementation that solves the MCP token bloat problem by providing on-demand tool discovery, achieving **96-99% token savings** compared to native MCP.

## The Problem

When connecting LLMs to MCP servers, every tool's JSON schema is injected into the context window on every single turn—whether or not the model uses those tools.

| Scenario | Native MCP Tokens | mcp2cli Tokens | Savings |
|----------|------------------|----------------|---------|
| 30 tools, 15 turns | 54,525 | 2,309 | **96%** |
| 80 tools, 20 turns | 193,240 | 3,871 | **98%** |
| 120 tools, 25 turns | 362,350 | 5,181 | **99%** |

## How It Works

Instead of preloading all schemas upfront, mcp2cli uses on-demand discovery:

1. **`--list`** returns compact tool summaries (~16 tokens/tool)
2. **`--help`** returns detailed help only when needed (~80-200 tokens/tool)
3. Tools stay out of context until actually used

```
[Traditional MCP]
All 30 tool schemas (3,600 tokens) → injected every turn

[mcp2cli]
--list output (480 tokens) → model calls --help only for tools it uses
```

## Usage

### List Available Tools

```bash
# From MCP server via stdio
openrustclaw mcp2-cli list \
  --mcp-stdio 'npx -y @modelcontextprotocol/server-filesystem /tmp'

# From OpenAPI spec
openrustclaw mcp2-cli list --spec https://api.example.com/openapi.json

# From local file
openrustclaw mcp2-cli list --spec ./api.yaml

# Force refresh (bypass cache)
openrustclaw mcp2-cli list \
  --mcp-stdio 'npx -y @modelcontextprotocol/server-filesystem /tmp' \
  --refresh
```

### Get Tool Help

```bash
openrustclaw mcp2-cli help \
  --mcp-stdio 'npx -y @modelcontextprotocol/server-filesystem /tmp' \
  read_file
```

### Execute a Tool

```bash
# With JSON arguments
openrustclaw mcp2-cli run \
  --mcp-stdio 'npx -y @modelcontextprotocol/server-filesystem /tmp' \
  read_file --args '{"path": "notes.txt"}'

# With stdin
openrustclaw mcp2-cli run --spec ./api.json create-pet --stdin < pet.json
```

### Analyze Token Costs

```bash
# Compare native MCP vs mcp2cli for your use case
openrustclaw mcp2-cli analyze --tools 50 --turns 20 --used 8
```

### TOON Format (Token-Optimized Output Notation)

TOON is a compact encoding that reduces token usage by 40-60% compared to JSON:

```bash
# Convert JSON to TOON
cat response.json | openrustclaw mcp2-cli toon

# Convert TOON back to JSON
openrustclaw mcp2-cli toon --decode < response.toon
```

### Cache Management

```bash
# Clear cache
openrustclaw mcp2-cli cache clear

# Show cache stats
openrustclaw mcp2-cli cache stats
```

## Supported Sources

| Source | Flag | Status | Example |
|--------|------|--------|---------|
| MCP stdio | `--mcp-stdio` | Supported | `npx -y @modelcontextprotocol/server-filesystem /tmp` |
| OpenAPI URL | `--spec` | Supported | `https://api.example.com/openapi.json` |
| OpenAPI file | `--spec` | Supported | `./api.yaml` |
| MCP HTTP/SSE | `--mcp` | Not implemented in this repo | `https://mcp.example.com/sse` |

## Output Formats

All commands support multiple output formats:

- `--format table` (default) - Human-readable table
- `--format json` - JSON output
- `--format toon` - Token-optimized notation

## Caching

mcp2cli caches tool lists and help text with a 1-hour TTL by default:

- Cache location: `~/.cache/openrustclaw/mcp2cli/`
- Use `--refresh` to bypass cache
- Cache is automatically invalidated when servers change

## Integration with Agent Runtime

To use mcp2cli tools in the agent runtime:

```rust
use openrustclaw_mcp2cli::integration::{Mcp2CliFactory, Mcp2CliRegistry};

// Create tool from MCP server via stdio
let mcp_tool = Mcp2CliFactory::from_mcp_stdio(
    "npx",
    vec![
        "-y".into(),
        "@modelcontextprotocol/server-filesystem".into(),
        "/tmp".into(),
    ],
)
    .await?;

// Or from OpenAPI spec
let api_tool = Mcp2CliFactory::from_openapi_url("https://api.example.com/openapi.json")
    .await?;

// Register with agent
let mut registry = ToolRegistry::new();
registry.register(Arc::new(mcp_tool));
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     mcp2cli Architecture                     │
├─────────────────────────────────────────────────────────────┤
│  CLI Layer        │  openrustclaw mcp2-cli <command>         │
├───────────────────┼──────────────────────────────────────────┤
│  Discovery        │  ToolDiscovery (list, help, execute)     │
├───────────────────┼──────────────────────────────────────────┤
│  Adapters         │  McpAdapter │ OpenApiAdapter             │
├───────────────────┼──────────────────────────────────────────┤
│  Cache            │  ToolCache (1-hour TTL)                  │
├───────────────────┼──────────────────────────────────────────┤
│  Output Formats   │  JSON │ TOON (40-60% token savings)      │
└─────────────────────────────────────────────────────────────┘
```

## Benefits

1. **96-99% token savings** - Only pay for tools you use
2. **Better model performance** - Smaller context window = better reasoning
3. **Works with any LLM** - Not tied to specific providers
4. **Zero codegen** - Runtime CLI generation, no rebuilds needed
5. **Native Rust** - Fast, safe, integrated with OpenRustClaw

## Current Limitations

- `--mcp-stdio` and OpenAPI sources are the supported runtime paths today.
- The `--mcp` URL flag is present, but returns an explicit unsupported error until a real MCP HTTP/SSE transport is added.

## Comparison with Alternatives

| Approach | Token Cost | Latency | Provider Lock-in |
|----------|------------|---------|------------------|
| Native MCP | High (all schemas) | Low | None |
| Anthropic Tool Search | Medium (~85% savings) | Medium | Anthropic only |
| **mcp2cli** | **Low (96-99% savings)** | **Low** | **None** |

## When to Use mcp2cli

**Best for:**
- 20+ tools in your MCP ecosystem
- Long conversations (10+ turns)
- Cost-sensitive applications
- Multi-provider deployments

**May not be worth it for:**
- < 10 tools
- Very short conversations (1-3 turns)
- When every millisecond of latency matters

## Further Reading

- [mcp2cli GitHub](https://github.com/knowsuchagency/mcp2cli) - Original Python implementation
- [Token Cost Analysis](./mcp-servers.md#token-costs) - Detailed cost breakdown
- [MCP Integration](./mcp-integration.md) - OpenRustClaw's MCP architecture

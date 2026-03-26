# CLI API Reference

This reference documents the OpenRustClaw command-line interface.

**Crate**: `openrustclaw-cli`

---

## Overview

The CLI provides commands for:
- Running the gateway server and sidecar
- Interactive chat with agents
- Managing models, skills, and memory
- Scheduling jobs
- Security operations
- MCP server management
- Token-efficient tool discovery (mcp2cli)

---

## Global Options

```bash
openrustclaw [OPTIONS] <COMMAND>

Options:
  -h, --help     Print help
  -V, --version  Print version
```

---

## Commands

### `start`

Start the gateway server and Python sidecar.

```bash
openrustclaw start [OPTIONS]

Options:
  -c, --config <CONFIG>  Config file path [default: config/default.toml]
```

**Example**:
```bash
# Start with default config
openrustclaw start

# Start with custom config
openrustclaw start --config config/production.toml
```

---

### `chat`

Interactive chat with the agent.

```bash
openrustclaw chat [OPTIONS]

Options:
  -p, --provider <PROVIDER>  Provider [default: anthropic]
                             Options: anthropic, openai, openrouter, ollama
  -m, --model <MODEL>        Model to use
```

**Examples**:
```bash
# Chat with default provider (Anthropic)
openrustclaw chat

# Chat with specific provider and model
openrustclaw chat --provider openai --model gpt-4o

# Chat with local Ollama
openrustclaw chat --provider ollama --model llama3.1

# Chat with OpenRouter
openrustclaw chat --provider openrouter --model anthropic/claude-sonnet-4
```

**Chat Commands**:
```
/quit, /q      - Exit the chat
/memory, /m    - Show current conversation context
/tools, /t     - List available tools
/help, /h      - Show help
```

---

### `models`

Manage LLM models.

```bash
openrustclaw models <ACTION>

Actions:
  list       List available models
  info <NAME>  Show model details
```

**Examples**:
```bash
# List all models
openrustclaw models list

# Show model details
openrustclaw models info gpt-4o
```

---

### `skills`

Manage agent skills.

```bash
openrustclaw skills <ACTION>

Actions:
  list              List installed skills
  install <NAME>    Install skill from the workspace or ClawHub registry
  verify <NAME>     Verify skill signatures
```

**Examples**:
```bash
# List installed skills
openrustclaw skills list

# Install a skill
openrustclaw skills install web-search

# Verify skill signature
openrustclaw skills verify web-search

# Inspect schedulable background services for a compiled skill
openrustclaw skills background-services web-search

# Schedule a background workflow for a compiled skill
openrustclaw skills schedule-background web-search --every-seconds 900

# Bind a compiled skill background service to a channel binding
openrustclaw skills bind-channel-extension dm-default web-search --trigger mentioned

# Bind a compiled skill to an OIDC auth-provider lane
openrustclaw skills bind-auth-plugin okta-prod web-search --issuer https://issuer.example.com --redirect-uri https://app.example.com/control/skills/auth-plugins/callback

# Start an authorization flow for a configured auth plugin
openrustclaw skills auth-authorize okta-prod
```

---

### `schedule`

Manage scheduled jobs.

```bash
openrustclaw schedule <ACTION>

Actions:
  list                      List scheduled jobs
  create --name <NAME> --workflow <WORKFLOW>  Create new job
  pause <ID>                Pause a job
  resume <ID>               Resume a job
```

**Examples**:
```bash
# List jobs
openrustclaw schedule list

# Create hourly report job
openrustclaw schedule create --name "hourly-report" --workflow "generate_report"

# Pause a job
openrustclaw schedule pause job_123

# Resume a job
openrustclaw schedule resume job_123
```

---

### `security`

Security audit and management.

```bash
openrustclaw security <ACTION>

Actions:
  audit         Run security audit
  generate-keys Generate Ed25519 keypair for skill signing
```

**Examples**:
```bash
# Run security audit
openrustclaw security audit

# Generate signing keys
openrustclaw security generate-keys
```

---

### `memory`

Memory management.

```bash
openrustclaw memory <ACTION>

Actions:
  export --output <PATH> [--user-id <ID>]  Export memory to markdown
  import --file <PATH> --user-id <ID>      Import memory from markdown file
  stats                                    Show memory statistics
```

**Examples**:
```bash
# Export memory for inspection
openrustclaw memory export --output memory_backup.md --user-id user_42

# Import memory from markdown file
openrustclaw memory import --file MEMORY.md --user-id user_42

# Show statistics
openrustclaw memory stats
```

---

### `doctor`

Run diagnostics.

```bash
openrustclaw doctor
```

Checks:
- Database connectivity
- Provider API keys
- MCP server availability
- Configuration validity
- Required binaries

---

### `cursor`

Set up Cursor IDE integration.

```bash
openrustclaw cursor <ACTION>

Actions:
  setup    Generate .cursor/mcp.json and .cursor/rules/
```

**Example**:
```bash
# Set up Cursor integration
openrustclaw cursor setup
```

Generates:
- `.cursor/mcp.json` - MCP server configuration
- `.cursor/rules/*.mdc` - Cursor rules

---

### `mcp-server`

Start MCP server for external clients.

```bash
openrustclaw mcp-server [OPTIONS]

Options:
  -t, --transport <TRANSPORT>  Transport type [default: stdio]
                               Only `stdio` is currently implemented
```

**Examples**:
```bash
# Start MCP server with stdio (for Claude Desktop)
openrustclaw mcp-server

# Explicit stdio transport
openrustclaw mcp-server --transport stdio
```

---

### `mcp2-cli`

Token-efficient MCP tool discovery and execution.

#### `mcp2-cli list`

List available tools (~16 tokens/tool vs 300-800 native).

```bash
openrustclaw mcp2-cli list [OPTIONS]

Options:
      --mcp <URL>          MCP server URL for remote legacy SSE endpoints
      --mcp-stdio <CMD>    MCP server via stdio command line
      --spec <SPEC>        OpenAPI spec URL or file
      --base-url <URL>     Base URL for OpenAPI
      --refresh            Force refresh cache
      --format <FORMAT>    Output format [default: table]
                           Options: table, json, toon
```

**Examples**:
```bash
# List tools from MCP server over stdio
openrustclaw mcp2-cli list --mcp-stdio 'npx -y @modelcontextprotocol/server-filesystem /'

# JSON output
openrustclaw mcp2-cli list --mcp-stdio 'npx -y @modelcontextprotocol/server-filesystem /' --format json

# OpenAPI spec
openrustclaw mcp2-cli list --spec https://api.example.com/openapi.json --base-url https://api.example.com

# Force cache refresh
openrustclaw mcp2-cli list --mcp-stdio 'npx -y @modelcontextprotocol/server-filesystem /' --refresh
```

#### `mcp2-cli help`

Get tool help (~80-200 tokens).

```bash
openrustclaw mcp2-cli help [OPTIONS] <TOOL>

Options:
      --mcp <URL>      MCP server URL (currently unsupported)
      --mcp-stdio <CMD>  MCP server via stdio command line
      --spec <SPEC>    OpenAPI spec URL or file
      --format <FMT>   Output format [default: text]
                       Options: text, json, toon
```

**Example**:
```bash
openrustclaw mcp2-cli help --mcp-stdio 'npx -y @modelcontextprotocol/server-filesystem /' read_file
```

#### `mcp2-cli run`

Execute a tool.

```bash
openrustclaw mcp2-cli run [OPTIONS] <TOOL>

Options:
      --mcp <URL>      MCP server URL (currently unsupported)
      --mcp-stdio <CMD>  MCP server via stdio command line
      --spec <SPEC>    OpenAPI spec URL or file
      --args <ARGS>    JSON arguments
      --stdin          Read arguments from stdin
      --format <FMT>   Output format [default: json]
                       Options: json, text
```

**Examples**:
```bash
# Run with inline args
openrustclaw mcp2-cli run --mcp-stdio 'npx -y @modelcontextprotocol/server-filesystem /' read_file --args '{"path":"/etc/hosts"}'

# Run with stdin
echo '{"path":"/etc/hosts"}' | openrustclaw mcp2-cli run --mcp-stdio 'npx -y @modelcontextprotocol/server-filesystem /' read_file --stdin
```

---

### `tools`

Workspace-owned local CLI profiling and host-artifact generation.

#### `tools setup`

Probe the common local CLI set and initialize the workspace tool registry.

```bash
openrustclaw tools setup [OPTIONS]

Options:
      --tool <TOOL_NAME>...   Explicit tool names to probe instead of the default set
      --host <HOST>...        Target startup bundle hosts
                              Options: claude-code, cursor, codex, gemini-cli, github-copilot
```

**Examples**:
```bash
openrustclaw tools setup
openrustclaw tools setup --tool git --tool cargo --host codex --host cursor
```

#### `tools add`

Probe one local CLI and persist its normalized tool profile.

```bash
openrustclaw tools add <NAME> [OPTIONS]

Options:
      --path <PATH>           Explicit executable path
      --host <HOST>...        Target startup bundle hosts
```

**Examples**:
```bash
openrustclaw tools add gh
openrustclaw tools add docker --path /usr/local/bin/docker --host claude-code
```

#### `tools status`

List persisted tool profiles and local drift state.

```bash
openrustclaw tools status [OPTIONS]

Options:
      --name <NAME>           Filter to one persisted tool profile
```

#### `tools show`

Inspect one persisted tool profile in detail.

```bash
openrustclaw tools show <NAME>
```

#### `tools sync`

Re-probe persisted tool profiles and optionally rewrite generated host artifacts.

```bash
openrustclaw tools sync [OPTIONS]

Options:
      --name <NAME>           Filter to one persisted tool profile
      --host <HOST>...        Override the generated host bundle set
      --apply                 Persist the refreshed profile and rewrite artifacts
```

Generated files:

- Registry: `.claw/control/tool-profiles.json`
- Host bundles: `.claw/control/tool-hosts/<host>/`

Matching control-plane APIs:

- `GET /control/tools`
- `GET /control/tools/{name}`
- `POST /control/tools/add`
- `POST /control/tools/setup`
- `POST /control/tools/sync`

#### `mcp2-cli analyze`

Analyze token cost savings.

```bash
openrustclaw mcp2-cli analyze [OPTIONS]

Options:
  -t, --tools <N>    Number of tools [default: 30]
  -n, --turns <N>    Number of turns [default: 15]
  -u, --used <N>     Tools used per turn [default: 5]
```

**Example**:
```bash
openrustclaw mcp2-cli analyze --tools 50 --turns 20 --used 8
```

#### `mcp2-cli toon`

Convert to/from TOON (Token-Optimized Output Notation) format.

```bash
openrustclaw mcp2-cli toon [INPUT] [OPTIONS]

Options:
      --decode    Decode TOON instead of encode
```

**Examples**:
```bash
# Encode JSON to TOON
cat tools.json | openrustclaw mcp2-cli toon

# Decode TOON to JSON
cat tools.toon | openrustclaw mcp2-cli toon --decode
```

#### `mcp2-cli cache`

Cache management.

```bash
openrustclaw mcp2-cli cache <ACTION>

Actions:
  clear    Clear all cached data
  stats    Show cache statistics
```

**Examples**:
```bash
# Show cache stats
openrustclaw mcp2-cli cache stats

# Clear cache
openrustclaw mcp2-cli cache clear
```

---

## Environment Variables

| Variable | Description | Required For |
|----------|-------------|--------------|
| `ANTHROPIC_API_KEY` | Anthropic API key | Anthropic provider |
| `OPENAI_API_KEY` | OpenAI API key | OpenAI provider |
| `OPENROUTER_API_KEY` | OpenRouter API key | OpenRouter provider |
| `OLLAMA_HOST` | Ollama host URL | Ollama provider (optional) |
| `JWT_SECRET` | JWT signing secret | Authentication |
| `DATABASE_URL` | SQLite database URL | Persistence |
| `RUST_LOG` | Log level (debug, info, warn, error) | Logging |

---

## Configuration File

Default location: `config/default.toml`

```toml
[server]
host = "127.0.0.1"
port = 8080

[database]
url = "sqlite://data/openrustclaw.db"

[providers]
primary = "anthropic"
fallback = ["openai", "openrouter"]

[providers.anthropic]
api_key = "${ANTHROPIC_API_KEY}"
model = "claude-sonnet-4-20250514"

[providers.openai]
api_key = "${OPENAI_API_KEY}"
model = "gpt-4o"

[providers.openrouter]
api_key = "${OPENROUTER_API_KEY}"
model = "anthropic/claude-sonnet-4"

[providers.ollama]
model = "llama3.1"
base_url = "http://localhost:11434"

[memory]
core_max_tokens = 500
recall_ttl_episodic_days = 90

[scheduler]
enabled = true
poll_interval_secs = 30
max_retries = 3

[security]
allowed_origins = ["https://app.example.com"]
skill_verification_required = true
input_sanitization_enabled = true

[mcp]
servers = [
    { name = "filesystem", command = "npx", args = ["-y", "@modelcontextprotocol/server-filesystem", "/"], enabled = true },
]

[logging]
level = "info"
format = "json"
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Configuration error |
| 4 | Network error |
| 5 | Authentication error |
| 6 | Not found |

---

## Shell Completion

Generate shell completion scripts:

```bash
# Bash
openrustclaw completions bash > /etc/bash_completion.d/openrustclaw

# Zsh
openrustclaw completions zsh > /usr/local/share/zsh/site-functions/_openrustclaw

# Fish
openrustclaw completions fish > ~/.config/fish/completions/openrustclaw.fish
```

---

## Examples

### Quick Start

```bash
# 1. Configure environment
export ANTHROPIC_API_KEY="sk-ant-..."

# 2. Run diagnostics
openrustclaw doctor

# 3. Start interactive chat
openrustclaw chat

# 4. Start server
openrustclaw start
```

### Development Workflow

```bash
# Start local Ollama for testing
ollama serve &
ollama pull llama3.1

# Test with local model
openrustclaw chat --provider ollama --model llama3.1

# Set up Cursor IDE
openrustclaw cursor setup

# Run security audit
openrustclaw security audit
```

### Production Deployment

```bash
# Create production config
cat > config/production.toml << 'EOF'
[server]
host = "0.0.0.0"
port = 8080

[providers]
primary = "anthropic"
fallback = ["openrouter"]

[security]
allowed_origins = ["https://myapp.com"]
skill_verification_required = true
EOF

# Run with production config
openrustclaw start --config config/production.toml
```

For the shipped config format, the gateway deployment contract is now explicit:

```toml
[gateway]
network_mode = "remote" # loopback, lan, remote
host = "0.0.0.0"
port = 8080
allowed_origins = ["https://myapp.com"]
```

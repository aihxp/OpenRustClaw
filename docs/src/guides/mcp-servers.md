# Connecting MCP Servers

The Model Context Protocol (MCP) enables OpenRustClaw to connect with thousands of existing tools and services. This guide covers setting up and using MCP servers.

---

## 🎯 What Are MCP Servers?

MCP servers are standalone processes that expose tools through a standardized protocol. They can be:

- **Local processes** — Run on your machine (filesystem, databases)
- **Remote services** — Connect over HTTP/SSE (web APIs)
- **Containerized** — Run in Docker for isolation

```mermaid
flowchart LR
    ORC["OpenRustClaw"] --> MCP["MCP Protocol"]
    
    subgraph Servers["MCP Servers"]
        FS["Filesystem"]
        DB["Database"]
        GH["GitHub"]
        WEB["Web Search"]
        GIT["Git"]
    end
    
    MCP --> Servers
```

---

## 📦 Available MCP Servers

### Official MCP Servers

| Server | Package | Description |
|--------|---------|-------------|
| Filesystem | `@modelcontextprotocol/server-filesystem` | Read/write local files |
| GitHub | `@modelcontextprotocol/server-github` | GitHub API access |
| PostgreSQL | `@modelcontextprotocol/server-postgres` | Query PostgreSQL |
| SQLite | `@modelcontextprotocol/server-sqlite` | Query SQLite databases |
| Brave Search | `@modelcontextprotocol/server-brave-search` | Web search |
| Fetch | `@modelcontextprotocol/server-fetch` | HTTP requests |
| Puppeteer | `@modelcontextprotocol/server-puppeteer` | Browser automation |
| Sentry | `@modelcontextprotocol/server-sentry` | Error tracking |
| Slack | `@modelcontextprotocol/server-slack` | Slack integration |

### Community MCP Servers

- `@modelcontextprotocol/server-google-drive` — Google Drive access
- `@modelcontextprotocol/server-notion` — Notion integration
- `@modelcontextprotocol/server-discord` — Discord bot
- `@modelcontextprotocol/server-spotify` — Spotify control

---

## ⚙️ Configuration

### Global Configuration

Create `config/mcp-servers.toml`:

```toml
# Filesystem access
[[servers]]
name = "filesystem"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/projects"]

# GitHub integration
[[servers]]
name = "github"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
[servers.env]
GITHUB_PERSONAL_ACCESS_TOKEN = "${GITHUB_TOKEN}"

# PostgreSQL database
[[servers]]
name = "postgres"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-postgres", "postgresql://localhost/mydb"]

# Brave Search
[[servers]]
name = "brave-search"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-brave-search"]
[servers.env]
BRAVE_API_KEY = "${BRAVE_API_KEY}"

# Web fetch
[[servers]]
name = "fetch"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-fetch"]
```

### Per-Project Configuration

Create `mcp.json` in your project root:

```json
{
  "servers": [
    {
      "name": "project-files",
      "transport": "stdio",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
    },
    {
      "name": "project-db",
      "transport": "stdio",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-postgres", "${DATABASE_URL}"]
    }
  ]
}
```

---

## 🚀 Setup Instructions

### Prerequisites

```bash
# Install Node.js (required for npx)
# macOS
brew install node

# Ubuntu/Debian
sudo apt install nodejs npm

# Verify installation
node --version  # Should be 18+
npm --version
```

### Filesystem Server

```bash
# Install globally (optional)
npm install -g @modelcontextprotocol/server-filesystem

# Test directly
npx -y @modelcontextprotocol/server-filesystem /path/to/allowed/dir
```

Configure in `mcp-servers.toml`:

```toml
[[servers]]
name = "my-files"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/projects"]
```

### GitHub Server

1. Get a GitHub personal access token:
   - Visit https://github.com/settings/tokens
   - Create token with `repo` scope

2. Configure:

```toml
[[servers]]
name = "github"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
[servers.env]
GITHUB_PERSONAL_ACCESS_TOKEN = "${GITHUB_TOKEN}"
```

3. Add to `.env`:

```bash
GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxx
```

### Brave Search Server

1. Get API key from https://brave.com/search/api/

2. Configure:

```toml
[[servers]]
name = "web-search"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-brave-search"]
[servers.env]
BRAVE_API_KEY = "${BRAVE_API_KEY}"
```

### PostgreSQL Server

```toml
[[servers]]
name = "database"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-postgres", "postgresql://user:pass@localhost/dbname"]
```

Or use connection string from environment:

```toml
[[servers]]
name = "database"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-postgres", "${DATABASE_URL}"]
```

---

## 🔧 Using MCP Servers

### Listing Available Tools

```bash
# List all MCP tools
openrustclaw mcp tools

# Output:
# Server: filesystem
#   - read_file(path: string)
#   - write_file(path: string, content: string)
#   - list_directory(path: string)
#   - search_files(path: string, pattern: string)
#
# Server: github
#   - search_repositories(query: string)
#   - create_issue(owner: string, repo: string, title: string, body: string)
#   - create_pull_request(owner: string, repo: string, title: string, ...)
```

### Testing MCP Servers

```bash
# Test a specific server
openrustclaw mcp test filesystem

# Test all servers
openrustclaw mcp test --all

# Output:
# ✓ filesystem (4 tools available)
# ✓ github (12 tools available)
# ✗ postgres (connection refused)
```

### Interactive MCP Shell

```bash
# Enter MCP shell for testing
openrustclaw mcp shell

> use filesystem
> call read_file {"path": "/home/user/README.md"}
< File content displayed...

> use github
> call search_repositories {"query": "rust ai agent"}
< Repository results...
```

---

## 💡 Usage Examples

### File Operations

With the filesystem MCP server connected:

```
User: Read the README file

Agent: [filesystem/read_file] {"path": "README.md"}

The README describes an AI agent framework called OpenRustClaw...
```

```
User: Find all Rust files in the project

Agent: [filesystem/search_files] {"path": ".", "pattern": "**/*.rs"}

Found 42 Rust files:
- src/main.rs
- src/lib.rs
- crates/core/src/lib.rs
...
```

### GitHub Integration

```
User: Check for open issues in the repository

Agent: [github/search_issues] {"owner": "openrustclaw", "repo": "openrustclaw", "state": "open"}

There are 5 open issues:
1. #123 - Improve error handling
2. #124 - Add more tests
...
```

### Database Queries

```
User: How many users signed up this week?

Agent: [postgres/query] {"sql": "SELECT COUNT(*) FROM users WHERE created_at > NOW() - INTERVAL '7 days'"}

This week, 156 new users signed up.
```

### Web Search

```
User: Search for recent Rust concurrency patterns

Agent: [brave-search/search] {"query": "Rust concurrency patterns 2024"}

Here are some recent articles about Rust concurrency:
1. "Modern Rust Concurrency" by...
2. "Tokio Best Practices" by...
```

---

## 🔒 Security Considerations

### Filesystem Access

**Always specify allowed directories:**

```toml
# Good - restricted access
args = ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/projects"]

# Bad - full filesystem access
args = ["-y", "@modelcontextprotocol/server-filesystem", "/"]
```

### Environment Variables

Use environment variable substitution for secrets:

```toml
# Good
[servers.env]
GITHUB_TOKEN = "${GITHUB_TOKEN}"

# Bad - hardcoded token
[servers.env]
GITHUB_TOKEN = "ghp_abc123..."
```

### Network Access

Only enable network access for trusted servers:

```toml
# Review before enabling
[[servers]]
name = "fetch"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-fetch"]
# This allows arbitrary HTTP requests
```

---

## 🔧 Troubleshooting

### "Server not found" errors

```bash
# Verify server is installed
npm list -g @modelcontextprotocol/server-filesystem

# Reinstall if needed
npm install -g @modelcontextprotocol/server-filesystem
```

### "Connection refused" errors

```bash
# For database servers, check service is running
pg_isready -h localhost

# For HTTP servers, check URL is correct
curl http://localhost:8080/health
```

### "Permission denied" errors

```bash
# Check filesystem permissions
ls -la /path/to/allowed/dir

# For MCP servers, ensure paths are absolute
# Bad: "./projects"
# Good: "/home/user/projects"
```

### Debugging MCP Servers

```bash
# Enable verbose logging
RUST_LOG=debug openrustclaw mcp test filesystem

# Run server manually to see errors
npx -y @modelcontextprotocol/server-filesystem /path/to/dir
```

---

## 📝 Advanced Configuration

### SSE Transport (Remote Servers)

```toml
[[servers]]
name = "remote-api"
transport = "sse"
url = "https://api.example.com/mcp"
headers = { Authorization = "Bearer ${API_TOKEN}" }
```

### Docker-based Servers

```toml
[[servers]]
name = "isolated-filesystem"
transport = "stdio"
command = "docker"
args = ["run", "--rm", "-i", "-v", "/host/path:/data:ro", "mcp/filesystem", "/data"]
```

### Conditional Loading

```toml
[[servers]]
name = "production-db"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-postgres", "${PROD_DATABASE_URL}"]
# Only load if environment variable is set
required_env = ["PROD_DATABASE_URL"]
```

---

## 🎓 Best Practices

### 1. Use Specific Paths

```toml
# Good
args = ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/projects/myapp"]

# Bad
args = ["-y", "@modelcontextprotocol/server-filesystem", "/"]
```

### 2. Separate Credentials

```bash
# .env file (never commit)
GITHUB_TOKEN=ghp_xxx
BRAVE_API_KEY=bs_xxx
DATABASE_URL=postgresql://...

# mcp-servers.toml
[servers.env]
GITHUB_TOKEN = "${GITHUB_TOKEN}"
```

### 3. Test Before Enabling

```bash
# Always test servers before using
openrustclaw mcp test filesystem
openrustclaw mcp test github
```

### 4. Document Your Setup

```markdown
# MCP Servers Setup

## Required Environment Variables
- `GITHUB_TOKEN` - GitHub personal access token
- `BRAVE_API_KEY` - Brave Search API key

## Available Tools
- `filesystem/read_file` - Read project files
- `github/search_issues` - Find GitHub issues
- `brave-search/search` - Web search
```

### 5. Limit Concurrent Servers

Too many MCP servers can impact performance:

```toml
# Recommended: 3-5 active servers
# Prioritize based on your workflow

# Essential
- filesystem
- github

# Optional
- postgres (only when working with DB)
- brave-search (only when researching)
```

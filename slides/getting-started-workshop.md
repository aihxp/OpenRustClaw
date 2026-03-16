---
marp: true
theme: default
paginate: true
class: invert
header: 'Getting Started Workshop'
footer: '© 2026 OpenRustClaw Project'
---

<!--
Speaker Notes: Hands-on workshop deck for getting started with OpenRustClaw. Plan for 45-60 minutes with live demos.
-->

<style>
section {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}
h1, h2 {
  color: #3498db;
}
strong {
  color: #e67e22;
}
table {
  font-size: 0.85em;
}
code {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.8em;
}
</style>

# 🚀 Getting Started Workshop

## Hands-On with OpenRustClaw

### From Zero to Your First Agent Interaction

---

<!--
Speaker Notes: Start with prerequisites check. Make sure everyone has these installed before proceeding.
-->

## 📋 Prerequisites

### Required Software

| Requirement | Version | Check Command |
|-------------|---------|---------------|
| **Rust** | 1.85+ | `rustc --version` |
| **Cargo** | 1.85+ | `cargo --version` |
| **Python** | 3.11+ | `python3 --version` |
| **protoc** | 3.20+ | `protoc --version` |

### Optional Tools

| Tool | Purpose | Install |
|------|---------|---------|
| **Just** | Command runner | `cargo install just` |
| **marp-cli** | Slide generation | `npm install -g @marp-team/marp-cli` |
| **mdbook** | Documentation | `cargo install mdbook` |

### System Requirements

```
Minimum:
- 4GB RAM
- 10GB disk space
- Internet connection (for providers)

Recommended:
- 8GB+ RAM
- SSD storage
- GPU (for local Ollama models)
```

---

<!--
Speaker Notes: Walk through the installation process step by step. Pause for questions.
-->

## 🔧 Installation Demo

### Step 1: Clone Repository

```bash
# Clone the repository
git clone https://github.com/hprincivil/OpenRustClaw.git
cd OpenRustClaw

# Verify structure
ls -la
# Output: Cargo.toml  crates/  sidecar/  proto/  config/  ...
```

### Step 2: Build the Project

```bash
# Build all crates (this may take 5-10 minutes on first run)
cargo build --workspace

# Verify build
cargo test --workspace --lib

# Expected output:
# running X tests
# test result: ok. X passed; 0 failed
```

### Step 3: Set Up Python Sidecar

```bash
cd sidecar

# Create virtual environment
python3 -m venv .venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate

# Install dependencies
pip install -r requirements.txt

# Verify
cd ..  # Back to project root
```

---

<!--
Speaker Notes: Configuration is critical. Explain each setting and its purpose.
-->

## ⚙️ Configuration

### Environment Setup

```bash
# Copy example configuration
cp .env.example .env

# Edit .env with your API keys
nano .env  # or your preferred editor
```

### Required Configuration

```bash
# .env

# At least one LLM provider required
ANTHROPIC_API_KEY=sk-ant-api03-...
OPENAI_API_KEY=sk-...
# OPENROUTER_API_KEY=sk-or-...

# Database (optional, defaults to local SQLite)
DATABASE_URL=sqlite:./data/openrustclaw.db

# Security (generate with: openrustclaw security generate-keys)
JWT_SECRET=your-secret-key-here

# Observability (optional)
LANGSMITH_API_KEY=ls-...
LANGSMITH_PROJECT=openrustclaw

# Gateway
GATEWAY_PORT=3000
GATEWAY_HOST=127.0.0.1
```

### Configuration Files

```toml
# config/openrustclaw.toml
[server]
websocket_port = 3000
max_connections = 1000

[memory]
embedding_model = "openai/text-embedding-3-small"
search_limit = 10

[providers]
primary = "anthropic"
fallback_chain = ["openai", "openrouter"]

[security]
require_authentication = true
allowed_origins = ["http://localhost:3000", "http://localhost:5173"]
```

---

<!--
Speaker Notes: First interaction with OpenRustClaw. Show the chat interface and basic commands.
-->

## 💬 First Chat

### Start the Services

```bash
# Terminal 1: Start the gateway and sidecar
cargo run --bin openrustclaw -- start

# You should see:
# [INFO] Starting OpenRustClaw gateway on 127.0.0.1:3000
# [INFO] Connected to Python sidecar via gRPC
# [INFO] WebSocket server ready
```

### Interactive Chat

```bash
# Terminal 2: Start interactive chat
cargo run --bin openrustclaw -- chat --provider anthropic

# Welcome to OpenRustClaw Chat!
# Provider: anthropic (claude-3-5-sonnet-20241022)
# Type /help for commands, /quit to exit

> Hello! What can you do?
Assistant: Hello! I'm an AI assistant powered by OpenRustClaw. I can help you with:
- Answering questions
- Running tools and skills
- Managing your memory and context
- Scheduling tasks

How can I help you today?

> /help
Available commands:
  /memory     - Show memory statistics
  /tools      - List available tools
  /clear      - Clear conversation history
  /provider   - Switch LLM provider
  /quit       - Exit chat
```

---

<!--
Speaker Notes: Demonstrate tool usage, which is a core feature of OpenRustClaw.
-->

## 🛠️ Tool Usage

### Available Tools

```markdown
# In chat, type:
> /tools

Available Tools:
┌─────────────────┬────────────────────────────────────────┐
│ Tool            │ Description                            │
├─────────────────┼────────────────────────────────────────┤
│ memory_search   │ Search agent memory for information    │
│ memory_add      │ Add information to memory              │
│ file_read       │ Read contents of a file                │
│ file_write      │ Write content to a file                │
│ shell_execute   │ Execute shell commands (sandboxed)     │
│ http_request    │ Make HTTP requests                     │
│ schedule_task   │ Schedule a task for later execution    │
└─────────────────┴────────────────────────────────────────┘
```

### Using Tools

```markdown
# Tools are automatically invoked when needed

> What's the weather in San Francisco?

Assistant: I'll check the weather for you.
[Tool: http_request]
  URL: https://api.weather.gov/points/37.7749,-122.4194
  
The weather in San Francisco is currently 65°F and partly cloudy.

# You can also ask explicitly
> Search my memory for "project goals"

Assistant: [Tool: memory_search]
  Query: "project goals"
  
Found 3 memories:
1. "Q4 Goals: Launch OpenRustClaw v1.0"
2. "Personal: Learn Rust async programming"
3. "Meeting notes: Discuss project roadmap"
```

---

<!--
Speaker Notes: Show the 3-tier memory system in action. This is a key differentiator.
-->

## 🧠 Memory Management

### Memory Commands

```bash
# In chat
> /memory

Memory Statistics:
┌─────────────────┬─────────┬──────────┐
│ Tier            │ Entries │ Size     │
├─────────────────┼─────────┼──────────┤
│ Core            │ 12      │ 423 tok  │
│ Recall          │ 1,247   │ 45.2 MB  │
│ Archive         │ 23      │ 12.1 MB  │
└─────────────────┴─────────┴──────────┘

Recent Core Memories:
- User: developer, prefers Rust
- Project: OpenRustClaw presentation
- Goals: Create comprehensive slides
```

### Adding to Memory

```markdown
# Automatically added:
- Tool results
- Important facts from conversation
- User preferences

# Manually add:
> Remember that my favorite color is blue

Assistant: [Tool: memory_add]
  Content: "User's favorite color is blue"
  Tier: core
  
Got it! I've saved that your favorite color is blue.

# Later...
> What did I tell you about my preferences?

Assistant: Based on my memory, I know:
- Your favorite color is blue
- You're a developer who prefers Rust
- You're working on OpenRustClaw presentation materials
```

---

<!--
Speaker Notes: Demonstrate the durable scheduler. This is a powerful feature for automation.
-->

## ⏰ Scheduling Tasks

### Creating Scheduled Tasks

```bash
# Using CLI
cargo run --bin openrustclaw -- schedule create \
  --name "daily-report" \
  --prompt "Generate a summary of yesterday's commits" \
  --cron "0 9 * * *" \
  --timezone "America/New_York"

# Output: Created job with ID: job_abc123
```

### Listing and Managing Tasks

```bash
# List all scheduled jobs
cargo run --bin openrustclaw -- schedule list

┌─────────────┬─────────────┬──────────────┬──────────┐
│ ID          │ Name        │ Schedule     │ Status   │
├─────────────┼─────────────┼──────────────┼──────────┤
│ job_abc123  │ daily-report│ 0 9 * * *    │ active   │
│ job_def456  │ weekly-sync │ 0 10 * * MON │ active   │
│ job_ghi789  │ reminder    │ 0 14 * * *   │ paused   │
└─────────────┴─────────────┴──────────────┴──────────┘

# Pause a job
cargo run --bin openrustclaw -- schedule pause job_abc123

# Resume
cargo run --bin openrustclaw -- schedule resume job_abc123

# Delete
cargo run --bin openrustclaw -- schedule delete job_abc123
```

### In Chat

```markdown
> Remind me to check the deployment in 30 minutes

Assistant: [Tool: schedule_task]
  Task: "Check deployment status"
  When: "in 30 minutes"
  
I've scheduled a reminder for you in 30 minutes.
```

---

<!--
Speaker Notes: Cursor integration is a key feature for developers. Show how to set it up.
-->

## 🖥️ Cursor Integration

### Setup

```bash
# Auto-generate Cursor configuration
cargo run --bin openrustclaw -- cursor setup

# Output:
# ✓ Created .cursor/mcp.json
# ✓ Created .cursor/rules/
#   - rust-conventions.mdc
#   - crate-architecture.mdc
#   - provider-patterns.mdc
#   - memory-patterns.mdc
#   - testing-patterns.mdc
# ✓ MCP server configured
# ✓ CLAUDE.md updated
```

### What's Configured

```json
// .cursor/mcp.json
{
  "mcpServers": {
    "openrustclaw": {
      "command": "cargo",
      "args": [
        "run",
        "--bin",
        "openrustclaw",
        "--",
        "mcp-server"
      ],
      "env": {
        "DATABASE_URL": "sqlite:./data/openrustclaw.db"
      }
    }
  }
}
```

### Using in Cursor

```markdown
# In Cursor Composer or Chat:

"@openrustclaw Search my memory for the database schema"

"@openrustclaw What tasks do I have scheduled?"

"@openrustclaw Run a security audit on my project"
```

---

<!--
Speaker Notes: Other useful CLI commands and diagnostic tools.
-->

## 🔍 Diagnostics & Utilities

### Doctor Command

```bash
# Run full diagnostics
cargo run --bin openrustclaw -- doctor

Running diagnostics...

✓ Rust version: 1.85.0
✓ Cargo version: 1.85.0
✓ Python version: 3.11.4
✓ Protoc version: libprotoc 25.1
✓ Database connection: OK
✓ Anthropic API: Connected
✓ OpenAI API: Connected
✓ Sidecar gRPC: Connected
✓ WebSocket server: Ready

All systems operational! 🎉
```

### Other Useful Commands

```bash
# Memory operations
cargo run --bin openrustclaw -- memory export --format markdown
cargo run --bin openrustclaw -- memory stats
cargo run --bin openrustclaw -- memory search "project goals"

# Security
cargo run --bin openrustclaw -- security audit
cargo run --bin openrustclaw -- security generate-keys

# Skills
cargo run --bin openrustclaw -- skills list
cargo run --bin openrustclaw -- skills install <skill-id>

# Models
cargo run --bin openrustclaw -- models list
cargo run --bin openrustclaw -- models info claude-3-5-sonnet
```

---

<!--
Speaker Notes: Troubleshooting common issues that users might encounter.
-->

## 🔧 Troubleshooting

### Common Issues

```markdown
## Issue: Build fails with "protoc not found"
Solution:
  Ubuntu/Debian: sudo apt install protobuf-compiler
  macOS: brew install protobuf
  Windows: choco install protoc

## Issue: "Connection refused" to sidecar
Solution:
  1. Check if Python venv is activated
  2. Verify requirements installed: pip install -r sidecar/requirements.txt
  3. Check sidecar logs: tail -f sidecar/logs/server.log

## Issue: API key errors
Solution:
  1. Verify .env file exists: cat .env
  2. Check key format (should start with sk-...)
  3. Test key directly: curl with provider API

## Issue: Database locked
Solution:
  1. Kill any hanging processes: pkill openrustclaw
  2. Check for WAL files: ls data/*.wal
  3. Restart gateway: cargo run --bin openrustclaw -- start
```

### Getting Help

```markdown
# Run with verbose logging
RUST_LOG=debug cargo run --bin openrustclaw -- start

# Check logs
tail -f logs/openrustclaw.log

# GitHub Issues
https://github.com/hprincivil/OpenRustClaw/issues

# Documentation
cat docs/src/troubleshooting.md
```

---

<!--
Speaker Notes: Wrap up with next steps and resources for continued learning.
-->

## 🎯 Next Steps

### Learning Path

```
Week 1: Basics
├── Complete this workshop
├── Try all CLI commands
├── Create your first scheduled task
└── Explore the memory system

Week 2: Advanced
├── Write a custom skill (SKILL.md)
├── Configure multiple providers
├── Set up Cursor integration
└── Review security settings

Week 3: Production
├── Deploy with Docker
├── Set up monitoring (LangSmith)
├── Configure backups
└── Join the community
```

### Resources

| Resource | Location |
|----------|----------|
| 📖 Full Documentation | `docs/` directory |
| 🎤 Presentation Slides | `slides/` directory |
| 🐛 Report Issues | GitHub Issues |
| 💬 Discussions | GitHub Discussions |
| 📚 Architecture | `docs/src/architecture/` |
| 🔐 Security | `docs/src/security/` |

---

<!--
Speaker Notes: Final slide with key takeaways and contact information.
-->

## ✅ Workshop Summary

### What You Learned

1. **🔧 Installation**
   - Prerequisites: Rust, Python, protoc
   - Build: `cargo build --workspace`
   - Configure: Copy and edit `.env`

2. **💬 First Interaction**
   - Start services: `openrustclaw start`
   - Chat: `openrustclaw chat --provider anthropic`
   - Tools: Automatically invoked or manual with `/`

3. **🧠 Memory System**
   - 3 tiers: Core, Recall, Archive
   - Search: `memory_search` tool
   - Add: Automatic or manual

4. **⏰ Scheduling**
   - CLI: `openrustclaw schedule create`
   - Durable: Survives restarts
   - Flexible: Cron or natural language

5. **🖥️ Cursor Integration**
   - Setup: `openrustclaw cursor setup`
   - MCP: Access tools from IDE
   - Rules: 5 MDC files for best practices

### You're Ready! 🎉

Start building with OpenRustClaw:
```bash
cargo run --bin openrustclaw -- chat --provider anthropic
```

---

## 🙏 Thank You!

## Questions?

### Quick Reference Card

```bash
# Start everything
cargo run --bin openrustclaw -- start

# Interactive chat
cargo run --bin openrustclaw -- chat

# Run diagnostics
cargo run --bin openrustclaw -- doctor

# Cursor setup
cargo run --bin openrustclaw -- cursor setup

# Get help
cargo run --bin openrustclaw -- --help
```

### Connect

🐙 **GitHub**: github.com/hprincivil/OpenRustClaw  
⭐ **Star the repo** to show support!  
🐛 **Issues**: Report bugs and request features  
💬 **Discussions**: Share your projects

---

<!--
Speaker Notes: Bonus content for those who want to go deeper. Can be skipped in time-constrained workshops.
-->

## 📎 Bonus: Development Mode

### Hot Reload for Development

```bash
# Install cargo-watch
cargo install cargo-watch

# Watch and restart on changes
cargo watch -x 'run --bin openrustclaw -- start'
```

### Running Tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p openrustclaw-memory

# With output
cargo test --workspace -- --nocapture

# Integration tests
cargo test --workspace --test '*'
```

### Code Quality

```bash
# Format
cargo fmt --all

# Lint
cargo clippy --workspace -- -D warnings

# Check
cargo check --workspace
```

---

## 📎 Bonus: Custom Skills

### Creating a SKILL.md

```markdown
<!-- skills/my-custom-skill/SKILL.md -->
# My Custom Skill

## Description
Does something awesome

## Signature
```json
{
  "name": "my_awesome_tool",
  "parameters": {
    "input": {"type": "string"}
  }
}
```

## Handler
<!-- Can be WASM, Python, or shell -->
```

### Installing

```bash
# Local skill
cargo run --bin openrustclaw -- skills install ./skills/my-custom-skill

# From registry
cargo run --bin openrustclaw -- skills install openrustclaw/calculator

# Verify
cargo run --bin openrustclaw -- skills list
```

### Verification (Ed25519)

```bash
# Generate keys
openrustclaw security generate-keys

# Sign skill
openrustclaw security sign-skill ./skills/my-custom-skill

# Verify
openrustclaw security verify-skill my-custom-skill
```

# OpenRustClaw

**A production-ready Rust AI agent framework** — secure, fast, and extensible.

> 🦀 Rust core + 🐍 Python sidecar for the best of both worlds

[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange)](https://rust-lang.org)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/Tests-250%2B-green)]()

## Why OpenRustClaw?

Built as a ground-up reimagining of [OpenClaw](https://github.com/openclaw), addressing critical security vulnerabilities and performance bottlenecks:

| Issue | OpenClaw | OpenRustClaw |
|-------|----------|--------------|
| **Security** | CVE-2026-25253 (CVSS 8.8) | ✅ Fixed: Mandatory auth + origin validation |
| **Memory Waste** | 93.5% token overhead | ✅ 3-tier recall-only system |
| **Injection Defense** | 17% success rate | ✅ Multi-layer protection |
| **Skill Safety** | 1000+ malicious skills | ✅ Ed25519 + WASM sandboxing |
| **Scheduling** | Missed reminders | ✅ Durable scheduler with leases |

---

## Quick Start

```bash
# Install
cargo install openrustclaw

# Run interactive setup
openrustclaw onboard

# Start services
openrustclaw start

# Chat with your agent
openrustclaw chat
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    RUST CORE (19 crates)                     │
│  Gateway (Axum)  ←→  Agent Runtime  ←→  Tool Execution      │
│  Auth/SSO         ←→  Memory Layer   ←→  WASM Sandbox       │
│  20+ Channels     ←→  Scheduler      ←→  Voice/Canvas       │
└──────────────────────────┬──────────────────────────────────┘
                           │ gRPC
┌──────────────────────────▼──────────────────────────────────┐
│              PYTHON LANGGRAPH SIDECAR                        │
│  Agent Orchestration  ←→  RAG Pipeline  ←→  Evaluators      │
└─────────────────────────────────────────────────────────────┘
```

---

## Features

### 🤖 LLM Providers
- **Anthropic** — Messages API, Batch API (50% savings)
- **OpenAI** — Responses API + Chat Completions
- **OpenRouter** — 400+ models with auto-routing
- **Gemini** — Google AI integration
- **Ollama** — Local models, fully offline

### 💬 Channels (20+ Integrations)
| Channel | Status | Channel | Status |
|---------|--------|---------|--------|
| Telegram | ✅ | Discord | ✅ |
| Slack | ✅ | WhatsApp | 🚧 |
| Teams | 🚧 | Google Chat | 🚧 |
| Signal | 🚧 | WebChat | ✅ |

*🚧 = In Development*

### 🧠 3-Tier Memory
- **Core** (~500 tokens) — Always-loaded identity
- **Recall** — On-demand semantic search
- **Archive** — Long-term summaries

### 🔒 Enterprise Security
- JWT authentication
- OIDC/SAML SSO (Okta, Azure AD, Auth0)
- Prompt injection defense
- Ed25519 skill signatures
- Per-session filesystem isolation

### 🛠️ Tool Ecosystem
- MCP client/server (18,000+ tools)
- Browser automation (Playwright/CDP)
- File system, shell execution
- Custom WASM sandboxed skills

### ☁️ Production Ready
- Kubernetes Helm charts
- Terraform (AWS/GCP/Azure)
- Docker/Podman support
- Horizontal autoscaling
- Distributed consensus (Raft)

---

## Installation

### From Source
```bash
git clone https://github.com/openrustclaw/openrustclaw.git
cd openrustclaw
cargo build --release
```

### Docker
```bash
docker run -p 8080:8080 \
  -e ANTHROPIC_API_KEY=sk-ant-... \
  ghcr.io/openrustclaw/openrustclaw:latest
```

### Kubernetes
```bash
helm repo add openrustclaw https://openrustclaw.github.io/charts
helm install openrustclaw openrustclaw/openrustclaw
```

---

## CLI Commands

```bash
openrustclaw start           # Start gateway + sidecar
openrustclaw chat            # Interactive chat
openrustclaw onboard         # Setup wizard
openrustclaw doctor          # Diagnostics
openrustclaw cursor setup    # IDE integration
openrustclaw mcp-server      # MCP server mode
```

---

## Configuration

```toml
# ~/.openrustclaw/config.toml
[gateway]
host = "0.0.0.0"
port = 8080

[providers.anthropic]
api_key = "sk-ant-..."
model = "claude-3-5-sonnet"

[memory]
core_max_tokens = 500
recall_top_k = 10

[sso]
enabled = true
provider = "okta"
```

---

## Development

```bash
# Run tests
cargo test --workspace

# Run E2E tests
cargo test --test e2e_tests smoke

# Build docs
cargo doc --workspace --no-deps

# Format & lint
cargo fmt --all && cargo clippy --workspace
```

---

## Roadmap

### ✅ Completed

**v1.0** — Core Platform
- 19 Rust crates
- 4 LLM providers with fallbacks
- MCP client/server
- 3-tier memory system
- Durable scheduler

**v2.0** — Ecosystem
- Native SDK crates (Anthropic, OpenAI, OpenRouter)
- Telegram/Discord/Slack bots
- Browser automation
- Gemini provider
- Cursor IDE integration
- Distributed mode (Raft)

**v2.1** — Platform Hardening
- Production deployment guides
- Kubernetes Helm charts
- Terraform modules (AWS/GCP/Azure)
- Enterprise SSO (OIDC/SAML)
- NSEW E2E testing framework

### 🚧 In Progress

**v2.2** — Feature Parity
- WhatsApp integration
- Microsoft Teams
- Voice Wake + Talk Mode
- Live Canvas (A2UI)
- Multi-agent routing
- Agent-to-agent communication
- Interactive onboarding
- Chat commands
- Heartbeat scheduler

### 📋 Planned

**v3.0** — Advanced Features
- iOS/Android companion apps
- Device nodes (camera, screen, location)
- ClawHub skills registry
- Webhooks + Gmail Pub/Sub
- Docker sandboxing

---

## Project Structure

```
OpenRustClaw/
├── crates/              # 19 Rust crates
│   ├── core/           # Types, traits, config
│   ├── gateway/        # Axum WebSocket server
│   ├── agent/          # Agent runtime
│   ├── memory/         # 3-tier memory
│   ├── providers/      # LLM providers
│   ├── channels/       # Chat integrations
│   ├── mcp/            # MCP client/server
│   ├── scheduler/      # Durable scheduler
│   ├── security/       # Auth, SSO, sandbox
│   ├── skills/         # Skill system
│   ├── voice/          # 🚧 Voice features
│   ├── canvas/         # 🚧 A2UI workspace
│   └── ...
├── sidecar/            # Python LangGraph
├── deployments/        # Helm, Terraform
├── tests/              # E2E tests
└── docs/               # Documentation
```

---

## Documentation

- [Architecture](docs/src/architecture.md)
- [Deployment](docs/src/deployment/production.md)
- [Security](docs/src/security.md)
- [API Reference](docs/src/api/)
- [Contributing](docs/src/contributing.md)

---

## Community

- [Discord](https://discord.gg/openrustclaw)
- [GitHub Discussions](https://github.com/openrustclaw/openrustclaw/discussions)
- [Twitter/X](https://twitter.com/openrustclaw)

---

## License

MIT License. See [LICENSE](LICENSE) for details.

---

<p align="center">
  <strong>Built with 🦀 Rust + ❤️ by the community</strong>
</p>

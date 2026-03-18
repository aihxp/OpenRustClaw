# OpenRustClaw

[![CI](https://github.com/aihxp/OpenRustClaw/actions/workflows/ci.yml/badge.svg)](https://github.com/aihxp/OpenRustClaw/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-2024_edition-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A high-performance AI agent platform written in Rust. 43 crates, 20 LLM providers, 15 CLI-startable messaging channels, and a Python sidecar for LangGraph workflows.

Current execution planning lives in:

- [docs/feature-matrix.md](docs/feature-matrix.md)
- [docs/reengineering-backlog.md](docs/reengineering-backlog.md)

## Quick Start

```bash
git clone https://github.com/aihxp/OpenRustClaw.git
cd OpenRustClaw
cargo build --release

# Interactive setup -- configures providers, channels, and security
./target/release/openrustclaw onboard

# Start the agent with your configured channels
openrustclaw start --channels=telegram,discord,slack
```

## Architecture

```
                         ┌─────────────────────────┐
                         │          CLI             │
                         └────────────┬─────────────┘
        ┌─────────────┬──────────────┼──────────────┬─────────────┐
        │  Channels   │    Voice     │   Canvas     │   Cursor    │
        │ (15 current)│  Wake/STT/TTS│   A2UI       │  IDE (ACP)  │
        └──────┬──────┴──────┬───────┴──────┬───────┴──────┬──────┘
               └─────────────┴──────────────┴──────────────┘
                         ┌──────────┴──────────┐
                         │   Gateway (Axum WS) │
                         └──────────┬──────────┘
        ┌──────────┬────────────┬───┴───┬────────────┬────────────┐
        │  Agent   │  Memory    │ Skills│ Scheduler  │  Security  │
        │ Runtime  │  3-Tier    │ WASM  │ Durable    │ SSO/JWT    │
        └────┬─────┴─────┬──────┴───┬───┴─────┬──────┴─────┬──────┘
             │           │          │         │            │
        ┌────┴───┐  ┌────┴───┐  ┌──┴──┐  ┌───┴────┐  ┌───┴──────┐
        │Providers│  │   DB   │  │ MCP │  │Langbrdg│  │Observabil│
        │ 20 LLMs │  │SQLite  │  │JSON │  │ gRPC   │  │OTel/Prom │
        └─────────┘  └────────┘  │-RPC │  └───┬────┘  └──────────┘
                                 └─────┘      │
                                    ┌─────────┴─────────┐
                                    │  Python Sidecar   │
                                    │  LangGraph/Smith  │
                                    └───────────────────┘
```

**Crate dependency order:**
`core` -> `db` -> `memory`, `providers`, `mcp`, `observability`, `security` -> `agent` -> `gateway`, `channels` -> `skills`, `scheduler` -> `langbridge` -> `cli`

## LLM Providers

20 providers with native SDK crates. All keys protected with `secrecy::SecretString`, all HTTP clients configured with connect/request timeouts.

| Provider | Crate | Streaming | Tool Use |
|----------|-------|-----------|----------|
| Anthropic (Claude) | `anthropic-rust` | Yes | Yes |
| OpenAI (GPT-4o) | `async-openai` | Yes | Yes |
| Google Gemini | `google-gemini` | Yes | Yes |
| OpenRouter | `openrouter-api` | Yes | Yes |
| AWS Bedrock | `aws-bedrock` | Yes | Yes |
| Azure OpenAI | `azure-openai` | Yes | Yes |
| Ollama (local) | `ollama-sdk` | Yes | Yes |
| Mistral | `mistral` | Yes | Yes |
| Cohere | `cohere` | Yes | Yes |
| Groq | `groq` | Yes | Yes |
| DeepSeek | `deepseek` | Yes | Yes |
| Together AI | `together-ai` | Yes | Yes |
| Fireworks AI | `fireworks-ai` | Yes | Yes |
| Replicate | `replicate` | Yes | Yes |
| Perplexity | `perplexity` | Yes | Yes |
| AI21 | `ai21` | Yes | Yes |
| Cloudflare AI | `cloudflare-ai` | Yes | Yes |
| vLLM | `vllm` | Yes | Yes |
| llama.cpp | `llama-cpp` | Yes | Yes |
| Ollama SDK | `ollama-sdk` | Yes | Yes |

Provider fallback chain with configurable cooldowns routes requests through available providers automatically.

## Messaging Channels

Current channel modules available in the repo:

Telegram, Discord, Slack, WhatsApp (Baileys bridge), Microsoft Teams, Google Chat, Gmail (Pub/Sub), Matrix, iMessage, LINE, Viber, WeChat, Messenger, Instagram DMs, WebChat

The current shipped startup path is narrower: `openrustclaw start` actively supports WebChat, Telegram, Discord, and Slack. Other channel modules remain in the repo but are gated or deferred from the shipped runtime surface.

Current tier-1 status:

- Telegram: auth probe, outbound send, Bot API polling receive, and local agent/session routing are implemented
- Discord: auth probe, outbound send, verified Interactions HTTP ingress, Gateway `MESSAGE_CREATE` receive, basic reconnect handling, and local agent/session routing are implemented; full session resume coverage remains incomplete
- Slack: auth probe, outbound send, built-in HTTP Events API ingress, and local agent/session routing are implemented for HTTP mode; Socket Mode remains incomplete

## Memory System

Three-tier architecture -- no full memory files injected into prompts:

- **Core Memory** (~500 tokens, always loaded) -- persistent user/system facts
- **Recall Memory** (on-demand search) -- hybrid BM25 + vector similarity + temporal decay
- **RAG Context** (budgeted assembly) -- deterministic retrieved context with stable source ids for citations and Rust-backed durable chunk storage
- **Archive Memory** (consolidated) -- long-term storage with Rust-backed maintenance that persists summaries and removes archived recall entries

```toml
[memory]
core_max_tokens = 500
recall_search_limit = 20
archive_after_days = 30
```

## MCP (Model Context Protocol)

Both client and server support over JSON-RPC stdio transport:

```bash
# Expose tools to MCP clients
openrustclaw mcp-server
```

Command allowlist enforced on subprocess spawning (`npx`, `uvx`, `node`, `python3`, `docker`, `deno`, `bun`, `cargo`, `go`).

## Security

Defense-in-depth across every layer:

| Layer | Mechanism |
|-------|-----------|
| **Authentication** | JWT with session tracking, Enterprise SSO (OIDC/SAML) |
| **API Keys** | `secrecy::SecretString` -- zeroized on drop, redacted in logs |
| **Transport** | Origin validation on all WebSocket connections; token auth enabled by default |
| **Webhooks** | HMAC-SHA256 with constant-time comparison, Stripe replay protection |
| **Sessions** | Filesystem isolation with path traversal prevention |
| **Skills** | Workspace and marketplace skill lifecycle is wired through the CLI with signature-state tracking, validated capability metadata, and a no-import WASM sandbox executor with timeout, memory limits, and capability-gated execution helpers |
| **Input** | Prompt injection detection (34+ patterns), canary tokens |
| **Network** | SSRF prevention on OIDC/SAML endpoints (private IP rejection) |
| **Subprocess** | MCP command allowlist, shell metacharacter rejection |
| **Audit** | Structured audit events with severity levels |

See [SECURITY.md](SECURITY.md) for the full security policy.

## Voice

```bash
openrustclaw talk --wake-word "Hey Assistant"
```

Wake word detection (Porcupine), speech-to-text (Whisper), text-to-speech (OpenAI, ElevenLabs), continuous talk mode. Audio dependencies are feature-gated behind `audio`.

## Configuration

```toml
[llm]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[channels]
telegram = { enabled = true, token = "${TG_TOKEN}" }
discord  = { enabled = true, token = "${DISCORD_TOKEN}" }

[security]
require_auth = true
allowed_origins = ["https://app.example.com"]

[voice]
wake_word = "Hey Assistant"
stt_provider = "whisper"
tts_provider = "openai"
```

## Building and Testing

```bash
# Build
cargo build --workspace

# Test (769 tests)
cargo test --workspace

# Lint
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --all -- --check

# Build without optional subsystems
cargo build -p openrustclaw-cli --no-default-features
```

### Feature Flags

Heavy dependencies are opt-in:

| Crate | Feature | Dependencies |
|-------|---------|-------------|
| `distributed` | `raft-consensus`, `etcd`, `consul`, `redis`, `mdns` | Raft, etcd-client, Consul, Redis, mDNS |
| `voice` | `audio` | cpal, rodio, rubato, hound |
| `cli` | `voice`, `cursor` (default on) | Voice subsystem, Cursor IDE integration |
| `mobile` | `ios`, `android` | Platform-specific FFI |
| `automation` | `chrome` | headless_chrome |

## Project Structure

```
crates/
  core/          # Types, traits, error hierarchy (thiserror)
  db/            # SQLite via sqlx, libSQL for vectors, rusqlite for CLI
  memory/        # 3-tier memory system with policies and search
  providers/     # LLM provider trait + Anthropic/OpenAI/Gemini/OpenRouter/Ollama
  mcp/           # MCP client/server over JSON-RPC stdio
  agent/         # Agent runtime with tool execution loop
  gateway/       # Axum WebSocket server with auth, sessions, and webhook integrations
  channels/      # 20 messaging channel integrations
  skills/        # Skill registry, loader, marketplace, and WASM sandbox
  scheduler/     # Durable job scheduling (no cron -- app-owned polling)
  security/      # Auth, SSO, isolation, input sanitization, skill verification
  langbridge/    # gRPC bridge to Python LangGraph sidecar
  observability/ # OpenTelemetry, Prometheus metrics, tracing
  cli/           # Terminal UI (ratatui), all CLI commands
  + 16 native LLM SDK crates
  + canvas, cursor, voice, automation, mobile, distributed
sidecar/         # Python LangGraph workflows + LangSmith observability
```

## Contributing

```bash
git clone https://github.com/aihxp/OpenRustClaw.git
cd OpenRustClaw
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

All async code uses tokio. Errors use `thiserror` for libraries, `anyhow` for the CLI. Logging via `tracing` macros (`info!`, `warn!`, `error!`) -- never `println!` outside the CLI crate. See [CLAUDE.md](CLAUDE.md) for the full conventions guide.

## License

MIT -- see [LICENSE](LICENSE).

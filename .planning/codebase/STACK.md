# Technology Stack

**Analysis Date:** 2026-04-04

## Languages

**Primary:**
- Rust 2024 edition - main workspace language across `Cargo.toml`, `crates/`, `tests/`, and `benches/`

**Secondary:**
- Python 3.11+ - optional LangGraph sidecar in `sidecar/pyproject.toml` and `sidecar/src/`
- JavaScript / Node.js 18+ - WhatsApp bridge only in `crates/channels/baileys-bridge/package.json`

## Runtime

**Environment:**
- Rust stable toolchain with `rustfmt` and `clippy` in `rust-toolchain.toml`
- Rust 1.85 baseline in provider SDK crates such as `crates/anthropic_rust/Cargo.toml`, `crates/async_openai/Cargo.toml`, and `crates/openrouter_api/Cargo.toml`
- Tokio async runtime is the default execution model from workspace deps in `Cargo.toml`
- Python sidecar runs as `python -m src.server` through `crates/langbridge/src/sidecar.rs` and `sidecar/src/server.py`

**Package Manager:**
- Cargo - workspace package manager in `Cargo.toml`
- Lockfile: present in `Cargo.lock`
- Python packaging uses `setuptools` and editable installs from `sidecar/pyproject.toml`
- Python lockfile: missing
- Node package manager is only needed for `crates/channels/baileys-bridge/package.json`

## Frameworks

**Core:**
- Axum 0.8 - HTTP/WebSocket gateway and internal APIs in `Cargo.toml` and `crates/gateway/src/server.rs`
- Tonic 0.12 / Prost 0.13 - Rust↔Python gRPC contract in `Cargo.toml`, `crates/langbridge/Cargo.toml`, and `sidecar/src/proto/`
- LangGraph / LangChain - Python workflow orchestration in `sidecar/pyproject.toml` and `sidecar/src/workflows/`
- Model Context Protocol - JSON-RPC MCP server/client in `crates/mcp/Cargo.toml` and `crates/mcp/src/server.rs`

**Testing:**
- Cargo test - workspace-native testing path in `README.md`, `Makefile`, and `.github/workflows/ci.yml`
- Pytest / pytest-asyncio - sidecar testing in `sidecar/pyproject.toml` and `sidecar/test_sidecar.py`
- Wiremock / Mockito / serial_test / proptest - HTTP, isolation, and property tests from `Cargo.toml`

**Build/Dev:**
- cargo-chef - Docker dependency caching in `Dockerfile`
- Docker / Docker Compose - local and production packaging in `Dockerfile`, `Dockerfile.dev`, `docker-compose.yml`, and `docker-compose.dev.yml`
- GitHub Actions - CI, E2E, and release automation in `.github/workflows/ci.yml`, `.github/workflows/e2e-tests.yml`, and `.github/workflows/release-binaries.yml`
- Ruff and mypy - sidecar linting and typing in `sidecar/pyproject.toml`

## Key Dependencies

**Critical:**
- `tokio` - async runtime used across the workspace in `Cargo.toml`
- `serde`, `serde_json`, `toml`, `config` - config and data serialization in `Cargo.toml` and `crates/core/src/config.rs`
- `axum`, `tower`, `tower-http` - gateway, control routes, and CORS handling in `Cargo.toml` and `crates/gateway/src/server.rs`
- `reqwest` - shared HTTP client layer for provider SDKs, channels, observability, and voice in `Cargo.toml`
- `sqlx` - primary async persistence layer in `Cargo.toml`, `crates/db/src/pool.rs`, and `crates/db/src/session_store.rs`
- `tracing`, `tracing-subscriber`, `metrics` - structured logging and metrics in `Cargo.toml` and `crates/observability/src/tracing_config.rs`
- `jsonwebtoken`, `argon2`, `ed25519-dalek`, `chacha20poly1305` - auth, crypto, and vault/security primitives in `Cargo.toml` and `crates/security/`

**Infrastructure:**
- `openrustclaw-providers` - runtime-wired LLM adapters for Anthropic, OpenAI, OpenRouter, Ollama, and Gemini in `crates/providers/Cargo.toml` and `crates/providers/src/`
- Native provider SDK crates - standalone workspace crates in `crates/anthropic_rust/`, `crates/async_openai/`, `crates/openrouter_api/`, `crates/azure_openai/`, `crates/bedrock/`, `crates/cohere/`, `crates/deepseek/`, `crates/fireworks/`, `crates/groq/`, `crates/mistral/`, `crates/perplexity/`, `crates/replicate/`, `crates/together/`, `crates/vllm/`, `crates/cloudflare_ai/`, `crates/ai21/`, and `crates/llama_cpp/`
- `langsmith`, `opentelemetry`, `opentelemetry-otlp`, `metrics-exporter-prometheus` - tracing export and metrics in `sidecar/pyproject.toml`, `crates/observability/Cargo.toml`, and `crates/gateway/src/metrics_endpoint.rs`
- `tokio-tungstenite` - WebSocket integrations for channels and CDP in `crates/channels/Cargo.toml` and `crates/automation/Cargo.toml`
- `headless_chrome`, Playwright/CDP support - browser automation in `crates/automation/Cargo.toml`

## Configuration

**Environment:**
- Runtime config loads from `config/default.toml` and `OPENRUSTCLAW_...` environment overrides with `__` separators in `crates/core/src/config.rs`
- Example override format: `OPENRUSTCLAW_GATEWAY__PORT=8080` from `crates/core/src/config.rs`
- Environment templates exist as `.env.example` and `.env.docker`
- Workspace secrets can also be stored in the encrypted runtime vault at `.claw/control/runtime-vault.json` via `crates/cli/src/commands/runtime.rs` and `crates/app/src/runtime_vault_control.rs`
- Vault access is gated by `OPENRUSTCLAW_VAULT_PASSPHRASE` in `crates/cli/src/commands/runtime.rs`

**Build:**
- Workspace manifest and shared deps: `Cargo.toml`
- Rust toolchain pinning: `rust-toolchain.toml`
- Sidecar packaging and lint/type/test settings: `sidecar/pyproject.toml`
- Production container image: `Dockerfile`
- Development containers: `Dockerfile.dev` and `docker-compose.dev.yml`
- Production compose stack: `docker-compose.yml`
- Local helper commands: `Makefile`

## Platform Requirements

**Development:**
- Rust stable / 1.85-capable toolchain from `rust-toolchain.toml` and `Dockerfile.dev`
- `protobuf-compiler` for gRPC/proto builds in `Dockerfile`, `Dockerfile.dev`, and `.github/workflows/*.yml`
- `libasound2-dev` for audio-capable builds in `.github/workflows/ci.yml` and `.github/workflows/e2e-tests.yml`
- Python 3.11+ for `sidecar/` from `sidecar/pyproject.toml`
- Node.js 18+ only when using the WhatsApp Baileys bridge from `crates/channels/baileys-bridge/package.json`

**Production:**
- Self-hosted binary or Docker deployment; canonical container target is Debian Bookworm Slim in `Dockerfile`
- Persistent local volume for SQLite data under `/app/data` from `Dockerfile` and `docker-compose.yml`
- Optional reverse proxy / TLS sidecars via `nginx` and `certbot` profiles in `docker-compose.yml`
- GitHub Releases publish tagged multi-arch binaries from `.github/workflows/release-binaries.yml`

---

*Stack analysis: 2026-04-04*

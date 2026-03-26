# Technology Stack

**Analysis Date:** 2026-03-26

## Languages

**Primary:**
- Rust 2024 edition - Main product runtime, CLI, gateway, providers, scheduling, security, channels, skills, voice, automation, and mobile support across the workspace declared in `Cargo.toml`

**Secondary:**
- Python 3 - Optional compatibility sidecar under `sidecar/` for LangGraph-based workflows and gRPC bridging
- TOML / YAML / Markdown / JSON - Runtime config, CI, deployment manifests, docs book, roadmap artifacts, and operator control files
- Shell - Release and quality scripts under `scripts/`

## Runtime

**Environment:**
- Native Rust async runtime built on `tokio` across the workspace
- Optional Python virtualenv sidecar in `sidecar/.venv` for compatibility workflows
- SQLite-backed local persistence via `sqlx`, `rusqlite`, and `libsql`

**Package Manager / Build Tools:**
- Cargo workspace with a single top-level `Cargo.toml` and `Cargo.lock`
- Python packaging via `sidecar/pyproject.toml`
- Protobuf compilation required in CI (`protobuf-compiler` installed in `.github/workflows/ci.yml`)

## Frameworks

**Core:**
- `axum` - HTTP and WebSocket control/gateway surfaces, especially in `crates/gateway/` and large command/control handlers in `crates/cli/src/commands/start.rs`
- `clap` - CLI command tree in `crates/cli/src/main.rs`
- `tokio` / `futures` / `async-trait` - Async execution model across runtime crates
- `tonic` / `prost` - gRPC bridge and protobuf transport for orchestration/runtime integration

**Persistence / State:**
- `sqlx`, `rusqlite`, `libsql` - SQLite and vector-oriented persistence
- File-backed operator state under `.claw/` and generated planning state under `.planning/`

**Observability / Security:**
- `tracing`, `opentelemetry`, `prometheus` - Tracing and metrics
- `ed25519-dalek`, `argon2`, `jsonwebtoken`, `chacha20poly1305`, `wasmtime` - Auth, signing, vault, and WASM sandboxing

**Compatibility / AI Workflows:**
- LangGraph + gRPC Python sidecar documented in `sidecar/README.md`

## Key Dependencies

**Critical runtime dependencies:**
- `tokio` - async runtime used across nearly every crate
- `axum` - gateway/control HTTP surfaces
- `reqwest` - provider and external integration HTTP client layer
- `serde` / `serde_json` / `toml` - config and API serialization
- `sqlx` / `rusqlite` / `libsql` - durable state and memory storage
- `clap` - operator-facing CLI
- `tonic` / `prost` - gRPC interop with the sidecar and distributed components

**Infrastructure-heavy areas:**
- `crates/providers/` plus many provider-specific SDK crates under `crates/*`
- `crates/channels/` for platform integrations
- `crates/security/` and `crates/skills/` for sandboxing and extension execution

## Configuration

**Runtime configuration:**
- Main shipped config in `config/default.toml`
- Example channel/provider configs under `config/*-example.toml`
- Runtime also uses workspace state under `.claw/`

**Build / project configuration:**
- `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`
- `sidecar/pyproject.toml`
- GitHub Actions in `.github/workflows/ci.yml`
- Docker and compose files at repository root plus deployment manifests under `deployments/`

## Platform Requirements

**Development:**
- Rust toolchain with Cargo
- Protobuf compiler for full check/test/CI parity
- Python environment only if using the sidecar compatibility path
- Local SQLite storage under `data/`

**Production / deployment targets:**
- Rust-first runtime is the intended production baseline
- Docker / Compose support via `Dockerfile`, `Dockerfile.dev`, `docker-compose.yml`, and `docker-compose.dev.yml`
- Kubernetes and Terraform support under `deployments/helm/` and `deployments/terraform/`

## High-Signal Paths

- `Cargo.toml`
- `crates/cli/src/main.rs`
- `config/default.toml`
- `crates/gateway/src/lib.rs`
- `sidecar/README.md`
- `.github/workflows/ci.yml`

---
*Stack analysis: 2026-03-26*
*Update after major dependency, runtime, or deployment changes*

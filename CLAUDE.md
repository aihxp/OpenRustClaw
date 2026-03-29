# OpenRustClaw

## Project Structure
Rust workspace with 42 crates in `crates/`. The Python sidecar in `sidecar/` is an optional compatibility lane, not the default production runtime.

## Build
```
cargo build --workspace
```

## Test
```
cargo test --workspace
```

## Key Conventions
- Prefer provider crates and `openrustclaw-providers` abstractions instead of ad hoc raw HTTP in feature code.
- Keep database access routed through the established persistence crates and typed runtime services.
- Memory writes go through the memory-policy layer so dedupe, confidence, and TTL rules stay consistent.
- No cron jobs — use the durable scheduler in `crates/scheduler` and the shipped scheduling surfaces.
- MCP server and tool surfaces live under `crates/mcp`.
- Security-sensitive surfaces should preserve origin validation, token checks, and bounded runtime trust by default.
- 3-tier memory remains the active model: Core → Recall → Archive.
- Avoid injecting full memory files directly into prompts when a bounded memory surface already exists.

## Crate Dependency Order
core → db → memory, providers, mcp, observability, security → agent → gateway → channels → skills, scheduler → langbridge → cli

## Greenfield Transition Defaults
- Prefer `openrustclaw-app` for new application-level business logic.
- Treat `crates/cli/src/commands/start.rs`, `mobile.rs`, `skills.rs`, `runtime.rs`, and `inspect.rs` as compatibility-heavy surfaces unless a task is explicitly about those adapters.
- If a change must touch a large legacy hotspot, keep the logic bounded and preserve verification coverage before broadening scope.

## Error Handling
- thiserror for library errors (crates/core/src/error.rs)
- anyhow for CLI/application errors

## Code Style
- Use tracing macros (info!, warn!, error!) — never println! outside CLI
- All async functions use tokio runtime
- Prefer Arc<dyn Trait> over generics for plugin boundaries

<!-- GSD:project-start source:PROJECT.md -->
## Project

**OpenRustClaw**

OpenRustClaw is a Rust-first version of OpenClaw: a general-purpose AI assistant platform intended to become clean, production-ready, and eventually enterprise-ready. It already contains a broad operator and runtime surface across chat, memory, tools, coding workflows, channels, voice, browser automation, and deployment, and the immediate goal is to turn that breadth into a trustworthy MVP rather than keep expanding unfinished surface area.

The MVP is for solo developers and small operator teams first. It should feel like a real OpenClaw-grade assistant platform that can be installed, configured, trusted, and used end-to-end for assistant chat, coding work, communications, and runtime operations without obvious rough edges.

**Core Value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

### Constraints

- **Tech stack**: Rust-first runtime must remain the primary production path; the Python sidecar remains a bounded compatibility lane, not the main system
- **Brownfield scope**: Prefer hardening and converging existing surfaces over adding new feature families unless a missing contract blocks MVP credibility
- **Reliability**: MVP workflows must be testable, diagnosable, restartable, and inspectable by operators
- **Security**: Auth, secret handling, sandboxing, and origin/runtime trust boundaries must stay on by default for production paths
- **Product focus**: Production-ready MVP comes before enterprise-ready expansion
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->
## Technology Stack

## Languages
- Rust 2024 edition - Main product runtime, CLI, gateway, providers, scheduling, security, channels, skills, voice, automation, and mobile support across the workspace declared in `Cargo.toml`
- Python 3 - Optional compatibility sidecar under `sidecar/` for LangGraph-based workflows and gRPC bridging
- TOML / YAML / Markdown / JSON - Runtime config, CI, deployment manifests, docs book, roadmap artifacts, and operator control files
- Shell - Release and quality scripts under `scripts/`
## Runtime
- Native Rust async runtime built on `tokio` across the workspace
- Optional Python virtualenv sidecar in `sidecar/.venv` for compatibility workflows
- SQLite-backed local persistence via `sqlx`, `rusqlite`, and `libsql`
- Cargo workspace with a single top-level `Cargo.toml` and `Cargo.lock`
- Python packaging via `sidecar/pyproject.toml`
- Protobuf compilation required in CI (`protobuf-compiler` installed in `.github/workflows/ci.yml`)
## Frameworks
- `axum` - HTTP and WebSocket control/gateway surfaces, especially in `crates/gateway/` and large command/control handlers in `crates/cli/src/commands/start.rs`
- `clap` - CLI command tree in `crates/cli/src/main.rs`
- `tokio` / `futures` / `async-trait` - Async execution model across runtime crates
- `tonic` / `prost` - gRPC bridge and protobuf transport for orchestration/runtime integration
- `sqlx`, `rusqlite`, `libsql` - SQLite and vector-oriented persistence
- File-backed operator state under `.claw/` and generated planning state under `.planning/`
- `tracing`, `opentelemetry`, `prometheus` - Tracing and metrics
- `ed25519-dalek`, `argon2`, `jsonwebtoken`, `chacha20poly1305`, `wasmtime` - Auth, signing, vault, and WASM sandboxing
- LangGraph + gRPC Python sidecar documented in `sidecar/README.md`
## Key Dependencies
- `tokio` - async runtime used across nearly every crate
- `axum` - gateway/control HTTP surfaces
- `reqwest` - provider and external integration HTTP client layer
- `serde` / `serde_json` / `toml` - config and API serialization
- `sqlx` / `rusqlite` / `libsql` - durable state and memory storage
- `clap` - operator-facing CLI
- `tonic` / `prost` - gRPC interop with the sidecar and distributed components
- `crates/providers/` plus many provider-specific SDK crates under `crates/*`
- `crates/channels/` for platform integrations
- `crates/security/` and `crates/skills/` for sandboxing and extension execution
## Configuration
- Main shipped config in `config/default.toml`
- Example channel/provider configs under `config/*-example.toml`
- Runtime also uses workspace state under `.claw/`
- `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`
- `sidecar/pyproject.toml`
- GitHub Actions in `.github/workflows/ci.yml`
- Docker and compose files at repository root plus deployment manifests under `deployments/`
## Platform Requirements
- Rust toolchain with Cargo
- Protobuf compiler for full check/test/CI parity
- Python environment only if using the sidecar compatibility path
- Local SQLite storage under `data/`
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
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

## Naming Patterns
- Rust modules use snake_case file names such as `origin_check.rs`, `memory_tools.rs`, `tool_factory.rs`
- Command surfaces are grouped by domain in `crates/cli/src/commands/*.rs`
- Provider/channel/platform modules are usually one file per integration
- Test-heavy modules often keep `mod tests` in the same file rather than splitting unit tests into separate files
- Functions, variables, and module names are snake_case in Rust
- Public structs, enums, and traits use PascalCase
- Constants are UPPER_SNAKE_CASE, for example `DEFAULT_VAULT_PATH` in `crates/cli/src/commands/runtime.rs`
- Traits are used heavily for stable subsystem boundaries, e.g. `LlmProvider`, `Tool`, `MemoryStore`, `Channel` in `crates/core/src/traits.rs`
## Code Style
- `cargo fmt` is enforced in CI via `.github/workflows/ci.yml`
- `cargo clippy -- -D warnings` is also enforced in CI
- Rustdoc-style module comments are common at file tops using `//!`
- Imports are usually grouped with std imports first, then third-party crates, then local crate imports
- CI sets `RUSTFLAGS: "-D warnings"`
- Workspace code tends to compile under a strict warning-free standard
## Import and Module Organization
- `pub mod ...` and `pub use ...` are used to build small public facades, for example in:
## Error Handling
- Use `Result`-returning functions, often with `anyhow::Result` or crate-local result aliases
- Convert errors at CLI or HTTP boundaries instead of panicking in runtime code
- Use serde/typed structs for request and response boundaries
- Test code uses many `unwrap()` / `expect()` calls
- Some source files also contain `expect()` / `unwrap()` in places that deserve care during edits, especially large command modules and SDK wrappers
## Logging and Observability
- `tracing` is the shared logging/tracing path
- OpenTelemetry and Prometheus are first-class observability dependencies
- Operational/runtime surfaces tend to favor structured state over ad hoc stdout
- CLI commands still mix human-readable output with typed JSON-style reporting in some areas
## Comments and Documentation
- File-level `//!` comments are common and usually explain subsystem purpose
- README and docs coverage is broad: root `README.md`, crate READMEs, `docs/src/`, ADRs
- This repo treats docs as part of the maintained surface contract, not just ancillary notes
## Function and Module Design
- Small crates expose narrow public surfaces through `lib.rs`
- Large command surfaces collect many related handlers in one file rather than splitting deeply
- Traits and crate boundaries are the main abstraction strategy; implementation details sit behind them
- Async functions do not use naming prefixes like `async_`; async behavior is inferred from signature
## Testing Conventions
- Unit tests usually live inline under `mod tests`
- Broader behavior is covered by `tests/integration/` and `tests/e2e/`
- `#[tokio::test]` is the dominant async test pattern
- Several crates also maintain their own `tests/integration_tests.rs`
## Practical Guidance for Edits
- Match the existing crate/module split before creating new top-level abstractions
- Prefer extending trait-backed layers over bypassing them directly
- Keep operator/config/runtime paths typed and serializable
- Be cautious when touching very large subsystem files because local conventions may be file-specific
## High-Signal Paths
- `crates/core/src/traits.rs`
- `crates/gateway/src/lib.rs`
- `crates/agent/src/lib.rs`
- `crates/cli/src/main.rs`
- `.github/workflows/ci.yml`
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## Pattern Overview
- Single Cargo workspace coordinating dozens of crates from shared traits upward
- CLI-first control plane with HTTP/WebSocket and control UI surfaces layered on top
- Local durable state through SQLite and file-backed `.claw/` runtime state
- Optional Python sidecar retained for bounded workflow compatibility rather than as the primary runtime
- Heavy emphasis on operator-facing surfaces: runtime inspection, orchestration, channels, browser automation, skills, voice, and mobile node control
## Layers
- Purpose: shared traits, types, config, and persistence primitives
- Contains: `crates/core/`, `crates/db/`, `crates/memory/`, common config in `config/default.toml`
- Depends on: workspace crates and external infra libraries
- Used by: providers, runtime crates, CLI command handlers
- Purpose: LLM/provider integrations and platform-specific capability crates
- Contains: `crates/providers/` plus provider-specific crates, `crates/mcp/`, `crates/skills/`, `crates/security/`, `crates/automation/`, `crates/voice/`, `crates/mobile/`
- Depends on: foundation layer
- Used by: agent runtime, gateway, CLI, orchestration/control surfaces
- Purpose: actual runtime behavior, routing, sessions, channels, orchestration, and operator commands
- Contains: `crates/agent/`, `crates/gateway/`, `crates/channels/`, `crates/scheduler/`, `crates/cli/`, `crates/distributed/`, `crates/cursor/`
- Depends on: foundation + provider/capability layers
- Used by: CLI entrypoint, web/control UI, messaging channels, automation flows
- Purpose: optional workflow interop with Python/LangGraph
- Contains: `sidecar/`, gRPC/proto assets, bridge-facing orchestration contracts
- Depends on: Python environment plus protobuf-generated bindings
- Used by: Rust runtime only when compatibility workflows are enabled
## Data Flow
- Mixed model:
## Key Abstractions
- Purpose: define the stable contracts the workspace composes around
- Examples: `LlmProvider`, `Tool`, `MemoryStore`, `CoreMemoryStore`, `Channel`
- Pattern: trait-first abstraction with crate-specific implementations
- Purpose: organize a very large operator surface into subcommand handlers
- Examples: `crates/cli/src/commands/start.rs`, `runtime.rs`, `skills.rs`, `browser.rs`
- Pattern: large module-per-surface command handlers with shared runtime state wiring
- Purpose: isolate third-party API behavior behind Rust-native types and traits
- Examples: `crates/providers/`, `crates/anthropic_rust/`, `crates/openrouter_api/`
- Pattern: adapter crates plus shared fallback/config plumbing
## Entry Points
- Location: `crates/cli/src/main.rs`
- Triggers: terminal invocation of `openrustclaw`
- Responsibilities: define subcommands and route into command modules
- Locations: `crates/gateway/src/` and `crates/cli/src/commands/start.rs`
- Triggers: HTTP/WebSocket runtime startup and `/control/...` APIs
- Responsibilities: runtime bootstrap, control surfaces, diagnostics, orchestration endpoints, browser/control UI
- Location: `sidecar/src/server.py`
- Triggers: explicit sidecar execution
- Responsibilities: gRPC workflow execution and LangGraph compatibility
## Error Handling
- Shared trait methods return `Result<...>` from crate-level error types
- Axum handlers convert failures into typed JSON/HTTP responses
- Tests use liberal `unwrap()` / `expect()` patterns, especially in integration suites
- Some very large command modules centralize error translation late, which increases local complexity
## Cross-Cutting Concerns
- `tracing` and OpenTelemetry are the common path
- Prometheus/Grafana assets exist for runtime metrics
- Config is strongly structured through TOML + serde-backed Rust types
- Runtime/control commands also expose validate/apply flows
- Auth, origin validation, vaulting, sandboxing, and SSO are handled in dedicated security crates and control/runtime paths
- The repo uses extensive shipped-surface documentation and roadmap artifacts as part of the maintained product contract
<!-- GSD:architecture-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd:quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd:debug` for investigation and bug fixing
- `/gsd:execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd:profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->

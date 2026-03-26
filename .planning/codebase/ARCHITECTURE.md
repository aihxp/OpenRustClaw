# Architecture

**Analysis Date:** 2026-03-26

## Pattern Overview

**Overall:** Rust-first monorepo platform with a large CLI/operator surface, modular provider/integration crates, durable local state, and an optional Python sidecar compatibility lane.

**Key Characteristics:**
- Single Cargo workspace coordinating dozens of crates from shared traits upward
- CLI-first control plane with HTTP/WebSocket and control UI surfaces layered on top
- Local durable state through SQLite and file-backed `.claw/` runtime state
- Optional Python sidecar retained for bounded workflow compatibility rather than as the primary runtime
- Heavy emphasis on operator-facing surfaces: runtime inspection, orchestration, channels, browser automation, skills, voice, and mobile node control

## Layers

**Foundation Layer:**
- Purpose: shared traits, types, config, and persistence primitives
- Contains: `crates/core/`, `crates/db/`, `crates/memory/`, common config in `config/default.toml`
- Depends on: workspace crates and external infra libraries
- Used by: providers, runtime crates, CLI command handlers

**Provider / Capability Layer:**
- Purpose: LLM/provider integrations and platform-specific capability crates
- Contains: `crates/providers/` plus provider-specific crates, `crates/mcp/`, `crates/skills/`, `crates/security/`, `crates/automation/`, `crates/voice/`, `crates/mobile/`
- Depends on: foundation layer
- Used by: agent runtime, gateway, CLI, orchestration/control surfaces

**Runtime / Interaction Layer:**
- Purpose: actual runtime behavior, routing, sessions, channels, orchestration, and operator commands
- Contains: `crates/agent/`, `crates/gateway/`, `crates/channels/`, `crates/scheduler/`, `crates/cli/`, `crates/distributed/`, `crates/cursor/`
- Depends on: foundation + provider/capability layers
- Used by: CLI entrypoint, web/control UI, messaging channels, automation flows

**Compatibility Layer:**
- Purpose: optional workflow interop with Python/LangGraph
- Contains: `sidecar/`, gRPC/proto assets, bridge-facing orchestration contracts
- Depends on: Python environment plus protobuf-generated bindings
- Used by: Rust runtime only when compatibility workflows are enabled

## Data Flow

**CLI / Runtime Execution:**
1. Operator runs `openrustclaw ...` from `crates/cli/src/main.rs`
2. `clap` dispatches into handlers in `crates/cli/src/commands/`
3. Handlers read config (`config/default.toml` or overrides), runtime state under `.claw/`, and persistence backends
4. Provider, channel, memory, scheduler, or control-plane abstractions execute
5. Results return to terminal, HTTP response, WebSocket client, or persisted runtime artifacts

**Gateway / Session Flow:**
1. External client connects through gateway/control surfaces (`crates/gateway/`, large handlers in `crates/cli/src/commands/start.rs`)
2. Session/auth/origin validation is applied
3. Agent runtime loads memory/context and tool availability
4. Provider layer executes completions, optionally calling tools, channels, skills, or memory stores
5. Output is returned to the active client and/or stored in durable state

**Compatibility Sidecar Flow:**
1. Rust runtime chooses compatibility path when enabled
2. gRPC contract communicates with `sidecar/src/server.py`
3. LangGraph workflows perform bounded orchestration/maintenance tasks
4. Results flow back to Rust runtime and persistence/telemetry layers

**State Management:**
- Mixed model:
  - SQLite for runtime/session/memory/job state
  - file-backed operational state under `.claw/`
  - repo docs/config/deployment assets under `docs/`, `deployments/`, and `config/`

## Key Abstractions

**Traits in `crates/core/src/traits.rs`:**
- Purpose: define the stable contracts the workspace composes around
- Examples: `LlmProvider`, `Tool`, `MemoryStore`, `CoreMemoryStore`, `Channel`
- Pattern: trait-first abstraction with crate-specific implementations

**Command Modules:**
- Purpose: organize a very large operator surface into subcommand handlers
- Examples: `crates/cli/src/commands/start.rs`, `runtime.rs`, `skills.rs`, `browser.rs`
- Pattern: large module-per-surface command handlers with shared runtime state wiring

**Provider and Integration Crates:**
- Purpose: isolate third-party API behavior behind Rust-native types and traits
- Examples: `crates/providers/`, `crates/anthropic_rust/`, `crates/openrouter_api/`
- Pattern: adapter crates plus shared fallback/config plumbing

## Entry Points

**Primary CLI Entry:**
- Location: `crates/cli/src/main.rs`
- Triggers: terminal invocation of `openrustclaw`
- Responsibilities: define subcommands and route into command modules

**Gateway / Control Runtime:**
- Locations: `crates/gateway/src/` and `crates/cli/src/commands/start.rs`
- Triggers: HTTP/WebSocket runtime startup and `/control/...` APIs
- Responsibilities: runtime bootstrap, control surfaces, diagnostics, orchestration endpoints, browser/control UI

**Compatibility Sidecar:**
- Location: `sidecar/src/server.py`
- Triggers: explicit sidecar execution
- Responsibilities: gRPC workflow execution and LangGraph compatibility

## Error Handling

**Strategy:** Rust code generally favors `Result`-returning APIs (`anyhow`, crate-local `Result` aliases, `thiserror`) with boundary translation in CLI and HTTP handlers.

**Patterns:**
- Shared trait methods return `Result<...>` from crate-level error types
- Axum handlers convert failures into typed JSON/HTTP responses
- Tests use liberal `unwrap()` / `expect()` patterns, especially in integration suites
- Some very large command modules centralize error translation late, which increases local complexity

## Cross-Cutting Concerns

**Logging / Observability:**
- `tracing` and OpenTelemetry are the common path
- Prometheus/Grafana assets exist for runtime metrics

**Validation / Config:**
- Config is strongly structured through TOML + serde-backed Rust types
- Runtime/control commands also expose validate/apply flows

**Security:**
- Auth, origin validation, vaulting, sandboxing, and SSO are handled in dedicated security crates and control/runtime paths

**Docs-as-contract:**
- The repo uses extensive shipped-surface documentation and roadmap artifacts as part of the maintained product contract

---
*Architecture analysis: 2026-03-26*
*Update when runtime boundaries, control surfaces, or sidecar responsibilities change*

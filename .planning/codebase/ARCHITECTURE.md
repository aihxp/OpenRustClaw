# Architecture

**Analysis Date:** 2026-04-04

## Pattern Overview

**Overall:** Rust-first modular monolith in a Cargo workspace, with a thin Python sidecar behind a gRPC compatibility boundary.

**Key Characteristics:**
- `crates/cli/src/main.rs` and `crates/cli/src/commands/start.rs` act as the main composition root for runtime startup, HTTP routing, channel wiring, scheduler boot, and optional sidecar startup.
- Shared contracts live at the bottom of the dependency graph in `crates/core/src/{config.rs,traits.rs,types.rs,error.rs}` and are consumed across the workspace.
- Persistent state is split between SQLite-backed runtime data in `crates/db/src/*.rs` and file-backed workspace state under `.claw/...`, managed mostly from `crates/cli/src/commands/{control.rs,runtime.rs,channels.rs,skills.rs,onboard.rs}`.

## Layers

**Contracts And Configuration:**
- Purpose: Define the stable types, traits, config schema, and error surface that the rest of the workspace builds on.
- Location: `crates/core/src`
- Contains: `AppConfig` in `crates/core/src/config.rs`, subsystem traits in `crates/core/src/traits.rs`, shared runtime models in `crates/core/src/types.rs`, and shared errors in `crates/core/src/error.rs`.
- Depends on: External serialization and async support only.
- Used by: Every higher-level crate, including `crates/db`, `crates/memory`, `crates/providers`, `crates/gateway`, `crates/agent`, and `crates/cli`.

**Infrastructure And Persistence:**
- Purpose: Own durable storage, memory tiers, provider adapters, MCP transport, security enforcement, and observability wiring.
- Location: `crates/db/src`, `crates/memory/src`, `crates/providers/src`, `crates/mcp/src`, `crates/security/src`, `crates/observability/src`, `crates/langbridge/src`
- Contains: SQLite pool and stores in `crates/db/src/{pool.rs,session_store.rs,memory_store.rs,core_memory_store.rs,rag_store.rs}`, memory policies in `crates/memory/src/policies.rs`, provider failover in `crates/providers/src/fallback.rs`, MCP server/client code in `crates/mcp/src/{server.rs,client.rs}`, origin validation in `crates/security/src/origin_check.rs`, and Rust-sidecar contract code in `crates/langbridge/src/{client.rs,contract.rs,sidecar.rs}`.
- Depends on: `crates/core` plus provider SDK crates such as `crates/async_openai`, `crates/anthropic_rust`, and `crates/openrouter_api`.
- Used by: `crates/agent`, `crates/gateway`, `crates/scheduler`, `crates/cli`, and selected `crates/app` services.

**Execution Engines:**
- Purpose: Execute conversations, tools, schedules, channels, and other runtime workflows.
- Location: `crates/agent/src`, `crates/gateway/src`, `crates/scheduler/src`, `crates/channels/src`, `crates/skills/src`, `crates/automation/src`, `crates/voice/src`, `crates/mobile/src`
- Contains: The LLM/tool loop in `crates/agent/src/runtime.rs`, session transport and Axum routes in `crates/gateway/src/{server.rs,sessions.rs}`, durable job polling in `crates/scheduler/src/worker.rs`, channel implementations in `crates/channels/src/*.rs`, and skill runtime/compilation in `crates/skills/src/{loader.rs,registry.rs,runtime.rs,compiler.rs}`.
- Depends on: Shared contracts and infrastructure crates.
- Used by: `crates/cli` directly, and indirectly by control-plane routes exposed from `crates/cli/src/commands/start.rs`.

**Application Services:**
- Purpose: Hold transport-agnostic use-case logic behind explicit source traits.
- Location: `crates/app/src`
- Contains: Services such as `ControlConfigService` in `crates/app/src/control_config.rs`, `ChannelRegistryLifecycleService` in `crates/app/src/channel_registry_lifecycle.rs`, setup lifecycle helpers in `crates/app/src/setup_lifecycle.rs`, and compiled-skill projection logic in `crates/app/src/compiled_skill_mcp.rs`.
- Depends on: `crates/core`, plus narrowly scoped crates like `crates/mobile` and `crates/skills`.
- Used by: CLI commands and HTTP control routes assembled in `crates/cli/src/commands/start.rs`.

**Delivery And Bootstrap:**
- Purpose: Expose the shipped product through CLI commands, runtime host startup, control routes, and optional compatibility processes.
- Location: `crates/cli/src`, `sidecar/src`
- Contains: Clap command dispatch in `crates/cli/src/main.rs`, command modules in `crates/cli/src/commands/*.rs`, the runtime host in `crates/cli/src/commands/start.rs`, onboarding in `crates/cli/src/commands/onboard.rs`, and the Python gRPC server in `sidecar/src/server.py`.
- Depends on: Nearly the entire Rust workspace plus the optional Python sidecar contract in `crates/langbridge`.
- Used by: Operators, HTTP clients, MCP clients, and CI/test harnesses.

## Data Flow

**Runtime Startup:**

1. `crates/cli/src/main.rs` parses `openrustclaw` subcommands and dispatches `Start` into `crates/cli/src/commands/start.rs`.
2. `crates/cli/src/commands/start.rs` loads `AppConfig` from `config/default.toml`, initializes SQLite through `crates/db/src/pool.rs`, and runs migrations from `crates/db/src/migrate.rs`.
3. The same startup path builds `GatewayState` from `crates/gateway/src/server.rs`, creates `SessionManager` from `crates/gateway/src/sessions.rs`, constructs memory stores from `crates/db/src/{memory_store.rs,core_memory_store.rs,rag_store.rs}`, and optionally starts `SidecarManager` from `crates/langbridge/src/sidecar.rs`.
4. Channel implementations from `crates/channels/src/*.rs`, scheduler workers from `crates/scheduler/src/worker.rs`, and control routers defined inside `crates/cli/src/commands/start.rs` are attached before Axum starts serving.

**Assistant Conversation Flow:**

1. A CLI assistant session or an inbound transport route reaches the runtime through `crates/cli/src/commands/{assistant.rs,start.rs}`.
2. Session identity and history are restored or created via `SessionManager` in `crates/gateway/src/sessions.rs`, backed by `SqliteSessionStore` in `crates/db/src/session_store.rs`.
3. `AgentRuntime` in `crates/agent/src/runtime.rs` builds the system prompt, injects core memory, filters tools, and calls an `LlmProvider` implementation.
4. Provider execution goes through unified adapters in `crates/providers/src/*.rs`; multi-provider failover is coordinated by `ProviderChain` in `crates/providers/src/fallback.rs`.
5. Tool calls are executed through `ToolRegistry` in `crates/agent/src/tools.rs`; memory-related tools use the recall/core stores behind `crates/core/src/traits.rs`.
6. New conversation state is persisted through `crates/db/src/session_store.rs`, while long-lived memory writes are expected to pass through policy checks in `crates/memory/src/policies.rs`.

**Scheduled And Background Work:**

1. `crates/cli/src/commands/start.rs` starts `SchedulerWorker` from `crates/scheduler/src/worker.rs`.
2. The worker polls SQLite for due jobs and event dispatches using `crates/scheduler/src/persistence.rs`.
3. Rust-native workflows dispatch through `WorkflowDispatcher` implementations in `crates/scheduler/src/workflow.rs`.
4. Compatibility-only workflows can cross the gRPC boundary using `WorkflowInvocation` in `crates/langbridge/src/contract.rs`, `LangBridgeClient` in `crates/langbridge/src/client.rs`, and workflow handlers in `sidecar/src/server.py`.

**Control-Plane Mutation Flow:**

1. HTTP control routes are assembled inside `crates/cli/src/commands/start.rs`.
2. Route handlers call application services from `crates/app/src/*.rs` instead of mutating files directly.
3. Those services delegate to source traits implemented by CLI-owned adapters in files such as `crates/cli/src/commands/{control.rs,runtime.rs,channels.rs,skills.rs}`.
4. Mutations land in file-backed manifests under `.claw/control`, `.claw/channels`, `.claw/skills`, `.claw/browser`, `.claw/mobile`, and related workspace-local paths.

**State Management:**
- SQLite is the main durable runtime store. The active entry point is `config/default.toml`, and the default database path is `data/openrustclaw.db`.
- Conversation/session state is stored through `crates/db/src/session_store.rs`.
- Recall/core/archive memory state is managed through `crates/db/src/{memory_store.rs,core_memory_store.rs,rag_store.rs}` plus policy code in `crates/memory/src`.
- Operator state, runtime beacons, model profiles, task manifests, browser artifacts, and onboarding state are file-backed under `.claw/...`, with path owners in `crates/cli/src/commands/{control.rs,runtime.rs,channels.rs,onboard.rs,memory.rs,schedule.rs}`.

## Key Abstractions

**Subsystem Traits:**
- Purpose: Keep crate boundaries explicit and allow runtime composition without transport coupling.
- Examples: `LlmProvider`, `Tool`, `MemoryStore`, `CoreMemoryStore`, and `Channel` in `crates/core/src/traits.rs`
- Pattern: Stable trait contracts at the bottom of the graph with concrete adapters above them.

**Application Service + Source Trait:**
- Purpose: Separate use-case logic from filesystem, config, and transport adapters.
- Examples: `ControlConfigService` and `ControlConfigSource` in `crates/app/src/control_config.rs`; `ChannelRegistryLifecycleService` and `ChannelRegistryLifecycleSource` in `crates/app/src/channel_registry_lifecycle.rs`
- Pattern: Service object holds decision logic; caller-supplied source trait performs mutation or loading.

**Agent Runtime:**
- Purpose: Own the message -> prompt -> provider -> tool loop.
- Examples: `AgentRuntime` in `crates/agent/src/runtime.rs`, prompt assembly in `crates/agent/src/prompt.rs`, and tool registration in `crates/agent/src/tools.rs`
- Pattern: Single runtime object composed from a provider, tool registry, optional memory stores, and workspace context.

**Provider Chain:**
- Purpose: Hide multi-provider retry and cooldown behavior behind one LLM surface.
- Examples: `ProviderChain` in `crates/providers/src/fallback.rs`
- Pattern: Ordered adapter chain with per-provider cooldown state.

**Rust-Sidecar Workflow Contract:**
- Purpose: Keep Python workflow execution behind an explicit serialization boundary.
- Examples: `WorkflowInvocation` in `crates/langbridge/src/contract.rs`, `LangBridgeClient` in `crates/langbridge/src/client.rs`, `parse_workflow_request` in `sidecar/src/workflow_contract.py`
- Pattern: Typed Rust request -> protobuf/gRPC -> typed Python normalization -> LangGraph execution.

## Entry Points

**Primary CLI:**
- Location: `crates/cli/src/main.rs`
- Triggers: `cargo run --bin openrustclaw -- ...`
- Responsibilities: Parse subcommands, initialize tracing, and dispatch into command modules under `crates/cli/src/commands`.

**Runtime Host:**
- Location: `crates/cli/src/commands/start.rs`
- Triggers: `openrustclaw start`
- Responsibilities: Load config, initialize storage, start Axum, wire channels, register control routes, boot the scheduler, and optionally spawn the sidecar.

**Onboarding Flow:**
- Location: `crates/cli/src/commands/onboard.rs`
- Triggers: `openrustclaw onboard`
- Responsibilities: Guide initial setup, write durable setup state, and prepare control-plane artifacts.

**Python Workflow Sidecar:**
- Location: `sidecar/src/server.py`
- Triggers: `SidecarManager` in `crates/langbridge/src/sidecar.rs` or manual `python -m src.server`
- Responsibilities: Accept gRPC workflow requests, normalize contract data, execute LangGraph workflows from `sidecar/src/workflows`, and report status back to Rust.

**MCP Server Surface:**
- Location: `crates/mcp/src/server.rs`
- Triggers: `openrustclaw mcp-server`
- Responsibilities: Expose workspace tools through MCP JSON-RPC over stdio or SSE-adjacent transport owned by CLI startup code.

## Error Handling

**Strategy:** Shared library crates return `openrustclaw_core::error::Result`, while the CLI layer uses `anyhow::Result` to add command-context and terminate gracefully.

**Patterns:**
- Low-level crates map external failures into typed domain errors, for example in `crates/db/src/pool.rs`, `crates/providers/src/openai.rs`, and `crates/langbridge/src/client.rs`.
- Transport boundaries translate errors into protocol responses, such as HTTP status mapping in `crates/gateway/src/server.rs` and JSON-RPC errors in `crates/mcp/src/server.rs`.
- Long-running background work isolates and records failures, for example dead-letter handling and retry scheduling in `crates/scheduler/src/worker.rs`.

## Cross-Cutting Concerns

**Logging:** `tracing` is the standard path, initialized from `crates/cli/src/main.rs` via `openrustclaw_observability::init_tracing` and used throughout runtime crates.

**Validation:** Config and request validation is distributed across composition points such as `crates/cli/src/commands/start.rs`, application services in `crates/app/src/*.rs`, and contract helpers like `crates/langbridge/src/contract.rs` and `sidecar/src/workflow_contract.py`.

**Authentication:** Gateway auth and origin checks are enforced in `crates/gateway/src/{auth.rs,server.rs}` using `OriginValidator` from `crates/security/src/origin_check.rs`; control routes add additional bearer/trusted-proxy protection in `crates/cli/src/commands/start/auth.rs`.

---

*Architecture analysis: 2026-04-04*

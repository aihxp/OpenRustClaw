# Codebase Structure

**Analysis Date:** 2026-04-04

## Directory Layout

```text
OpenRustClaw/
├── crates/                  # Rust workspace crates: core contracts, runtime engines, adapters, delivery surfaces
├── sidecar/                 # Optional Python LangGraph/LangSmith compatibility lane
├── tests/                   # Workspace-level integration and E2E test crates
├── benches/                 # Criterion benchmark crate
├── config/                  # Checked-in runtime configuration templates
├── docs/                    # mdBook sources and canonical product/docs references
├── proto/                   # Shared protobuf definitions used by `crates/langbridge` and `sidecar/`
├── scripts/                 # Repo automation and maintenance scripts
├── .planning/               # GSD project state, milestones, and codebase maps
├── Cargo.toml               # Workspace manifest and crate membership
└── README.md                # Product-level operator and contributor entry point
```

## Directory Purposes

**`crates/core`:**
- Purpose: Lowest-level shared contracts.
- Contains: `crates/core/src/{config.rs,error.rs,traits.rs,types.rs}`
- Key files: `crates/core/src/traits.rs`, `crates/core/src/config.rs`

**`crates/app`:**
- Purpose: Transport-agnostic application services and use-case boundaries.
- Contains: One service file per use case, such as `crates/app/src/control_config.rs`, `crates/app/src/channel_registry_lifecycle.rs`, and `crates/app/src/setup_lifecycle.rs`
- Key files: `crates/app/src/lib.rs`, `crates/app/src/compiled_skill_mcp.rs`, `crates/app/src/runtime_vault_control.rs`

**`crates/cli`:**
- Purpose: Primary product entry point and composition root.
- Contains: `crates/cli/src/main.rs` plus subcommand modules under `crates/cli/src/commands`
- Key files: `crates/cli/src/main.rs`, `crates/cli/src/commands/start.rs`, `crates/cli/src/commands/onboard.rs`, `crates/cli/src/commands/runtime.rs`, `crates/cli/src/commands/control.rs`

**`crates/agent`:**
- Purpose: Conversation runtime, tool execution loop, routing, and prompt assembly.
- Contains: `crates/agent/src/{runtime.rs,prompt.rs,tools.rs,routing.rs,memory_tools.rs}`
- Key files: `crates/agent/src/runtime.rs`, `crates/agent/src/tools.rs`

**`crates/gateway`:**
- Purpose: Axum HTTP/WebSocket delivery layer and session coordination.
- Contains: `crates/gateway/src/{server.rs,sessions.rs,auth.rs,health.rs,metrics_endpoint.rs}`
- Key files: `crates/gateway/src/server.rs`, `crates/gateway/src/sessions.rs`

**`crates/scheduler`:**
- Purpose: Durable job polling, retry logic, event dispatch, and workflow dispatching.
- Contains: `crates/scheduler/src/{worker.rs,workflow.rs,persistence.rs,jobs.rs,triggers.rs}`
- Key files: `crates/scheduler/src/worker.rs`, `crates/scheduler/src/workflow.rs`

**`crates/db`:**
- Purpose: SQLite-backed persistence and migrations.
- Contains: Stores and migration helpers under `crates/db/src`, SQL migrations under `crates/db/migrations`
- Key files: `crates/db/src/pool.rs`, `crates/db/src/session_store.rs`, `crates/db/src/memory_store.rs`

**`crates/memory`:**
- Purpose: Memory tier orchestration, policies, search, and context shaping.
- Contains: `crates/memory/src/{policies.rs,context.rs,core_memory.rs,recall.rs,rag.rs}`
- Key files: `crates/memory/src/policies.rs`, `crates/memory/src/context.rs`

**`crates/providers`:**
- Purpose: Unified LLM provider adapters used by the runtime.
- Contains: One adapter per provider in `crates/providers/src/{anthropic.rs,openai.rs,openrouter.rs,ollama.rs,gemini.rs}`, plus failover in `crates/providers/src/fallback.rs`
- Key files: `crates/providers/src/lib.rs`, `crates/providers/src/fallback.rs`

**`crates/<provider-sdk>` crates:**
- Purpose: Native or provider-specific client crates used below `crates/providers`.
- Contains: Standalone crates such as `crates/async_openai`, `crates/anthropic_rust`, `crates/openrouter_api`, `crates/ai21`, `crates/replicate`, and `crates/llama_cpp`
- Key files: `crates/async_openai/src/lib.rs`, `crates/openrouter_api/src/lib.rs`, `crates/ai21/src/lib.rs`

**`crates/channels`:**
- Purpose: Messaging platform adapters.
- Contains: One file per platform under `crates/channels/src/*.rs` and the Baileys WhatsApp bridge under `crates/channels/baileys-bridge`
- Key files: `crates/channels/src/lib.rs`, `crates/channels/src/webchat.rs`, `crates/channels/src/telegram.rs`, `crates/channels/src/discord.rs`

**`crates/skills`:**
- Purpose: Skill loading, registry, compilation, sandboxing, and runtime execution.
- Contains: `crates/skills/src/{loader.rs,registry.rs,compiler.rs,runtime.rs,sandbox.rs}`
- Key files: `crates/skills/src/lib.rs`, `crates/skills/src/compiler.rs`, `crates/skills/src/runtime.rs`

**`crates/mcp`, `crates/mcp2cli`, `crates/cursor`, `crates/canvas`:**
- Purpose: External tool and UI protocol surfaces.
- Contains: MCP server/client code, adaptive CLI discovery, Cursor ACP integration, and live canvas support
- Key files: `crates/mcp/src/server.rs`, `crates/mcp2cli/src/lib.rs`, `crates/cursor/src/lib.rs`, `crates/canvas/src/lib.rs`

**`crates/langbridge`:**
- Purpose: Rust-side bridge to the Python sidecar.
- Contains: gRPC client, workflow contract helpers, protobuf bindings, and sidecar process management
- Key files: `crates/langbridge/src/{client.rs,contract.rs,sidecar.rs,lib.rs}`

**`crates/security` and `crates/observability`:**
- Purpose: Shared cross-cutting crates.
- Contains: Origin/auth/input protection and tracing/metrics/LangSmith helpers
- Key files: `crates/security/src/origin_check.rs`, `crates/observability/src/lib.rs`

**`crates/automation`, `crates/voice`, `crates/mobile`, `crates/distributed`, `crates/optimization`:**
- Purpose: Optional or specialized runtime capabilities layered on top of the shared core.
- Contains: Browser automation, voice processing, mobile node support, distributed runtime pieces, and bounded optimization workflows
- Key files: `crates/automation/src/lib.rs`, `crates/voice/src/lib.rs`, `crates/mobile/src/lib.rs`, `crates/distributed/src/lib.rs`, `crates/optimization/src/lib.rs`

**`sidecar`:**
- Purpose: Optional Python workflow runtime.
- Contains: `sidecar/src/server.py`, workflow definitions in `sidecar/src/workflows`, generated gRPC stubs in `sidecar/src/proto`, and evaluators in `sidecar/src/evaluators`
- Key files: `sidecar/src/server.py`, `sidecar/src/workflow_contract.py`, `sidecar/src/workflows/agent_orchestrator.py`, `sidecar/pyproject.toml`

**`tests`:**
- Purpose: Workspace-level test crates beyond crate-local unit tests.
- Contains: Integration tests in `tests/integration/src` and E2E suites in `tests/e2e/{src,tests}`
- Key files: `tests/integration/src/agent_runtime_test.rs`, `tests/e2e/tests/e2e_tests.rs`, `tests/e2e/src/main.rs`

**`config`:**
- Purpose: Default and example runtime configuration.
- Contains: `config/default.toml` plus channel-specific examples
- Key files: `config/default.toml`, `config/channels-example.toml`

## Key File Locations

**Entry Points:**
- `crates/cli/src/main.rs`: Main `openrustclaw` binary
- `crates/cli/src/commands/start.rs`: Runtime bootstrap and Axum/router composition
- `crates/cli/src/commands/onboard.rs`: Interactive setup flow
- `sidecar/src/server.py`: Python gRPC server entry point
- `tests/e2e/src/main.rs`: Standalone E2E test runner binary

**Configuration:**
- `Cargo.toml`: Workspace crate membership and shared dependency versions
- `config/default.toml`: Default runtime config checked into the repo
- `crates/core/src/config.rs`: Rust config schema corresponding to `config/default.toml`
- `sidecar/pyproject.toml`: Python sidecar dependency and tooling config

**Core Logic:**
- `crates/core/src/traits.rs`: Core subsystem interfaces
- `crates/agent/src/runtime.rs`: Agent execution loop
- `crates/gateway/src/server.rs`: HTTP/WebSocket routes and gateway state
- `crates/scheduler/src/worker.rs`: Durable scheduler poll loop
- `crates/db/src/session_store.rs`: Conversation/session persistence
- `crates/memory/src/policies.rs`: Memory write rules
- `crates/providers/src/fallback.rs`: Multi-provider failover logic

**Testing:**
- `tests/integration/src`: Cross-crate integration tests
- `tests/e2e/tests`: Smoke, regression, horizontal, and vertical E2E suites
- `crates/*/tests`: Crate-local integration tests where present, such as `crates/automation/tests` and `crates/ai21/tests`

## Naming Conventions

**Files:**
- Rust source files are snake_case and usually map one file to one concept, for example `crates/app/src/control_config.rs`, `crates/gateway/src/server.rs`, and `crates/providers/src/openai.rs`.
- CLI command files match the subcommand domain under `crates/cli/src/commands`, for example `start.rs`, `onboard.rs`, `memory.rs`, and `schedule.rs`.
- Provider-specific low-level crates use lowercase or snake_case crate directories, for example `crates/anthropic_rust`, `crates/openrouter_api`, and `crates/llama_cpp`.

**Directories:**
- Top-level runtime subsystems live as peer crates under `crates/`.
- Test layers are separated by scope: `tests/integration` for cross-crate tests and `tests/e2e` for end-to-end scenarios.
- Python sidecar code stays under `sidecar/src` and mirrors runtime concerns with subdirectories like `sidecar/src/workflows` and `sidecar/src/evaluators`.

## Where to Add New Code

**New Feature:**
- Primary code: Put transport-agnostic use-case logic in `crates/app/src/<feature>.rs` when the behavior will be shared by CLI and HTTP/control surfaces.
- Tests: Put cross-crate behavior tests in `tests/integration/src/<feature>_test.rs`; use `tests/e2e/tests/...` only when the feature exercises the full runtime.

**New Delivery Surface Or Operator Command:**
- Implementation: Add a command module in `crates/cli/src/commands/<feature>.rs`, export it from `crates/cli/src/commands/mod.rs`, and wire the subcommand in `crates/cli/src/main.rs`.

**New Gateway Or Control Route:**
- Implementation: Add the handler and router assembly inside `crates/cli/src/commands/start.rs` if it is part of the live runtime host.
- Shared route-facing business logic: Push reusable decision logic into `crates/app/src/<feature>.rs` instead of keeping it inline in `start.rs`.

**New Runtime Module:**
- Agent/tool behavior: `crates/agent/src`
- Scheduler/job behavior: `crates/scheduler/src`
- Channel adapter: `crates/channels/src/<platform>.rs`
- Memory logic: `crates/memory/src`
- Persistence: `crates/db/src`

**New Provider:**
- Native SDK or low-level client: Create `crates/<provider>/`
- Runtime adapter used by the assistant: Add `crates/providers/src/<provider>.rs`
- Config shape: Extend `crates/core/src/config.rs` and `config/default.toml`

**New Sidecar Workflow:**
- Rust-side contract or process management: `crates/langbridge/src`
- Python workflow implementation: `sidecar/src/workflows/<workflow>.py`
- Shared request normalization: `sidecar/src/workflow_contract.py`

**Utilities:**
- Shared contracts: `crates/core/src`
- Shared workspace-wide scripts: `scripts/`
- Avoid adding domain logic to `crates/cli/src/commands` when it can live lower in `crates/app` or another subsystem crate.

## Special Directories

**`.planning/`:**
- Purpose: Project state, milestones, and planning artifacts
- Generated: Yes
- Committed: Yes

**`.planning/codebase/`:**
- Purpose: Generated codebase reference documents consumed by later GSD commands
- Generated: Yes
- Committed: Yes

**`.claw/`:**
- Purpose: Workspace-local runtime artifacts such as control state, channel registries, setup manifests, browser artifacts, task manifests, and runtime beacons referenced from `crates/cli/src/commands/{control.rs,runtime.rs,onboard.rs,channels.rs,browser.rs,schedule.rs}`
- Generated: Yes
- Committed: No

**`crates/db/migrations/`:**
- Purpose: SQL migrations for the SQLite runtime store
- Generated: No
- Committed: Yes

**`sidecar/src/proto/`:**
- Purpose: Generated Python protobuf bindings used by the sidecar server
- Generated: Yes
- Committed: Yes

**`proto/`:**
- Purpose: Source protobuf definitions used to generate bridge code
- Generated: No
- Committed: Yes

---

*Structure analysis: 2026-04-04*

# Codebase Structure

**Analysis Date:** 2026-03-26

## Top-Level Layout

- `Cargo.toml` - Workspace root for the Rust monorepo
- `crates/` - Main Rust product/workflow/provider/runtime crates
- `tests/` - Workspace-level integration and E2E crates
- `sidecar/` - Optional Python compatibility sidecar
- `config/` - Default and example runtime configuration
- `deployments/` - Helm and Terraform deployment assets
- `docs/` - mdBook/user/operator/architecture docs
- `scripts/` - build, audit, and budget scripts
- `proto/` - shared protobuf contracts
- `.claw/` - workspace runtime/operator state
- `.codex/` - local skill and workflow tooling added for GSD orchestration

## Key Rust Workspace Areas

**Core platform crates:**
- `crates/core/` - shared types, traits, and errors
- `crates/db/` - persistence and migrations
- `crates/memory/` - recall/core memory, artifact registry, RAG helpers
- `crates/providers/` - provider abstraction and fallback chain
- `crates/agent/` - runtime and tool registry composition
- `crates/gateway/` - gateway server, sessions, metrics, webhooks
- `crates/scheduler/` - durable scheduling and retry behavior
- `crates/security/` - auth, sanitization, isolation, SSO
- `crates/skills/` - skill loading, marketplace, compilation, sandboxing

**Operator / interaction surfaces:**
- `crates/cli/` - main CLI and large operator command set
- `crates/channels/` - platform integrations
- `crates/automation/` - browser/page/media-oriented automation tools
- `crates/voice/` - STT/TTS/talk mode
- `crates/mobile/` - mobile node and capability surfaces
- `crates/cursor/` - Cursor integration support
- `crates/distributed/` - distributed/node-oriented runtime pieces

**Provider-specific crates:**
- Many crates under `crates/` are direct provider SDK wrappers, for example `anthropic_rust`, `async_openai`, `azure_openai`, `bedrock`, `cohere`, `fireworks`, `gemini`, `groq`, `ollama_sdk`, `openrouter_api`, `replicate`, `together`, `vllm`

## Important Entry Paths

- `crates/cli/src/main.rs` - top-level CLI command registration
- `crates/cli/src/commands/` - the largest concentration of operator behavior
- `crates/cli/src/commands/start.rs` - massive control/gateway/orchestration/browser runtime surface
- `crates/cli/src/commands/skills.rs` - large skills/control-plane command surface
- `crates/gateway/src/server.rs` - gateway runtime implementation
- `sidecar/src/server.py` - Python sidecar gRPC entrypoint

## Test Layout

- `tests/integration/` - workspace integration crate
- `tests/e2e/` - workspace end-to-end crate
- crate-local `mod tests` blocks exist throughout many Rust modules
- crate-specific integration tests exist for several SDK crates, e.g.:
  - `crates/anthropic_rust/tests/integration_tests.rs`
  - `crates/cursor/tests/integration_tests.rs`
  - `crates/distributed/tests/integration_tests.rs`
  - `crates/automation/tests/integration_tests.rs`
  - `crates/mcp2cli/tests/integration_tests.rs`

## Documentation / Planning Layout

- `docs/src/` - mdBook source
- `docs/book/` - generated book output
- `docs/adr/` - architecture decision records
- `docs/roadmap.md`, `docs/feature-matrix.md`, `docs/surface-matrix.md`, `docs/product-positioning.md` - maintained shipped-surface docs
- `.planning/codebase/` - generated brownfield codebase map created by GSD workflows

## Naming Patterns

- Crate directories are snake_case and mirror crate purpose
- Command modules under `crates/cli/src/commands/` are one file per major surface
- Channel/provider implementations are usually one file per platform/provider
- Generated/runtime state prefers explicit directory roots (`.claw/`, `.planning/`, `data/`)

## Notable Structural Characteristics

- This is a wide monorepo, not a small app: `Cargo.toml` declares more than 40 workspace members
- Some command/integration files are extremely large and act as subsystem hubs:
  - `crates/cli/src/commands/start.rs` (~12.8k lines)
  - `crates/cli/src/commands/skills.rs` (~5.5k lines)
  - `crates/channels/src/discord.rs` (~3.0k lines)
  - `crates/channels/src/teams.rs` (~2.6k lines)
- The Python sidecar directory currently includes local environment/cache artifacts:
  - `sidecar/.venv`
  - `sidecar/.pytest_cache`
  - `sidecar/__pycache__`
  - `sidecar/src/__pycache__`

## High-Signal Paths

- `Cargo.toml`
- `crates/cli/src/commands/`
- `crates/gateway/src/`
- `crates/security/src/`
- `crates/skills/src/`
- `tests/`
- `docs/src/`

---
*Structure analysis: 2026-03-26*
*Update when top-level directories or subsystem boundaries change*

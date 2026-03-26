# Codebase Concerns

**Analysis Date:** 2026-03-26

## Tech Debt

**Oversized command hubs:**
- Issue: Several operator/runtime files act as very large multi-feature aggregation points
- Evidence:
  - `crates/cli/src/commands/start.rs` ~12.9k lines
  - `crates/cli/src/commands/skills.rs` ~5.5k lines
  - `crates/channels/src/discord.rs` ~3.0k lines
  - `crates/channels/src/teams.rs` ~2.6k lines
- Impact: Higher regression risk, harder navigation, more coupling between unrelated changes
- Fix approach: Gradually split by bounded subdomain while preserving public command/API contracts

**Rust-first runtime plus compatibility sidecar:**
- Issue: The repo still carries a Python sidecar alongside the Rust-first runtime
- Evidence: `sidecar/README.md`, `sidecar/src/server.py`, protobuf/gRPC assets
- Impact: Contract drift risk between Rust-owned behavior and optional compatibility workflows
- Fix approach: Keep protocol ownership explicit and prefer contract tests whenever Rust and Python paths overlap

## Known Bugs / Risk Signals

**CI references missing parity docs:**
- Symptoms: `.github/workflows/ci.yml` expects `docs/parity-matrix.md` and `docs/parity-positioning.md`, but those files are absent in this checkout
- Trigger: Running the `parity-inventory` job as currently written
- Workaround: Restore or rename the expected docs, or update the CI contract to current filenames
- Root cause: Documentation/CI contract appears out of sync

## Security Considerations

**Secret-heavy config surface:**
- Risk: Many provider/channel/auth integrations require tokens, webhook secrets, and vault-managed credentials
- Evidence: `config/default.toml`, `crates/security/`, runtime vault commands, large channel/provider coverage
- Current mitigation: dedicated security crate, encrypted runtime vault, origin/auth controls, sandboxing
- Recommendations: be careful not to mirror live secrets into docs, fixtures, generated planning artifacts, or checked-in config copies

**Extension and external backend breadth:**
- Risk: Skills, MCP bridges, browser tooling, and external backend allowlists widen the trusted execution boundary
- Evidence: `crates/skills/`, `crates/mcp/`, `crates/automation/`, `[external_backends]` in `config/default.toml`
- Current mitigation: explicit allowlists, sandboxing, security modules
- Recommendations: treat these paths as security-sensitive during refactors and verify policy enforcement when adding new execution lanes

## Performance Bottlenecks

**Command/control compilation surface:**
- Problem: The workspace is large and operator surfaces are concentrated in giant modules
- Cause: broad feature scope plus many crates and integrations in one workspace
- Impact: slower edit/compile/test cycles and harder local reasoning
- Improvement path: continue decomposing large command modules and use narrower integration boundaries where possible

## Fragile Areas

**Control/runtime surface in `start.rs`:**
- Why fragile: combines runtime startup, control APIs, orchestration endpoints, browser controls, and UI-serving behavior
- Common failures: small edits can affect unrelated runtime/control paths
- Safe modification: isolate the exact handler/state path first and verify via targeted tests before broad refactors
- Test coverage: some tests exist in the file, but breadth of behavior still makes it high-risk

**Channel implementations:**
- Why fragile: each platform file is large and protocol-specific
- Common failures: transport/auth/schema changes in one platform adapter can be subtle and hard to simulate locally
- Safe modification: prefer localized edits and reuse existing test scaffolding where present

## Dependency / Workspace Hygiene Risks

**Local environment artifacts inside the repo tree:**
- Evidence:
  - `sidecar/.venv`
  - `sidecar/.pytest_cache`
  - `sidecar/__pycache__`
  - `sidecar/src/__pycache__`
- Risk: noise during searches, accidental coupling to local machine state, confusion about what is source vs generated/local
- Recommendation: keep tooling aware of tracked vs local artifacts and avoid using these as canonical inputs

## Test Coverage Gaps

**Default CI scope is narrower than total feature surface:**
- What's not fully covered: some optional feature paths, examples, sidecar behavior, and the full breadth of external integrations
- Risk: large changes can appear healthy under default Rust lib CI while leaving compatibility or non-lib paths unverified
- Priority: High for cross-runtime or cross-integration refactors
- Mitigation: add targeted integration tests when touching shared contracts

## High-Signal Paths

- `.github/workflows/ci.yml`
- `crates/cli/src/commands/start.rs`
- `crates/cli/src/commands/skills.rs`
- `crates/channels/src/discord.rs`
- `crates/channels/src/teams.rs`
- `sidecar/src/server.py`
- `config/default.toml`

---
*Concerns audit: 2026-03-26*
*Update as issues are fixed or newly discovered*

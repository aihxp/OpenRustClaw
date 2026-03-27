# Phase 37: Codebase Cleanup Inventory and Refactor Contract - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning and execution
**Mode:** Autonomous cleanup discovery

<domain>
## Phase Boundary

This phase turns cleanup into an explicit brownfield contract. The goal is not to refactor the whole repo yet, but to identify the canonical cleanup targets, document no-touch boundaries, and choose the first bounded cleanup slices for later phases.

</domain>

<decisions>
## Implementation Decisions

### inventory before code movement
Cleanup should start with a maintained inventory instead of ad hoc "clean as you go" edits. The repo is already broad enough that unsourced cleanup would create avoidable regressions.

### prioritize real drift and one safe refactor slice
The first concrete targets are:
- stale CI references to deleted parity docs
- sidecar local/generated artifacts as non-canonical repo noise
- a cohesive `start.rs` auth and middleware extraction rather than a broad uncontrolled split

### preserve shipped contracts
This milestone must not weaken enterprise auth, origin validation, setup behavior, or the shipped documentation contract while cleaning internals.

</decisions>

<code_context>
## Existing Code Insights

- `.planning/codebase/CONCERNS.md` already calls out oversized command hubs, CI drift, and local-environment artifact noise.
- `.github/workflows/ci.yml` still expects `docs/parity-matrix.md` and `docs/parity-positioning.md`, while the canonical docs are now `docs/feature-matrix.md`, `docs/surface-matrix.md`, and `docs/product-positioning.md`.
- `crates/cli/src/commands/start.rs` is now over 14k lines and contains a cohesive control-auth and enterprise-access middleware slice that can be extracted without changing operator-facing behavior.

</code_context>

<specifics>
## Specific Ideas

- add one canonical cleanup inventory file under `.planning/codebase/`
- make no-touch vs extra-care boundaries explicit
- target CI/docs drift in Phase 38
- target `start.rs` middleware extraction in Phase 39

</specifics>

<deferred>
## Deferred Ideas

- full `skills.rs` decomposition
- deeper Discord or Teams adapter splitting
- sidecar protocol simplification beyond the cleanup contract

</deferred>

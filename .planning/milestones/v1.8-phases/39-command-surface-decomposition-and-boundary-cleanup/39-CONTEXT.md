# Phase 39: Command Surface Decomposition and Boundary Cleanup - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning and execution
**Mode:** Autonomous structural cleanup

<domain>
## Phase Boundary

This phase performs one bounded structural refactor instead of trying to split all oversized modules at once. The selected slice is the control-auth and enterprise-access middleware cluster inside `crates/cli/src/commands/start.rs`.

</domain>

<decisions>
## Implementation Decisions

### choose one cohesive extraction
The middleware/auth slice already has clear inputs, outputs, and focused tests. That makes it the safest first extraction from `start.rs`.

### move the focused tests with the extracted code
The auth and enterprise-access middleware tests should travel with the extracted module so the behavior contract remains pinned to the new boundary.

</decisions>

<code_context>
## Existing Code Insights

- `start.rs` was over 14k lines before this phase.
- The auth cluster handled control bearer auth, trusted proxy auth, origin validation, enterprise access middleware, and their dedicated tests.
- The router wiring at the top of `run(...)` could stay unchanged if the extraction preserved the same function names and state types.

</code_context>

<specifics>
## Specific Ideas

- create `crates/cli/src/commands/start/auth.rs`
- keep the public behavior contract the same from `run(...)`
- verify with the existing focused auth and enterprise middleware tests

</specifics>

<deferred>
## Deferred Ideas

- larger route-family extraction from `start.rs`
- `skills.rs` decomposition

</deferred>

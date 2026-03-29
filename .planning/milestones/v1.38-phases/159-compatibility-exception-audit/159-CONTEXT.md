# Phase 159: Compatibility Exception Audit - Context

**Gathered:** 2026-03-28
**Status:** Completed

<domain>
## Phase Boundary

Audit any remaining native shims or compatibility exceptions explicitly so the final native-product claim cannot hide unresolved ownership inside surviving legacy surfaces.

</domain>

<decisions>
## Implementation Decisions

### explicit exceptions
Treat surviving `main.rs` and command-tree ownership as bounded exceptions unless source evidence proves deletion or full retirement.

### claim discipline
An explicit exception is acceptable. A hidden exception is not. The phase should classify what remains without forcing a false “all gone” conclusion.

</decisions>

<code_context>
## Existing Code Insights

- `crates/cli/src/main.rs` still contains the main clap parser and command dispatch.
- `crates/cli/src/commands/mod.rs` still declares the command-tree module set.
- `crates/gateway` and `crates/mcp` are real crate boundaries, but their existence does not erase the surviving CLI command layer.

</code_context>

<specifics>
## Specific Ideas

- Record the surviving command-tree and bootstrap surfaces as explicit bounded exceptions.
- Use those exceptions to shape the final native-product claim boundary.

</specifics>

<deferred>
## Deferred Ideas

- Future source deletion or deeper retirement work under a new denominator

</deferred>

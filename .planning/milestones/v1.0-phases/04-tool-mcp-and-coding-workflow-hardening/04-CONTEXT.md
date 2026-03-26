# Phase 4: Tool, MCP, and Coding Workflow Hardening - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Harden the shipped tool, MCP, and coding workflow surfaces so they execute within explicit bounds, fail clearly, and leave operator-reviewable artifacts behind. This phase improves the current Rust-owned control plane, MCP exposure, tool registry, and workspace coding integrations rather than inventing a separate autonomous execution stack.

</domain>

<decisions>
## Implementation Decisions

### Execution boundaries
- **D-01:** Treat explicit bounds as the core production requirement: timeout, allow or deny policy, workspace scope, and failure classification must be part of the default tool and coding contract.
- **D-02:** Reuse the existing control-plane manifests (`tool_allow`, `tool_deny`, approval policy, autonomy policy) as the canonical source of execution policy instead of inventing a second permission model.
- **D-03:** Side-effectful tool or coding actions should prefer blocked or clearly refused behavior over opportunistic execution when policy, workspace, or runtime context is ambiguous.
- **D-04:** Existing MCP command validation and bounded compiled-skill execution are good foundations, but they need clearer end-to-end operator semantics around what was allowed, what timed out, and why something failed.

### Tool and MCP behavior
- **D-05:** Tool discovery should stay anchored on shipped registries and declared MCP surfaces, not ad hoc shell probing during live assistant execution.
- **D-06:** MCP and tool failures must be classified in operator-meaningful terms such as validation failure, connection failure, timeout, execution failure, or policy refusal instead of collapsing into generic errors.
- **D-07:** The runtime should make it easy to inspect recent tool or MCP activity without requiring operators to infer behavior from raw logs alone.
- **D-08:** Phase 4 should preserve the Rust-owned control APIs and MCP server as the primary production path; Python or experimental lanes remain secondary compatibility surfaces.

### Coding workflow boundary
- **D-09:** The MVP coding workflow is workspace-bounded inspect, edit, run, and verify behavior against the current repository, not arbitrary machine-wide shell autonomy.
- **D-10:** Coding actions should leave the workspace recoverable: no silent destructive mutation, and every meaningful run should leave behind enough transcript, patch, or command output for operator review.
- **D-11:** Existing Cursor ACP and local tool-profile generation are the right surfaces to harden first because they already expose code search, edits, command execution, and host-specific tool briefings.
- **D-12:** Coding flows should prefer auditable structured artifacts over “trust me” agent narratives. If code changed or a command ran, the operator should be able to inspect what happened afterward.

### Operator visibility
- **D-13:** Tool and coding hardening is incomplete unless Control UI or control APIs can show recent execution outcomes and artifact locations directly.
- **D-14:** LangSmith traces and metrics are useful but insufficient on their own; the MVP also needs workspace-local reviewable artifacts for operators who are debugging real runs.
- **D-15:** Operator inspection should foreground the latest execution status and artifact summary before raw JSON, following the same trust pattern used in Phases 2 and 3.

### the agent's Discretion
- The exact artifact schema and file layout are at the agent's discretion as long as they are stable, workspace-local, and easy to inspect from CLI or control surfaces.
- The split between tool or MCP execution hardening and coding-workflow artifact hardening can move slightly as long as the phase ends with one coherent trust contract.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Tool registry and operator control
- `crates/cli/src/commands/tools.rs` — local tool profiling, drift detection, generated host artifacts, and control-surface reports
- `crates/cli/src/commands/control.rs` — control-plane manifests, tool allow or deny policy, autonomy policy, and runtime mode definitions
- `crates/cli/src/commands/start.rs` — `/control/tools` APIs, MCP server registration, traced MCP handlers, and current runtime tool instrumentation

### Agent and coding workflow runtime
- `crates/agent/src/tools.rs` — registry execution path and current tool error handling
- `crates/agent/src/runtime.rs` — assistant tool loop, workspace path propagation, and tool iteration limits
- `crates/cli/src/commands/cursor.rs` — shipped coding and IDE integration surface for inspect, edit, run, and git-style workflows

### MCP discovery and transport
- `crates/mcp/src/transport.rs` — subprocess allowlist and command validation for MCP servers
- `crates/mcp/src/registry.rs` — MCP server connection and tool discovery lifecycle
- `crates/cli/src/commands/mcp2cli.rs` — token-efficient tool discovery and execution path for saved MCP or OpenAPI sources

### Existing verification anchors
- `tests/integration/src/agent_runtime_test.rs` — baseline runtime tool-calling behavior
- `tests/integration/src/mcp_test.rs` — MCP discovery, translation, and registry coverage
- `tests/integration/src/documented_scenario_test.rs` — approval-oriented operator workflow examples

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `tools.rs` already persists local tool profiles, drift status, and generated host artifacts under `.claw/control`, which is a natural artifact root for broader execution evidence.
- `control.rs` already defines allow or deny lists, approval policy, and autonomy settings that can become the execution-policy contract for tool and coding actions.
- `start.rs` already exposes `/control/tools` APIs and wraps MCP tool handlers with LangSmith tracing, so Phase 4 can extend an existing operator surface instead of starting from zero.
- `cursor.rs` already frames the coding lane around bounded project-root operations such as search, read, edit, create, delete, run command, lint, and format.
- `transport.rs` already rejects arbitrary MCP subprocess commands and shell metacharacters, which gives Phase 4 a concrete security floor for subprocess-based tool integrations.

### Established Patterns
- Earlier phases improved trust by surfacing typed summaries and decision metadata in shipped operator surfaces rather than burying state in logs.
- Workspace-local files under `.claw/control` are already used for durable operator metadata and generated artifacts, so execution evidence should probably live there too.
- The Rust CLI and control runtime are the canonical operator path for MVP; deeper experimental integrations should conform to those surfaces rather than bypass them.

### Gaps to Close
- Tool execution behavior is spread across runtime helpers, MCP handlers, and control APIs, but there is not yet a single operator-visible ledger of recent tool or coding runs.
- Current tool failures are often returned as generic error strings without consistent status categories or artifact capture.
- Coding-oriented surfaces advertise bounded operations, but the repo does not yet clearly expose per-run transcripts, diffs, or verification artifacts through operator inspection.
- Control APIs expose tool profiles and memory or session inspection, but not a cohesive “what just ran, what happened, and where is the evidence?” view for tool and coding actions.

</code_context>

<specifics>
## Specific Ideas

- A strong first slice would be a durable execution ledger for MCP and tool calls with explicit status, elapsed time, and artifact pointers.
- The coding workflow should feel like “bounded operator-assist coding” rather than “unbounded shell agent,” even when exposed through Cursor ACP or compiled skill tools.
- Artifact review should likely converge on a small, consistent schema that can describe command output, diffs or patches, and verification results the same way across CLI and control surfaces.

</specifics>

<deferred>
## Deferred Ideas

- Fully unsupervised multi-step business automation remains outside this phase even if better tool contracts help enable it later.
- Enterprise approval chains, RBAC, and compliance audit export are out of scope for the MVP hardening pass.
- New large feature domains such as broader browser automation or travel booking remain deferred; this phase hardens the existing tool and coding substrate first.

</deferred>

---
*Phase: 04-tool-mcp-and-coding-workflow-hardening*
*Context gathered: 2026-03-26*

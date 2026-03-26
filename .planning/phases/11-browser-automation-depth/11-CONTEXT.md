# Phase 11: Browser Automation Depth - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Deepen the shipped browser lane so operators can run richer multi-step browser workflows and inspect durable workflow evidence afterward. This phase should extend the existing Rust-owned browser control plane rather than inventing a separate autonomous browser stack or weakening the explicit external-backend policy boundary.

</domain>

<decisions>
## Implementation Decisions

### Browser depth contract
- **D-01:** Treat richer multi-step workflows as the primary parity gap. The repo already supports one-off browser actions, sessions, artifacts, screenshots, PDFs, and inspection, but it lacks a durable operator-facing history of higher-level browser workflow runs.
- **D-02:** Preserve the existing browser trust contract. Any deeper browser surface must keep the current backend policy, backend audit, and explicit session/backend selection instead of bypassing them.
- **D-03:** Prefer durable, workspace-local browser workflow receipts over frontend-only reconstruction. Operators should be able to inspect what ran, where it ended, and which artifacts were produced from shipped runtime surfaces.

### Control-plane shape
- **D-04:** Reuse the existing `.claw/browser` storage conventions and control HTTP surface rather than introducing a new database or service.
- **D-05:** Follow the same pattern used in prior phases: append structured records to a durable local ledger, expose a typed runtime endpoint, then render that endpoint in Control UI.
- **D-06:** Browser depth should make action sequences more auditable first; more ambitious browser reasoning or agent-led autonomy remains outside this phase.

### Operator visibility
- **D-07:** Browser workflow history should foreground practical operator questions: which backend ran, which session was used, how many steps executed, whether the run succeeded, where it ended, and which durable artifact was written.
- **D-08:** Control UI should surface recent browser workflow runs directly alongside existing browser sessions and artifacts so operators do not need to inspect raw workspace files.
- **D-09:** The browser depth slice should remain truthful about what exists today: richer workflow visibility and inspection, not full unconstrained browser agent parity.

### the agent's Discretion
- The exact workflow-history schema, naming, and summary fields are at the agent's discretion as long as they are stable, easy to inspect, and preserve the current trust model.
- Supporting docs can be updated where they most directly affect operator understanding of the shipped browser depth lane.

</decisions>

<canonical_refs>
## Canonical References

**Downstream implementation MUST read these before editing.**

### Browser runtime and storage
- `crates/cli/src/commands/browser.rs` — browser request/result types, session persistence, artifact creation, backend policy, backend audit, and sequence execution
- `crates/cli/src/commands/start.rs` — shipped `/control/browser/*` runtime routes

### Operator surfaces
- `crates/cli/src/commands/control_ui.html` — current browser actions, sessions, and artifacts panels
- `crates/cli/src/commands/control_ui.rs` — dashboard contract tests for Control UI surfaces

### Existing durable evidence patterns
- `crates/cli/src/commands/inspect.rs` — durable JSONL history helpers used by other operator-visible ledgers
- `tests/integration/src/tool_execution_history_test.rs` — pattern for focused durable-history regression coverage

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `browser.rs` already persists browser sessions and writes browser artifacts under `.claw/browser`, which gives Phase 11 a natural durable storage root.
- `run_sequence()` already supports richer multi-step workflows including navigation, wait, click, type, fill, select, extract, screenshot, PDF, storage mutation, and navigation history actions.
- `start.rs` already exposes a real browser control surface with `/control/browser/run-sequence`, `/control/browser/sessions`, `/control/browser/artifacts`, `/control/browser/backend-policy`, and `/control/browser/backend-audit`.
- `control_ui.html` already ships a Browser Actions panel plus Browser Artifacts and Browser Sessions tables, so the operator surface can be deepened in place.

### Established Patterns
- Prior parity and hardening phases consistently ship durable local ledgers plus typed control endpoints plus Control UI tables instead of relying on raw logs.
- Control UI panels tend to read from runtime JSON endpoints directly and keep browser-specific state local to the page.
- Verification and lifecycle work from v1.1 now expects a real `11-VERIFICATION.md` before phase completion.

### Gaps to Close
- There is no operator-visible history of recent browser workflow runs beyond whatever artifact files happen to exist on disk.
- Operators can inspect sessions and artifacts independently, but not a single recent-history view tying together backend, session, final URL, step count, and durable artifact output.
- Browser depth docs still describe the current lane more as actions and artifacts than as a richer workflow surface.

</code_context>

<specifics>
## Specific Ideas

- Add a durable browser workflow history ledger for sequence runs and richer browser actions that produce artifacts.
- Expose recent browser workflow history through a typed control endpoint rather than asking operators to inspect `.claw/browser` manually.
- Add a Control UI table for recent browser workflows so browser depth feels materially closer to OpenClaw's broader operator surface.

</specifics>

<deferred>
## Deferred Ideas

- Full browser-agent autonomy, task planning, or unconstrained web automation across arbitrary side-effectful sites remains out of scope.
- A standalone browser database, remote browser cluster management, or cloud-only browser orchestration is out of scope for this phase.
- Deeper supervision or mobile parity work should stay in later phases even if this phase establishes a reusable durable-history pattern.

</deferred>

---
*Phase: 11-browser-automation-depth*
*Context gathered: 2026-03-26*

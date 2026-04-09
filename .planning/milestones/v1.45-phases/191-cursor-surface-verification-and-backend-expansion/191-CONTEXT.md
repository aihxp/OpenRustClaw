# Phase 191: Cursor Surface Verification and Backend Expansion - Context

**Gathered:** 2026-04-08
**Status:** Complete

<domain>
## Phase Boundary

Verify Cursor against the current machine and its documented CLI surface, then either preserve detection-only behavior or promote it into the same delegated local-agent contract used by the other supported CLIs. This phase covers truthful Cursor discovery, execution-capable classification, and onboarding or model-surface parity. It does not yet solve multi-host routing, shared route receipts, or routing-console UX.

</domain>

<decisions>
## Implementation Decisions

### Cursor truthfulness
- **D-01:** Cursor must be classified from current executable evidence, not stale assumptions from the prior milestone.
- **D-02:** `cursor agent` is the only acceptable execution lane for this phase because it exposes documented login, model listing, and headless print surfaces.
- **D-03:** Operator-facing surfaces should distinguish backend readiness from operator policy allowlisting so onboarding can verify a signed-in local Cursor install without pretending policy is already open.

### Delegated backend contract
- **D-04:** Cursor should reuse the existing delegated local-agent contract instead of adding a special one-off backend path.
- **D-05:** Cursor model discovery should be surfaced as supported because `cursor agent models` returns a real vendor-managed catalog.
- **D-06:** Subscription-managed onboarding is acceptable for providerless delegated backends when the product never imports vendor tokens or browser sessions.

### Deferred scope
- **D-07:** Actual routed execution still remains policy-gated and audit-backed; phase 191 does not auto-allow Cursor in enterprise policy.
- **D-08:** First-task routing that prefers Cursor based on setup state belongs to later phases in this milestone.

</decisions>

<canonical_refs>
## Canonical References

- `.planning/PROJECT.md`
- `.planning/REQUIREMENTS.md`
- `.planning/ROADMAP.md`
- `.planning/STATE.md`
- `crates/app/src/agent_backend_catalog.rs`
- `crates/app/src/agent_backend_control.rs`
- `crates/app/src/onboarding_lane_catalog.rs`
- `crates/app/src/runtime_provider_switch.rs`
- `crates/cli/src/commands/models.rs`
- `crates/cli/src/commands/onboard.rs`
- `crates/cli/src/commands/runtime.rs`

</canonical_refs>

<code_context>
## Existing Code Insights

### What changed since v1.44 assumptions
- The local machine’s `cursor` binary now exposes `cursor agent`, `cursor agent status`, and `cursor agent models`.
- `cursor agent --help` documents headless `--print`, `--output-format`, `--mode`, `--trust`, `--workspace`, `--model`, and `--yolo`.
- The prior detection-only classification was therefore stale.

### Existing seams reused here
- `AgentBackendCatalogService` already owns safe local probing for vendor CLIs.
- `AgentBackendControlService` already normalizes discovered tools into policy-aware delegated backend contracts.
- Onboarding already knows about `subscription_managed` access mode, but it previously rejected it before any backend-specific validation path existed.

</code_context>

<deferred>
## Deferred Ideas

- Auto-writing Cursor into backend allowlists
- Multi-host Cursor routing
- Route receipts and recovery hints that mention Cursor specifically
- Cursor-first task suggestions after onboarding

</deferred>

---

*Phase: 191-cursor-surface-verification-and-backend-expansion*
*Context gathered: 2026-04-08*

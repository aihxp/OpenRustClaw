# Phase 194: Agent Routing Console and Operator Policy UX - Context

**Gathered:** 2026-04-08
**Status:** Complete

<domain>
## Phase Boundary

Turn the delegated routing data model into an operator-facing surface. This phase is about one coherent routing console across Control UI, CLI, and inspect surfaces, plus shipped policy controls that avoid raw file edits. It does not yet change first-task orchestration behavior after onboarding.

</domain>

<decisions>
## Implementation Decisions

- **D-01:** The routing console should reuse the existing route-policy receipts, fabric inventory, and enterprise browser-policy contract instead of inventing a parallel frontend-only state model.
- **D-02:** Policy updates must flow through shipped update paths so operators can change allowlists and runtime guardrails without hand-editing TOML or JSON.
- **D-03:** Control UI, CLI, and inspect surfaces should use the same vocabulary: allowed backends, route signals, route receipts, and recovery hints.
- **D-04:** The dedicated routing console can be additive inside the broader Control UI as long as it has its own focused panel, tables, and action form.

</decisions>

---

*Phase: 194-agent-routing-console-and-operator-policy-ux*
*Context gathered: 2026-04-08*

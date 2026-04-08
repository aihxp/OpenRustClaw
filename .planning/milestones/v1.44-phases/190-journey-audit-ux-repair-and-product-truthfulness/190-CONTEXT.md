# Phase 190: Journey Audit, UX Repair, and Product Truthfulness - Context

**Gathered:** 2026-04-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Audit the end-to-end user journey and agent journey now that delegated discovery, onboarding, and runtime execution are in place. This phase should remove dead ends, inconsistent terminology, and mismatches between docs, onboarding text, inspect output, and control surfaces.

</domain>

<decisions>
## Implementation Decisions

- **D-01:** Favor truthful copy and explicit recovery guidance over marketing-style claims.
- **D-02:** Reuse existing typed services and reports rather than adding one-off UX strings that drift again.
- **D-03:** The “agent journey” must include discovery, selection, runtime routing, receipt inspection, and recovery paths.

</decisions>

<canonical_refs>
## Canonical References

- `.planning/PROJECT.md`
- `.planning/REQUIREMENTS.md`
- `.planning/ROADMAP.md`
- `.planning/STATE.md`
- `.planning/phases/188-onboarding-and-model-selection-cohesion/188-02-SUMMARY.md`
- `.planning/phases/189-runtime-integration-and-agent-journey-delivery/189-02-SUMMARY.md`
- `README.md`
- `crates/cli/src/commands/onboard.rs`
- `crates/cli/src/commands/models.rs`
- `crates/cli/src/commands/inspect.rs`
- `crates/cli/src/commands/control_ui.html`

</canonical_refs>

<code_context>
## Existing Code Insights

- Onboarding, models, and inspect now share enough typed lane metadata to support a coherent story.
- Delegated runtime execution is bounded and inspectable, but the product narrative has not yet been re-audited after those capabilities landed.
- Control UI and README still need an explicit pass for updated delegated backend language and support boundaries.

</code_context>

---

*Phase: 190-journey-audit-ux-repair-and-product-truthfulness*
*Context gathered: 2026-04-08*

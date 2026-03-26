# Phase 8: Verification Artifact Contract - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 8 hardens the GSD lifecycle so verification artifacts stop being optional folklore. The contract for this phase is narrow and concrete: milestone and phase bootstrap must see the real roadmap state, phase completion must fail when verification evidence is missing or stale, and operators must be able to inspect verification debt before milestone audit or archive work begins.

</domain>

<decisions>
## Implementation Decisions

### Lifecycle integrity first
- **D-01:** This phase should fix the workflow contract inside the GSD tooling itself, not paper over the v1.0 audit gap with manual guidance.
- **D-02:** `init milestone-op` must derive milestone phase counts from the active roadmap, not from pre-existing phase directories, because a new milestone begins with roadmap phases before disk scaffolding exists.
- **D-03:** `phase complete` must treat missing, stale, pending, and `gaps_found` verification as blocking conditions. `human_needed` remains a routed state, not an automatic pass.

### One verification truth source
- **D-04:** Verification readiness should be inspected through one shared helper so `phase complete`, `audit-uat`, and later lifecycle flows do not drift again.
- **D-05:** "Stale verification" should mean the verification artifact is older than the latest execution evidence in the phase directory, especially summaries and UAT artifacts.
- **D-06:** Operators need synthetic debt entries for missing or stale verification artifacts, not only parsed items from files that already exist.

### the agent's Discretion
- The exact helper/module boundary is flexible as long as the inspection rules are shared and reused.
- Docs changes should stay tightly scoped to the changed contract: verification artifact shape, blocking rules, and operator visibility.

</decisions>

<canonical_refs>
## Canonical References

**Downstream planning and implementation should anchor on these files.**

### Current lifecycle behavior
- `.codex/get-shit-done/bin/lib/init.cjs` — milestone bootstrap currently counts only phase directories
- `.codex/get-shit-done/bin/lib/phase.cjs` — phase completion currently emits verification debt as warnings only
- `.codex/get-shit-done/bin/lib/uat.cjs` — cross-phase debt audit currently misses absent or stale verification artifacts
- `.codex/get-shit-done/bin/lib/commands.cjs` — direct verification scaffold currently emits outdated frontmatter
- `.codex/get-shit-done/bin/lib/template.cjs` and `.codex/get-shit-done/bin/lib/frontmatter.cjs` — current structured verification template/schema

### Workflow intent
- `.codex/get-shit-done/workflows/autonomous.md` — autonomous flow expects post-execution verification routing
- `.codex/get-shit-done/workflows/execute-phase.md` — execute-phase expects `VERIFICATION.md` before phase completion
- `.codex/get-shit-done/workflows/audit-milestone.md` — milestone audit treats missing phase verification as a blocker

### Existing project evidence
- `.planning/milestones/v1.0-MILESTONE-AUDIT.md` — preserved evidence of the missing v1.0 verification-artifact debt
- `.planning/ROADMAP.md` and `.planning/REQUIREMENTS.md` — Phase 8 goal, requirements, and success criteria

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `getMilestonePhaseFilter()` in `core.cjs` already knows how many phases belong to the active milestone, even when phase directories do not exist yet.
- `extractCurrentMilestone()` already scopes roadmap parsing correctly across archived milestones.
- `template fill verification` already emits the newer structured verification frontmatter and body shape.

### Gaps to Close
- `init milestone-op` reports `phase_count: 0` for a brand-new milestone that already has roadmap phases.
- `phase complete` can still mark a phase complete when `VERIFICATION.md` is absent, stale, or failed.
- `audit-uat` cannot tell the operator that a phase has no verification artifact at all.
- `scaffold verification` still creates a legacy frontmatter/body shape that does not match the active verification schema.

</code_context>

<specifics>
## Specific Ideas

- Build one shared verification-readiness inspector that returns file path, status, blocking issues, warnings, and freshness details for a phase.
- Use that inspector both for hard lifecycle gates (`phase complete`) and for operator reporting (`audit-uat`).
- Verify the contract with temp-workspace CLI probes so the phase proves behavior, not just static code edits.

</specifics>

<deferred>
## Deferred Ideas

- Retrofitting all archived v1.0 phases with reconstructed verification reports is out of scope for this phase.
- Milestone audit/archive enforcement belongs to Phase 9 after the per-phase contract is solid.
- Enterprise approval and audit policy surfaces remain for Phase 10.

</deferred>

---
*Phase: 08-verification-artifact-contract*
*Context gathered: 2026-03-26*

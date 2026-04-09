---
phase: 188
requested_reviewers: [claude]
reviewers: [claude]
reviewed_at: 2026-04-09T21:04:50.113Z
plans_reviewed: [188-01-PLAN.md, 188-02-PLAN.md]
---
# Cross-AI Plan Review — Phase 188

## Claude Review

# Cross-AI Review: Phase 188 — Onboarding and Model Selection Cohesion

## Plan 188-01: Shared Onboarding Lane Catalog

### Summary
Introduces a unified `onboarding_lane_catalog` module in `openrustclaw-app` to replace scattered static provider tables in onboard and models commands. The scope is well-bounded — catalog only, no persistence changes — which is the right sequencing choice.

### Strengths
- Clean separation: catalog creation in plan 01, persistence in plan 02
- Reuses existing `AgentBackendCatalogEntry` and `DelegatedAgentBackendContract` rather than inventing new discovery
- Explicitly preserves direct-provider paths (D-03 from CONTEXT.md)
- Compatibility notes for delegated lanes prevent false readiness signals

### Concerns
- **MEDIUM**: The plan doesn't specify the shape of `OnboardingLaneDescriptor` beyond what can be inferred. If the struct is underspecified, plan 02 may need to backfill fields, creating churn.
- **LOW**: `models.rs` currently has a `get_default_models()` HashMap that returns detailed `ModelInfo` per provider. The plan says models should use the shared catalog, but model-level detail (context window, vision support) lives outside the lane concept. Unclear whether `models list` keeps its own model detail table or the lane catalog absorbs it.
- **LOW**: No mention of how ordering works in the unified menu. Current onboard and models commands have implicit provider ordering. If the catalog returns lanes in discovery order, the UX may shift unpredictably.

### Suggestions
- Specify the `OnboardingLaneDescriptor` fields in the plan frontmatter or objective so plan 02 doesn't have to retrofit.
- Clarify that `models list` continues to own per-model detail (context window, features) while the lane catalog owns the provider/backend row. This avoids scope creep into model metadata.
- Pin a deterministic sort order for lanes (e.g., direct API first, local runtimes second, delegated agents last) so the menu is stable across machines.

### Risk Assessment
**LOW**. The scope is narrow, the dependency on existing catalog services is solid, and the verification commands target the right crates.

---

## Plan 188-02: Lane Persistence Through Handoff, Repair, and Inspect

### Summary
Extends setup handoff and lifecycle state to carry the selected lane identity (including delegated backend ID, lane kind, and compatibility note) through persistence, repair, resume, and inspect. This closes the disconnect where onboarding showed one thing and inspect remembered another.

### Strengths
- Directly addresses the stated disconnect in CONTEXT.md between onboarding selection and downstream surfaces
- Fields like `selected_lane_kind`, `selected_backend_id`, and `selected_lane_compatibility_note` already exist in `SetupHandoffState` and `SetupHandoffReport` — the plan extends an established pattern rather than inventing new persistence
- Covers the repair/resume path, not just happy-path onboarding
- Dependency on 188-01 is correct and explicit

### Concerns
- **MEDIUM**: `SetupHandoffState` and `SetupLifecycleState` are two separate structs with overlapping lane fields. The plan modifies both but doesn't address whether they should converge or stay separate. Adding more fields to both without a clear ownership rule increases drift risk over time.
- **MEDIUM**: The plan says "delegated backend lane identity survives repair" but doesn't specify what happens when a previously-selected delegated backend is no longer detected on the machine (e.g., user uninstalls Claude Code between sessions). The repair path should degrade gracefully — flag the lane as stale rather than crash or silently drop it.
- **LOW**: `setup_lifecycle.rs` has `selected_model_lane_summary()` which builds a human-readable string from lane fields. If new fields are added, this function needs updating too, but isn't listed in `files_modified`.
- **LOW**: No migration story for existing `.claw/control/setup-state.json` files that lack the new fields. Serde defaults should handle this, but worth confirming the structs use `#[serde(default)]` on new optional fields.

### Suggestions
- Add a test case for the "backend disappeared between sessions" scenario — load state with a `selected_backend_id` that no longer appears in discovery, and verify inspect/repair surface a clear message.
- Confirm `selected_model_lane_summary()` in `setup_lifecycle.rs` is updated to render delegated-lane metadata; add it to `files_modified` if needed.
- Ensure all new `Option<String>` fields on handoff/lifecycle structs carry `#[serde(default)]` to avoid deserialization failures on existing state files.

### Risk Assessment
**LOW-MEDIUM**. The persistence extension is straightforward, but the missing-backend-on-resume edge case and the dual-struct drift between `SetupHandoffState` and `SetupLifecycleState` deserve explicit handling to avoid subtle bugs in repair flows.

---

## Phase-Level Assessment

**Overall Risk: LOW**. The two-plan split is well-sequenced: catalog first, persistence second. The phase goals (success criteria 1–4) are directly addressed. The main gap is the missing-backend-on-resume edge case in plan 02 — worth a test but not a blocker. No security, performance, or scope-creep concerns.

---

## Consensus Summary

### Agreed Strengths
- Single-reviewer artifact: see the completed reviewer section above for the usable strengths signal.

### Agreed Concerns
- No cross-review consensus is available because only one reviewer completed successfully.

### Divergent Views
- No multi-reviewer comparison is available; parsed overall risk label: claude=LOW.

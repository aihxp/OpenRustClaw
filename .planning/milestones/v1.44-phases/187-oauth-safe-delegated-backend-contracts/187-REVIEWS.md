---
phase: 187
requested_reviewers: [claude]
reviewers: [claude]
reviewed_at: 2026-04-09T21:04:04.035Z
plans_reviewed: [187-01-PLAN.md, 187-02-PLAN.md]
---
# Cross-AI Plan Review — Phase 187

## Claude Review

# Cross-AI Review: Phase 187 — OAuth-Safe Delegated Backend Contracts

## 187-01: Typed Delegated-Backend Contract Layer

### Summary
Plan 01 introduces the contract types and policy evaluation logic in `agent_backend_control.rs`, bridging discovery metadata from Phase 186 into typed, policy-aware structures. The code is already implemented and the review is retrospective. The implementation is clean, well-scoped, and directly addresses AUTH-01, AUTH-02, and AUTH-04.

### Strengths
- Reuses existing `ExternalBackendAuditEntry` and config structures instead of inventing parallel types
- `DelegatedModelCatalogMode` with explicit `VendorManaged` and `Unknown` variants satisfies the "no guessing" requirement
- Detection-only backends (`integration_only` policy classification) correctly map to `Unsupported` execution kind
- Four-gate policy evaluation (execution kind → local-wrapper policy → allowlist → readiness) is well-ordered
- Tests cover the key decision paths: vendor-managed labeling, detection-only ineligibility, deny on not-ready, allow on ready+listed

### Concerns
- **LOW**: `execution_eligible` is derived purely from `readiness == Ready`, but a backend could be `Ready` with `delegated_execution == Unsupported` in theory — the current guard on `DelegatedExecutionKind::LocalCli` in `evaluate_execution` catches this, but the `execution_eligible` field on the contract is slightly misleading in that edge case.
- **LOW**: `policy_classification` is a free-form `String` compared via `==` against magic literals like `"delegated_cli_candidate"` and `"integration_only"`. An enum would be safer but this matches the existing catalog pattern, so it's consistent.
- **LOW**: `audit_entry` sets `success: allowed` — conflating "policy allowed it" with "execution succeeded." Acceptable for the contract layer since actual execution hasn't happened yet, but downstream consumers should not treat this as execution-success evidence.

### Suggestions
- Consider a doc comment on `execution_eligible` clarifying it means "local CLI + ready", not just "ready"
- If `policy_classification` strings multiply, promote them to an enum in a future cleanup pass

### Risk Assessment
**LOW** — Well-bounded, reuses existing infrastructure, tests cover the meaningful branches. No security or correctness gaps.

---

## 187-02: Policy Surface Wiring (Inspect, Enterprise Policy, Browser)

### Summary
Plan 02 wires the contracts from 01 into CLI-facing inspect and policy commands so operators can see why a delegated backend is allowed, blocked, or detection-only. It depends correctly on 01 and targets the right files.

### Strengths
- Correctly scoped to surface wiring, not new business logic
- Reuses existing `external_backends` allowlist and local-wrapper controls (D-04, D-05)
- Verification commands target the right test suites
- Keeps browser backend control and agent backend control as parallel but non-overlapping surfaces

### Concerns
- **MEDIUM**: The plan lists `browser.rs` as modified but the phase goal is about *agent* delegated backends. If browser commands are only getting minor display updates for consistency, that's fine — but if new agent-backend policy logic leaks into the browser command surface, it risks coupling two distinct control paths.
- **LOW**: No mention of `--json` output support for the new contract fields in inspect. Operators scripting against inspect output may need structured access.
- **LOW**: The verification section only lists `cargo test` for the three CLI command modules. No manual verification step for "run `openrustclaw inspect` and confirm contract fields appear" — acceptable for a plan but worth noting.

### Suggestions
- Clarify in the plan *what* changes in `browser.rs` — if it's just ensuring the existing browser-backend policy display doesn't conflict with the new agent-backend contracts, say so explicitly
- Ensure inspect surfaces include the `model_catalog_mode` and `delegated_execution_kind` fields so operators can distinguish vendor-managed from truly unsupported

### Risk Assessment
**LOW** — Straightforward surface wiring with correct dependency ordering. The browser.rs scope is the only item worth watching to avoid unintended coupling.

---

## Overall Phase Assessment

**Risk: LOW.** Both plans are well-scoped, correctly ordered, and directly satisfy the four success criteria. The implementation reuses existing config and audit infrastructure instead of building parallel systems. The main residual risk is minor: free-form `policy_classification` strings and the `success == allowed` conflation in pre-execution audit entries. Neither blocks correctness or security today.

---

## Consensus Summary

### Agreed Strengths
- Single-reviewer artifact: see the completed reviewer section above for the usable strengths signal.

### Agreed Concerns
- No cross-review consensus is available because only one reviewer completed successfully.

### Divergent Views
- No multi-reviewer comparison is available, and no explicit overall risk label was parsed from the completed review.

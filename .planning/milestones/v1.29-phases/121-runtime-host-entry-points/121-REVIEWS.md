---
phase: 121
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T20:09:43.959Z
plans_reviewed: [121-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 121

## Gemini Review

Here is a review of the provided implementation plan for Phase 121: Runtime Host Entry Points.

### Summary
The plan is an extremely high-level restatement of the phase's context and decisions, but it fundamentally lacks any concrete technical implementation details. While it correctly identifies *what* needs to be done conceptually (mapping bootstraps, tying entrypoints to ports), it provides absolutely no direction on *how* or *where* this will happen within the Rust codebase. It reads more like a product requirement than an actionable engineering plan.

### Strengths
- **Goal Alignment:** The plan perfectly mirrors the phase boundary and implementation decisions outlined in the context.
- **Scope Discipline:** It successfully avoids bundling repository-adapter work into this slice, adhering to the requested constraints.

### Concerns
- **HIGH: Zero Technical Specificity.** The plan does not identify which files, modules, or crates (e.g., `crates/app`, `crates/cli`, `crates/distributed`) will be modified or created to establish these new native delivery owners and app ports.
- **HIGH: Un-testable Verification.** Step 3 ("Verify that the roadmap leaves an incremental implementation path...") is subjective and cannot be empirically tested or verified by an engineer or CI pipeline. 
- **MEDIUM: Missing Legacy Context.** There is no identification of the *current* "legacy command-layer bootstrap path" or "legacy startup helpers" that are being replaced, which makes it impossible to design an effective transition strategy.

### Suggestions
- **Identify Target Architectures:** Explicitly name the Rust traits, structs, or modules that will serve as the "app ports" (e.g., `AppRuntimePort`, `BackgroundWorkerPort`) and specify which crate will own them.
- **Map the Transition:** Provide a concrete list of the current legacy helpers that are targeted for eventual deprecation so the new design explicitly accounts for their current responsibilities.
- **Define Concrete Verification:** Replace Step 3 with actionable verification criteria. For a planning/contract-level milestone, this should involve writing the actual Rust trait definitions (even if unimplemented) and writing a compiler-checked unit test or mock to prove the interface is viable, or producing a specific architectural ADR document.

### Risk Assessment
**MEDIUM** 
While the risk of breaking existing functionality is low (since this phase focuses on defining contracts and planning), the risk of executing this plan poorly is high. Because the plan lacks technical detail, whoever implements this will be forced to make all the architectural decisions on the fly, which frequently leads to misaligned interfaces, scope creep, or a failure to actually resolve the legacy coupling.

---

## Claude Review

I can't write files in plan mode except to the plan file, and the Write tool isn't available. Let me provide my review directly.

---

# Cross-AI Review: Phase 121 — Runtime Host Entry Points (Plan 121-01)

## Summary

A thin planning-contract phase that maps runtime-host and background-worker bootstrap ownership from legacy CLI command surfaces to explicit native delivery entrypoints. Appropriately scoped as conceptual ownership transfer, but so abstract it risks being unverifiable — the three steps read more like goals than actionable work items.

## Strengths

- Correct architectural move: treating runtime-host bootstrap as delivery ownership rather than a CLI command helper
- Incremental approach: leaves room for future implementation without bundling unrelated work
- Aligned with the greenfield transition trajectory and crate dependency order
- Avoids scope creep by staying at planning-contract level

## Concerns

- **HIGH** — No concrete deliverables. "Map entrypoints to app ports" and "tie to app ports" are the same step restated. What artifact proves this phase is done? A doc? A stub? A trait? Without tangible output, verification is subjective.
- **MEDIUM** — No definition of "app ports." The term references an abstraction in `openrustclaw-app` but the plan doesn't identify which existing interfaces constitute these ports or whether new ones are needed.
- **MEDIUM** — No inventory of what "runtime-host bootstrap" and "background-worker bootstrap" currently entail. Without listing the specific legacy startup paths (e.g., which functions in `start.rs` or `runtime.rs`), a future implementer must rediscover scope.
- **LOW** — Step 3 ("verify roadmap leaves incremental path") is a review activity, not a plan step. Doesn't belong as a peer of implementation steps.

## Suggestions

- Replace the three abstract steps with: (1) enumerate specific legacy bootstrap paths with file:line references, (2) define target app-port contracts in `openrustclaw-app`, (3) produce a mapping document tying each legacy path to its native successor
- Specify the verification artifact — even a planning phase needs a concrete output
- Name the specific files: likely `crates/cli/src/commands/start.rs`, `runtime.rs`, and a target in `crates/app/`

## Risk Assessment

**LOW** overall. The phase is small and non-destructive. The main risk isn't breakage but shipping without enough specificity to guide the next implementation phase — passing verification while leaving the successor phase exactly where this one started.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Reviewers disagreed on overall risk level: gemini=MEDIUM, claude=LOW.

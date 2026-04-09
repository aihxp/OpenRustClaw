---
phase: 113
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T20:00:17.950Z
plans_reviewed: [113-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 113

## Gemini Review

# Review of Plan 113-01

## 1. Summary

Plan 113-01 accurately reflects the strategic intent of Phase 113 by focusing on decoupling CLI routing from `main.rs` and transitioning toward an app-port invocation model. However, the plan is currently too abstract to be actionable. It functions more as a restatement of the phase goals rather than a concrete technical roadmap. It lacks specific details regarding deliverables, file modifications, interactions with existing CLI frameworks (like `clap`), and verification strategies, making it difficult for an engineer to execute without significant upfront architectural design work.

## 2. Strengths

*   **Strategic Alignment:** The plan perfectly mirrors the stated user decisions, ensuring the focus remains on the boundary definition and decoupling effort.
*   **Constrained Scope:** By focusing on defining the boundaries and dispatch path *before* full implementation, it correctly avoids a premature, high-risk rewrite of the entire `main.rs` routing logic.
*   **Architectural Correctness:** The shift from command-module cross-calls to app-port invocation is the correct architectural direction for a maintainable, decoupled system.

## 3. Concerns

*   **HIGH: Lack of Concrete Deliverables.** The verbs "Define" and "Verify" are highly ambiguous in this context. It is entirely unclear if this plan intends to produce documentation (e.g., ADRs, architecture diagrams), skeleton code (e.g., Rust traits, empty modules), or a working prototype.
*   **HIGH: Missing Technical Details.** The plan provides no specifics on *how* the native CLI dispatch will be implemented. There is no mention of how the existing CLI parsing framework (presumably `clap`) will interface with the new `openrustclaw-cli` dispatch layer or how the app ports will be structured.
*   **HIGH: Absence of a Verification Strategy.** There are no measurable criteria for success. Step 3 asks to "Verify that the roadmap preserves...", but it does not specify how to prove that the new dispatch path actually works or that `main.rs` can indeed be narrowed.

## 4. Suggestions

*   **Specify Concrete Outputs:** Clearly define what this phase produces. If it is an architectural design phase, specify the documentation files to be created (e.g., `docs/adr/00X-cli-dispatch-architecture.md`). If it involves code, list the specific modules or traits to be created in `crates/cli/` (assuming `openrustclaw-cli` maps there).
*   **Detail the Interface:** Explain the technical bridge between argument parsing and app-port invocation. For example: "Introduce a `CommandDispatcher` trait in `crates/cli/src/dispatch.rs` that takes a parsed `clap` subcommand and maps it to the corresponding application port."
*   **Create a Proof of Concept:** Add a step to implement a single, low-risk command using the new dispatch architecture. This provides tangible proof that the design works and that `main.rs` can cleanly delegate to it.
*   **Define Clear Verification Steps:** Replace Step 3 with actionable tests. For example: "Write a unit test in `crates/cli/` asserting that a mock command correctly invokes the expected app port without invoking legacy routing logic."

## 5. Risk Assessment

**HIGH**

**Justification:** While the architectural intent is sound, the plan in its current state is unexecutable. An engineer assigned to this plan would be forced to invent the entire technical solution from scratch, leading to unpredictable implementation timelines and a high probability of diverging from the intended architectural vision. The plan must be expanded with concrete technical details and deliverables before execution.

---

## Claude Review

# Review: Phase 113-01 — Native CLI Dispatch Layer

## Summary

This is a thin architectural-boundary plan that aims to decouple `main.rs` from being the permanent routing owner by defining a native dispatch layer routed through app ports. The plan is intentionally narrow — it defines ownership and contracts rather than implementing them — which is appropriate for a delivery-layer design phase. However, the plan is underspecified: three high-level steps with no file paths, no concrete contract shapes, and no verification criteria.

## Strengths

- Correct separation of concerns — bootstrap vs. dispatch vs. command execution
- Preserves the `openrustclaw` binary contract, avoiding a breaking change
- Aligns with the greenfield transition pattern already established in earlier milestones
- Scope is appropriately narrow for a design/contract phase

## Concerns

- **HIGH** — No concrete file paths or code references. The plan says "define native CLI dispatch ownership" but doesn't specify *where* or *what* that definition looks like. Which files in `crates/cli/` are affected? What does the app-port dispatch interface look like?
- **MEDIUM** — No verification criteria. Step 3 says "verify that the roadmap preserves the binary" but doesn't define what passing looks like. A `VERIFICATION.md` artifact is required per v1.1 conventions.
- **MEDIUM** — "App-port invocation" is referenced but never defined. If this is an existing pattern from earlier greenfield phases, the plan should reference the concrete type or trait. If it's new, it needs a shape.
- **LOW** — No mention of backward compatibility with existing command modules during transition. The plan implies a clean cut but `main.rs` in `crates/cli/` is described elsewhere as a compatibility-heavy surface.

## Suggestions

- Reference the actual current state of `crates/cli/src/main.rs` — how many subcommands route through it today, and which ones move to the new dispatch layer first.
- Name the concrete Rust type or trait that represents "app-port invocation" for CLI dispatch, or point to where it already exists (e.g., from the v1.34 native dispatch work).
- Add a verification step: after the contract is defined, confirm that at least one command can be dispatched through the new path without regressing the existing binary.
- Cross-reference Phase 34 (`Native CLI Dispatch and Core Operator Paths`) — that milestone already shipped native CLI dispatch. Clarify what *this* phase adds that wasn't already delivered.

## Risk Assessment

**MEDIUM** — The plan is directionally correct but too abstract to execute confidently. The biggest risk is redundancy with already-shipped v1.34 work — if native CLI dispatch was already implemented there, this phase may be defining something that already exists, leading to confusion or unnecessary abstraction. The fix is simple: read `main.rs` and the v1.34 archive, then sharpen the plan with concrete references.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Reviewers disagreed on overall risk level: gemini=HIGH, claude=MEDIUM.

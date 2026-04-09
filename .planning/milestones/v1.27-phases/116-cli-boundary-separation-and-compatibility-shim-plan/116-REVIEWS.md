---
phase: 116
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T20:03:52.528Z
plans_reviewed: [116-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 116

## Gemini Review

Here is a review of the proposed plan for Phase 116 based on the provided context.

### Review of 116-01-PLAN.md

**Summary**
Plan 116-01 is extremely high-level and functions more as a restatement of the phase's goals rather than a concrete implementation plan. It correctly identifies the core requirements from the context—separating parsing, rendering, and app invocation, and defining rules for temporary compatibility shims—but it completely lacks technical specifics, file paths, structural design, or actionable development steps. As an implementation plan for a "CLI Boundary Separation and Compatibility Shim", it is insufficient for an engineer to begin coding without making significant architectural assumptions.

**Strengths**
- **Alignment with Context:** Accurately reflects the decisions outlined in `CONTEXT.md`, specifically the mandate to separate parsing from rendering/invocation and to strictly bound compatibility shims.
- **Goal-Oriented:** Correctly identifies that the milestone advancement (`3/8`) is contingent on establishing this boundary end-to-end.

**Concerns**
- **Lack of Actionable Technical Detail (HIGH):** The plan lists "Separate parsing, rendering, and app invocation responsibilities explicitly" but does not define *how* or *where*. It needs to specify the target Rust modules/crates (e.g., `crates/cli`, `crates/app`), the traits or structs involved, and the data flow between them.
- **Undefined Shim Strategy (HIGH):** "Define the allowed role for any temporary compatibility shims" is a requirement for the plan, not an implementation step. The plan itself needs to explicitly define what that role is (e.g., "Legacy commands in `src/legacy_cli.rs` will be refactored to parse arguments using existing logic, but will immediately instantiate and call the new `crates/app` handlers, returning the native result types").
- **Missing Scope/Target (MEDIUM):** The plan does not identify which specific CLI commands or modules constitute the "first native CLI slice." Without this, the scope of the immediate implementation is unbounded.
- **No Testing or Verification Strategy (MEDIUM):** There are no steps detailing how the boundary separation or the compatibility shims will be tested to ensure they behave identically to the legacy implementation.

**Suggestions**
- **Define the Architecture:** Update Step 1 to explicitly map out the new boundary. For example: define a `Parser` trait/module, a distinct `App`/`Core` boundary that accepts strongly-typed requests, and a `Renderer` module that takes domain outputs and formats them.
- **Specify the Shim Contract:** Update Step 2 to explicitly state the rules for shims. (e.g., "Shims must only map legacy CLI args to the new `AppRequest` struct and map the `AppResponse` to legacy stdout/stderr outputs. No business logic may exist in the shim.")
- **Select the Slice:** Identify exactly which CLI command(s) will be the guinea pig for this first slice to prove the boundary works.
- **Add Verification Steps:** Include explicit steps for adding unit tests for the new parsers/renderers and integration tests to verify the shimmed legacy CLI still passes existing E2E workflows.

**Risk Assessment**
**HIGH**. The plan as currently written is too vague to be executed safely or consistently. It pushes all architectural and scoping decisions down to the implementation phase, which drastically increases the risk of creating a messy, ad-hoc boundary that fails to meet the goal of preventing another "legacy-style command tree." The plan needs to be revised to include concrete technical designs before implementation begins.

---

## Claude Review

The review is complete above. The key takeaway: the plan's direction is correct but it needs concrete interface definitions, file paths, and at least one worked example to be safely implementable without ambiguity creeping across later phases.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Risk posture could not be parsed reliably from the reviewer output; inspect the sections above.

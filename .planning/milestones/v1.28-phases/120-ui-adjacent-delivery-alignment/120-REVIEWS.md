---
phase: 120
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T20:08:26.697Z
plans_reviewed: [120-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 120

## Gemini Review

Here is the review of the provided implementation plan.

### Summary

Plan `120-01-PLAN.md` is currently too abstract to be an executable engineering document. While it correctly captures the high-level business goals and constraints of the phase (aligning UI-adjacent surfaces to native entrypoints while preserving compatibility), it essentially restates the phase context without providing any concrete implementation details. It lacks specific file targets, module boundaries, architectural mechanisms for the alignment, and a technical validation strategy. 

### Strengths

- **Alignment with Context:** The plan accurately reflects the decisions outlined in the `CONTEXT.md`, specifically the focus on gateway delivery and preserving compatibility.
- **Safety Conscious:** Explicitly calls out the requirement to preserve compatibility for shipped operator surfaces, reducing the risk of breaking existing workflows.
- **Clear Milestone Gate:** Correctly identifies that the roadmap should not advance to the `4/8` completion state until the end-to-end alignment is explicitly verified.

### Concerns

- **HIGH:** Lack of Technical Specificity. The plan does not identify the actual "UI-adjacent operator surfaces" (e.g., specific API routes, Control UI endpoints, or IPC channels) or the target "native entrypoints" (e.g., `crates/gateway`, `crates/cli`).
- **HIGH:** Unactionable Steps. The steps provided ("Align...", "Preserve...", "Verify...") are goals, not implementation steps. An engineer cannot execute this plan without doing the architectural design and scoping work themselves.
- **HIGH:** Missing Verification Strategy. There are no technical steps outlining how to test the alignment or how to prove that legacy compatibility has been successfully preserved (e.g., E2E tests, unit tests, manual UI flows).
- **MEDIUM:** Ambiguous Scope. Because the specific surfaces are not enumerated, the boundary of what is and isn't included in this "second CLI slice" is undefined, which could lead to scope creep or missed components.

### Suggestions

- **Enumerate Target Surfaces:** Update the plan to explicitly list the UI-adjacent components being targeted (e.g., specific gateway handlers, frontend-facing API controllers) and map them to their new native entrypoints.
- **Detail the Implementation Mechanism:** Describe *how* the alignment will happen. Will this involve creating new adapter traits? Rerouting existing API endpoints to new native dispatchers? Modifying the CLI's core command loop to accept programmatic UI input?
- **Define the Compatibility Layer:** Specify the exact technical approach for preserving legacy compatibility. Will legacy routes be kept alive side-by-side, or will legacy payloads be translated into native commands under the hood?
- **Add Concrete Verification Steps:** Include specific testing requirements, such as running the existing UI E2E suite against the new native entrypoints, or adding integration tests that verify UI-adjacent payloads trigger the correct native CLI behavior.
- **Identify Affected Crates/Files:** List the primary files or crates that will be modified (likely within `crates/gateway`, `crates/app`, or `crates/cli`).

### Risk Assessment

**HIGH**. The risk is high because the plan is currently not executable. If handed to an engineer or an autonomous agent, it provides no structural guidance on how to safely mutate the codebase. The lack of detailed scope and technical verification greatly increases the chance of breaking existing UI integrations or creating divergent code paths instead of the intended alignment. The plan needs to be rewritten with concrete architectural and implementation steps before execution begins.

---

## Claude Review

# Review: Phase 120-01 — UI-Adjacent Delivery Alignment

## Summary

This is an extremely thin plan — three high-level bullet points with no concrete file paths, no specific surfaces named, no code changes described, and no verification criteria. It reads more like a restated phase goal than an actionable implementation plan. For a project at this maturity (v1.47, 43 crates), a plan this vague provides almost no guidance for execution.

## Strengths

- Correctly identifies the core tension: UI-adjacent surfaces shouldn't have separate ownership from CLI/gateway native paths
- Keeps scope bounded to alignment rather than new feature work
- Explicitly gates roadmap progress on truthful completion (`4/8`)

## Concerns

- **HIGH** — No concrete surfaces identified. Which UI-adjacent entrypoints are being aligned? Control UI routes? WebSocket handlers? Browser automation UI? Without naming them, the plan is unverifiable.
- **HIGH** — No file paths or code references. A project with 43 crates and extensive gateway/CLI surfaces needs specificity about what's changing and where.
- **HIGH** — No definition of "native entrypoint" vs "legacy command ownership" in concrete terms. What constitutes legacy ownership that needs to change?
- **MEDIUM** — No verification steps. How does an operator or reviewer confirm alignment was achieved?
- **MEDIUM** — Step 3 is a progress-tracking assertion, not an implementation step.
- **LOW** — No mention of whether this requires API/route changes visible to existing consumers.

## Suggestions

- Enumerate the specific UI-adjacent surfaces (by file path and route) that currently depend on legacy command ownership
- For each surface, specify the target native entrypoint it should align to
- Add a verification section: what commands/routes to exercise, what to assert
- Replace step 3 with an actual implementation step; move the roadmap-progress gate to verification
- Reference the existing native delivery entrypoints from phases 33-34 (`crates/gateway/`, `crates/cli/`) so the alignment target is concrete

## Risk Assessment

**HIGH** — The plan is too abstract to execute confidently or review meaningfully. There's significant risk of scope ambiguity during implementation, leading to either under-delivery (claiming alignment without substance) or scope creep (touching surfaces that weren't intended). The phase goal is reasonable, but the plan needs a concrete inventory of what's moving and where it's landing.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Reviewers converged on an overall HIGH risk posture.

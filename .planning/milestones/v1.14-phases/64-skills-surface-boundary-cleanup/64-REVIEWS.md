---
phase: 64
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T18:29:45.452Z
plans_reviewed: [64-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 64

## Gemini Review

# Review of Phase 64: Skills Surface Boundary Cleanup

## Summary
Plan 64-01 provides a clear, focused approach to extracting the read-only compiled-skill overview logic from the heavily overloaded `skills.rs` CLI module into a shared application-layer service in `openrustclaw-app`. The plan correctly targets the duplication between `skills.rs` and `start.rs` (MCP registration), which validates the choice of this specific slice as a useful shared boundary. However, the plan fails to address two of the three explicit success criteria for the phase: updating contributor guidance and explicitly preserving the remaining `skills.rs` cleanup debt.

## Strengths
- **Highly Focused Scope:** The plan strictly adheres to the constraint of extracting a single, well-defined slice (the read-only compiled-skill overview) rather than attempting a risky, wholesale rewrite of the 5,000+ line `skills.rs` file.
- **Immediate Value:** Targeting logic duplicated across the CLI (`skills.rs`) and MCP server (`start.rs`) immediately proves the utility of the new application-layer service boundary.
- **Test-Driven Verification:** The plan explicitly calls out targeted regression tests to ensure the existing CLI and MCP behavior remains intact after the refactoring.

## Concerns
- **[HIGH] Missing Success Criteria:** The plan does not include steps to update contributor guidance to point to the new seam, nor does it explicitly document or preserve the remaining cleanup debt for `skills.rs`. Both were explicit success criteria for Phase 64.
- **[MEDIUM] Lack of Architectural Detail:** The plan lacks specifics regarding the data structures, traits, or error types the new service will expose. Establishing a "stable service seam" requires careful design of the interface boundary to ensure it doesn't leak CLI-specific concepts into the application layer.
- **[LOW] Error Handling Ambiguity:** Moving logic from a CLI command to a core application service usually requires translating CLI-specific `anyhow::Error` or exit-oriented errors into structured domain errors. The plan does not mention error handling translation.

## Suggestions
- **Add a Documentation Step:** Explicitly add a step to update `ARCHITECTURE.md`, `CONTRIBUTING.md`, or a specific ADR to explain the new `openrustclaw-app` service seam and direct future skill-related read operations there.
- **Document Remaining Debt:** Add a step to inventory the remaining mutation-heavy paths (install, auth-plugin, voice-plugin) in `skills.rs` and record them in the project's cleanup backlog or as `TODO` comments linking to tracking issues.
- **Define the Interface:** Briefly outline the expected trait or struct interface for the `CompiledSkillOverviewService` to ensure it remains decoupled from CLI/MCP specific context (e.g., passing in plain paths or domain identifiers rather than CLI argument structs).
- **Address Error Types:** Specify that the new service should return structured domain errors defined in `openrustclaw-app`, which the CLI and MCP layers will map to their respective user-facing error formats.

## Risk Assessment
**LOW**
The technical execution risk is low because this is a pure refactoring of read-only logic with existing test coverage to rely on. The primary risk is organizational—failing to document the new pattern and the remaining debt—which would defeat the overarching goal of changing contributor behavior around the `skills.rs` hotspot. Addressing the suggestions above will mitigate this risk completely.

---

## Claude Review

The review is complete above. The plan is **low risk** and well-scoped — the main improvement opportunity is adding more specificity about which exact functions move and clarifying the test strategy.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=LOW.

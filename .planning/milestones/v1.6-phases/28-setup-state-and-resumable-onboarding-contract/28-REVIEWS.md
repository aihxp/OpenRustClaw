---
phase: 28
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T17:33:28.924Z
plans_reviewed: [28-01-PLAN.md, 28-02-PLAN.md]
---
# Cross-AI Plan Review — Phase 28

## Gemini Review

Here is the cross-AI review of the implementation plans for Phase 28: Setup State and Resumable Onboarding Contract.

### 1. Summary

The plans for Phase 28 are conceptually sound and well-aligned with the project's architectural direction. By moving onboarding state from transient process memory to a durable contract (`setup-state.json` under `.claw/control/`), the plans effectively solve the "fresh start" problem. Plan 28-01 establishes the necessary foundation by defining the state contract and integrating it with `onboard.rs` and `doctor.rs`, while Plan 28-02 builds on this to provide a resumable user experience with clear standard, advanced, and custom paths. The scope is appropriately constrained, deferring deeper repair actions and dashboard rendering to future phases.

### 2. Strengths

*   **Architectural Alignment:** Reusing the existing `.claw/control/` workspace for the `setup-state.json` manifest prevents fragmentation and avoids creating a secondary settings root.
*   **Unified Source of Truth:** Ensuring both `onboard` and `doctor` commands consume the exact same durable state contract will prevent behavioral drift and inconsistent reporting.
*   **Scope Discipline:** Explicitly deferring richer repair actions, UI dashboard rendering, and unrelated documentation updates keeps the implementation focused strictly on the state contract and resumability.
*   **Integrated Custom Path:** Keeping the custom setup path integrated into the main flow, rather than branching it off into a disconnected system, reduces maintenance overhead and ensures all paths output the same state schema.

### 3. Concerns

*   **[HIGH] Lack of Explicit "Reset" Mechanism:** Plan 28-02 mentions offering a resume when unfinished work is detected, but it does not explicitly specify how a user can forcefully bypass the resume prompt to start completely fresh (e.g., a `--force-reset` flag or a clear prompt option). Without this, users might get stuck in a broken setup state.
*   **[MEDIUM] Schema Versioning and Migration:** Plan 28-01 does not mention versioning the `setup-state.json` schema. As the platform evolves (e.g., new required steps are added in future versions), an unversioned state file could cause panics or silent failures if a user updates their CLI binary mid-onboarding.
*   **[MEDIUM] Corrupted State Handling:** There is no explicit mention in the plans of how `onboard.rs` or `doctor.rs` should gracefully handle a malformed, manually edited, or corrupted `setup-state.json` file.
*   **[LOW] Concurrency/Locking:** If a user accidentally runs two onboarding/doctor commands simultaneously in different terminal windows, there could be race conditions writing to the `setup-state.json`. 

### 4. Suggestions

*   **Define a Versioned Schema:** Ensure the Rust struct serializing to `setup-state.json` includes a `version` field from day one. Implement a basic fallback or migration strategy if the CLI encounters an older version.
*   **Add a Reset Action:** Explicitly include a "Start Over" option in the interactive resume prompt and a `--reset` flag in the CLI arguments for Plan 28-02 to allow operators to clear the durable state and begin anew.
*   **Implement Robust Deserialization:** In Plan 28-01, ensure that deserialization of `setup-state.json` uses strict error handling (e.g., via `serde`). If the file is unreadable or malformed, gracefully prompt the user to reset rather than crashing the CLI.
*   **Specify Step Granularity:** For the "custom" path in Plan 28-02, briefly define in the plan how granular these steps are (e.g., are they predefined blocks like "configure_llm", "setup_db", or arbitrary actions?). This will help ensure the state machine remains predictable.

### 5. Risk Assessment

**Overall Risk Level: LOW**

**Justification:** The implementation is primarily isolated to local file I/O (`setup-state.json`) and CLI routing logic within existing commands (`onboard.rs`, `doctor.rs`). It does not require complex network interactions, deep systemic refactoring, or risky migrations of production data. The primary risks are related to edge cases in UX (getting stuck in a resume loop) and schema parsing, which are easily mitigated by incorporating the suggestions above before implementation begins. The plans successfully meet the phase goals defined in `CONTEXT.md`.

---

## Claude Review

The review is complete above. Both plans are sound with low overall risk — the main actionable items are adding schema versioning and atomic writes to 28-01, and constraining custom step selection in 28-02 to avoid scope creep.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=LOW.

---
phase: 18
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T16:40:14.133Z
plans_reviewed: [186-01-PLAN.md, 186-02-PLAN.md]
---
# Cross-AI Plan Review — Phase 18

## Gemini Review

Here is the structured review of the implementation plans for Phase 186.

### 1. Summary
The plans for Phase 186 effectively capture the goal of discovering local agent backends safely without resorting to credential scraping, and they properly sequence the work into a core capability (Plan 01) followed by broader integration (Plan 02). Plan 01 is well-scoped and outlines a solid TDD approach for building the catalog and exposing it in the `models` CLI. However, Plan 01 lacks considerations for subprocess timeouts and parallel execution, which could degrade the CLI experience. Plan 02 is structurally deficient, completely missing the required `<tasks>` section, rendering it unexecutable by an autonomous agent.

### 2. Strengths
* **Clear Separation of Concerns:** Plan 01 correctly splits the workload into a reusable core domain service (`agent_backend_catalog.rs`) and a single initial consumer (`models.rs`), preventing scope creep.
* **Strict Compliance Boundary:** The approach strictly adheres to the decisions (D-03, D-04) to use safe local probes (`--help`, `--version`, auth status commands) rather than invading cached credentials or browser sessions.
* **Truthful Surfacing:** Explicitly classifying states into typed capabilities (e.g., "DetectionOnly", "Candidate", and "Ready") ensures operator-facing surfaces don't overpromise execution readiness.
* **Logical Sequencing:** Breaking the work into two waves (01 for discovery/models, 02 for onboarding/inspect) prevents a massive "big bang" PR, allowing the catalog to be validated in a read-only command before wiring it into complex onboarding state logic.

### 3. Concerns
* **Missing Task Definitions [HIGH]:** `186-02-PLAN.md` entirely omits the `<tasks>` section. An autonomous agent will fail to execute this plan because it contains no actionable steps, behavioral expectations, or file-specific instructions.
* **Subprocess Hang Risk [HIGH]:** Probing third-party CLI tools via `Command::new` can be risky if a tool unexpectedly prompts for interactive input (e.g., a first-run EULA, interactive login prompt, or hanging on a bad network request). Without explicit timeouts in Plan 01, `openrustclaw models` could hang indefinitely.
* **Synchronous Latency [MEDIUM]:** Probing 4+ local CLIs sequentially will add noticeable latency to CLI commands that invoke the catalog. Plan 01 does not specify using parallel execution (e.g., `rayon` or `tokio::spawn`), which could make the CLI feel sluggish.
* **Ambiguous Scope in Plan 02 [MEDIUM]:** `crates/app/src/control_registry.rs` is listed in Plan 02's `files_modified`, but without tasks, it's unclear why the control registry needs modification. Are we adding a new field for detected backends, or updating autonomy policies?

### 4. Suggestions
* **Draft the `<tasks>` for Plan 02:** Update `186-02-PLAN.md` to explicitly define the implementation steps for `onboard.rs`, `inspect.rs`, `setup_lifecycle.rs`, and `control_registry.rs`. Include expected behaviors and verification commands for each task.
* **Enforce Subprocess Timeouts:** In Plan 01, explicitly instruct the agent to wrap `Command::new` probes with a strict timeout (e.g., 500ms - 2s) to prevent unresponsive third-party CLIs from blocking the OpenRustClaw thread.
* **Parallelize Discovery Probing:** Update Plan 01 to require parallel execution of the `BackendProbe` inspections to minimize the performance penalty of spawning multiple subprocesses.
* **Clarify `control_registry.rs` Modifications:** In the newly added tasks for Plan 02, explicitly state what schema changes (if any) are required in the control registry to accommodate delegated local agents.

### 5. Risk Assessment
**MEDIUM**

**Justification:** The architectural design is safe and aligns perfectly with the phase context, avoiding high-risk compliance issues. However, the operational risk of hanging the CLI via unbounded synchronous subprocesses in Plan 01, combined with the missing execution instructions in Plan 02, means the plans cannot be safely or successfully executed by an autonomous agent in their current state. Minor plan revisions are required.

---

## Claude Review

# Cross-AI Review: Phase 186 — Local Agent Discovery and Compliance Inventory

## Plan 186-01: Shared Local-Agent Discovery Catalog

### Summary
Solid first slice that introduces a typed discovery catalog with safe local probing and surfaces it through the existing `models` command. The implementation reuses `tool_host_service.rs` parsing utilities and keeps the capability model honest about ambiguous backends. Well-scoped for a wave-1 deliverable.

### Strengths
- Reuses existing `ToolHostService` parsing instead of duplicating help/version extraction
- Typed readiness model (`Ready`/`Candidate`/`DetectionOnly`/`Unavailable`) prevents boolean-flattening of nuanced states
- Auth probing uses only documented CLI surfaces (`auth status`, `login status`), no credential scraping
- Cursor gets `Supported` capability for both model discovery and delegated execution via documented `cursor agent` subcommands, which is more truthful than the original "detection-only" framing in the context doc
- TDD structure with fake executables keeps tests deterministic without requiring real vendor CLIs

### Concerns
- **MEDIUM**: `env::set_var("PATH", ...)` in tests is `unsafe` and not thread-safe. Concurrent test execution could produce flaky results. The code already uses `unsafe` blocks, so the authors are aware, but parallel `cargo test` runs touching PATH are inherently racy.
- **LOW**: `run_command` uses `Stdio::null()` for stdin but doesn't set a timeout. A hung vendor binary (e.g., `gemini` waiting for interactive input) would block discovery indefinitely.
- **LOW**: `strip_ansi_sequences` handles CSI sequences but not OSC or other escape types. Unlikely to matter for `--version`/`--help` output in practice.

### Suggestions
- Consider adding a process timeout (2-5s) to `run_command` to match the existing `check_ollama_with_timeout` pattern
- The `discover()` method runs probes sequentially; if any binary hangs, it blocks all subsequent probes. Could be parallelized with `tokio::task::spawn_blocking` in a future pass, but fine for now.

### Risk Assessment
**LOW** — Clean scope, good test coverage, no security boundary violations. The PATH mutation in tests is the only real concern and it's already handled with save/restore.

---

## Plan 186-02: Catalog Reuse in Onboarding and Inspect

### Summary
Wave-2 plan to propagate the shared catalog into onboarding and inspect surfaces. Notably underspecified compared to 186-01 — no task breakdown, no TDD behavior specs, no explicit file-level actions. This is more of a directional intent than an executable plan.

### Strengths
- Correct dependency ordering (depends on 186-01)
- Right instinct to close the catalog split immediately rather than letting two source-of-truth tables persist
- Verification commands are specific and testable

### Concerns
- **HIGH**: No task decomposition or behavior specifications. The plan says "reuse the catalog" but doesn't specify *how* onboarding should change its lane selection, whether `inspect` should add a new section, or what `setup_lifecycle.rs` and `control_registry.rs` changes are needed. This makes review of correctness impossible.
- **MEDIUM**: `setup_lifecycle.rs` and `control_registry.rs` are listed as modified files but have no connection to discovery in their current implementations. The plan doesn't explain what changes these files need or why.
- **MEDIUM**: The `onboard.rs` surface is described in CLAUDE.md as a "compatibility-heavy surface" — modifications there carry higher risk of regression without explicit scope boundaries.
- **LOW**: No mention of whether delegated backends should appear as selectable onboarding lanes or only as informational readiness indicators.

### Suggestions
- Flesh out with explicit tasks and behavior specs matching 186-01's level of detail before execution
- Clarify whether this plan persists discovery results to setup lifecycle state or just renders them transiently
- Specify which `inspect` section gains catalog data and what the output format looks like
- Consider whether `control_registry.rs` changes belong in this phase or are premature

### Risk Assessment
**MEDIUM** — The intent is sound but the plan is too vague to execute confidently. The gap between "reuse the catalog" and "here's exactly what changes in onboarding flow" needs to be closed before wave 2 starts. Risk of scope creep into onboarding persistence and journey repair (explicitly deferred in CONTEXT.md) is real without tighter task boundaries.

---

## Overall Phase Assessment

The phase achieves its stated goal through plan 01, which is well-structured and already shipped based on the code I reviewed. Plan 02 needs more specificity before execution to avoid drifting into deferred work. The compliance boundary (no token scraping, detection-only for ambiguous surfaces) is consistently respected across both plans.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=MEDIUM.

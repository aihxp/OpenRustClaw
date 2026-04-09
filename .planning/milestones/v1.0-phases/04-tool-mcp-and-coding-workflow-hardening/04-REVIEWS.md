---
phase: 4
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T16:02:13.424Z
plans_reviewed: [04-01-PLAN.md, 04-02-PLAN.md, 04-03-PLAN.md]
---
# Cross-AI Plan Review — Phase 4

## Gemini Review

Here is the structured review for the Phase 4 plans based on the provided project context and requirements.

### Review of 04-01-PLAN.md (Durable Execution Ledger)

**Summary**
This plan introduces a durable, file-backed ledger for tool and MCP executions, along with inspection endpoints to surface these records to the operator. It correctly identifies the need to move away from raw logs to structured, typed failure and success classifications, providing a strong foundation for tool observability.

**Strengths**
- Directly addresses the core requirement (TOOL-01, TOOL-02) to make tool execution failures operator-meaningful.
- Reuses existing inspection paradigms (typed inspection reports) rather than inventing a new disjoint operator surface.
- Integrates well with the existing `.claw/control` directory conventions for workspace-local storage.

**Concerns**
- **Architectural Regression (HIGH):** The project context indicates that `start.rs` and `inspect.rs` have been heavily migrated and reduced to mere "HTTP adapters" and "workspace adapters" in recent greenfield milestones (v1.14 - v1.38), with business logic moving to `openrustclaw-app`. Adding execution persistence logic directly into `start.rs` risks violating the new native-delivery architecture.
- **Unbounded Storage (MEDIUM):** There is no mention of ledger rotation, TTL, or bounding the number of retained execution records. Unbounded ledger growth could degrade inspection performance or exhaust local workspace storage.

**Suggestions**
- Move the core persistence and business logic for the execution ledger into the `openrustclaw-app` service layer, keeping `start.rs` and `inspect.rs` strictly as transport/HTTP adapters.
- Add a requirement to the implementation task to bound the ledger size (e.g., "Retain only the last 100 executions" or a time-based TTL).

**Risk Assessment:** **HIGH** (Due to the significant risk of architectural regression against the recent greenfield and native-delivery migrations).

---

### Review of 04-02-PLAN.md (Bounded Coding Workflows)

**Summary**
This plan focuses on confining coding actions (inspect, edit, run) to the workspace and ensuring they emit stable, reviewable artifacts. It bridges the gap between the agent's runtime loop and the CLI's coding surfaces (like Cursor integration), ensuring actions are recoverable and auditable.

**Strengths**
- Establishes a strict workspace boundary, preventing the agent from acting as an unbounded, machine-wide entity.
- Shifts the trust model from "agent narrative" to "verifiable artifacts" (diffs, command outputs).
- Effectively targets both the `agent` crate (for runtime bounds) and the `cli` crate (for artifact emission).

**Concerns**
- **Path Traversal Security (HIGH):** While "workspace-bounded" is specified, the plan does not explicitly mandate path normalization and directory traversal prevention (e.g., `../../` attacks) in the `runtime.rs` boundary checks.
- **Concurrency Collisions (MEDIUM):** If multiple tools or coding workflows run concurrently, writing to a single shared artifact file could result in race conditions or interleaved evidence.

**Suggestions**
- Explicitly instruct the implementation to enforce strict path jailing and path normalization to guarantee the workspace boundary is cryptographically sound.
- Define a run-specific directory structure (e.g., `.claw/runs/<execution_id>/`) to isolate artifacts and prevent concurrent write collisions.

**Risk Assessment:** **MEDIUM** (The concept is sound, but security specifics around file-system bounding need explicit enforcement).

---

### Review of 04-03-PLAN.md (Expose and Document Audit Path)

**Summary**
This plan closes the operator loop by surfacing the newly created execution and coding artifacts in the Control UI and updating the canonical documentation. It ensures that the technical telemetry implemented in the previous plans translates into actual operator visibility.

**Strengths**
- Strong emphasis on the operator journey, ensuring that the new audit capabilities are discoverable from the first run.
- Aligns perfectly with the project's strict documentation contract (docs must reflect shipped behavior).
- Includes cross-surface E2E verification to ensure the UI, CLI, and backend remain in sync.

**Concerns**
- **File List Mismatch (LOW):** `docs/src/guides/memory.md` is listed in the `files_modified` frontmatter but is not addressed or mentioned in any of the specific tasks.
- **UI API Coupling (MEDIUM):** The Control UI task mentions rendering the artifact summaries but does not specify how the UI will securely fetch this newly structured data, assuming the backend endpoints from 04-01 are sufficient without tailoring them for the frontend.

**Suggestions**
- Remove `docs/src/guides/memory.md` from the modified files list, or add a specific task detailing how tool executions interact with the memory context guide.
- Explicitly verify that the `inspect.rs` endpoint updates from Plan 01 provide a UI-friendly pagination or summary format to prevent the Control UI from loading massive artifact payloads directly into the browser.

**Risk Assessment:** **LOW** (Standard documentation and UI surfacing, with minimal technical risk assuming previous plans are executed safely).

---

## Claude Review

The review is complete above. To summarize the key findings:

- **04-01** (execution ledger): **LOW risk** — needs retention policy and storage format pinned
- **04-02** (coding auditability): **MEDIUM risk** — git/diff interaction and partial-failure recovery undefined  
- **04-03** (expose and document): **LOW risk** — stale `memory.md` reference, otherwise clean
- **Phase overall**: **LOW-MEDIUM** — coherent progression, achieves all four requirements, no scope creep

The biggest actionable item: pin the storage format (recommend SQLite table) in 04-01 so downstream plans build on a defined contract.

Ready to persist this review to the repo when you'd like to proceed.

---

## Consensus Summary

### Agreed Strengths
- Reviewers generally agreed the plans are structured and traceable to the phase goal, but they emphasized different strengths.

### Agreed Concerns
- No clearly shared concern theme was detected across reviewers; reviewer-specific concerns are preserved above.

### Divergent Views
- Reviewers converged on an overall HIGH risk posture.

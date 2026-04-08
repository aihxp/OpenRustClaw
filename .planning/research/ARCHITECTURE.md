# Architecture Patterns

**Domain:** v1.43 learning loop, deeper memory, skill improvement, and God Mode inside the existing Rust-first runtime
**Researched:** 2026-04-07

## Recommended Architecture

Keep Rust as the system of record for all durable learning state, memory state, policy, and God Mode enablement. Use the Python sidecar only for bounded asynchronous workflows that summarize, score, or evaluate candidate artifacts. Do not let the sidecar become the source of truth for memory, learning, or autonomy state.

The existing split is already usable:
- `crates/app/src/*` holds transport-agnostic use-case logic.
- `crates/db/src/*` and `crates/memory/src/*` hold durable memory state and policy.
- `crates/cli/src/commands/control.rs` and `crates/cli/src/commands/enterprise_autonomy.rs` already own file-backed runtime policy and the full-autonomy override lane.
- `crates/cli/src/commands/orchestrate.rs` already emits run receipts, reflection candidates, supervision state, and promotes reflection into lessons.

The right v1.43 shape is therefore:

```text
runtime receipts + tool audit + memory events
    -> Rust learning ingest service
    -> durable candidate stores in SQLite
    -> operator review/promotion surfaces
    -> one of:
       - decision lesson in `.claw/control/lessons/*.yaml`
       - structured user/operator model artifact in SQLite
       - core-memory projection in `core_memory`
       - archive summary in `memory_archive`
       - skill improvement proposal under `.claw/skills/proposals/`

request execution
    -> Rust memory retrieval/assembly service
    -> ranked recall + model artifact projection + bounded core memory
    -> existing `AgentRuntime`

God Mode
    -> existing enterprise full-autonomy manifest/events
    -> run-scoped overlay on routing/policy
    -> same runtime, stronger grants, stronger audit
```

`God Mode` should be implemented as a stricter named overlay on the existing enterprise full-autonomy lane in `crates/cli/src/commands/enterprise_autonomy.rs`, not as a second runtime stack. The current runtime already expresses the needed execution knobs through autonomy level, approval policy, kill switch, and run receipts. Duplicating that into a separate runtime would create policy drift fast.

## Component Boundaries

| Component | Responsibility | Communicates With |
|-----------|---------------|-------------------|
| `crates/app/src/memory_retrieval.rs` | New app service that assembles the runtime memory pack from core, recall, archive, and structured user/operator model artifacts | `crates/db`, `crates/memory`, `crates/agent` |
| `crates/app/src/learning_loop_control.rs` | New app service that converts orchestration receipts, tool audit, and successful runs into reviewable learning candidates | `crates/cli/src/commands/orchestrate.rs`, `crates/app/src/tool_execution_audit.rs`, `crates/db` |
| `crates/app/src/learning_review.rs` | New app service that promotes or rejects candidates into lessons, memory artifacts, projections, or skill proposals | `crates/cli/src/commands/control.rs`, `crates/db`, `crates/skills` |
| `crates/app/src/skill_improvement_control.rs` | New app service that generates reviewable skill-improvement proposals and compiles them into the existing compiled-skill lane only after approval | `crates/app/src/skill_registry_mutation.rs`, `crates/skills`, `.claw/skills/compiled` |
| `crates/db/src/learning_store.rs` | New SQLite store for learning candidates, promotions, evaluation evidence, and rollback metadata | `crates/app/src/learning_*`, control routes |
| `crates/db/src/model_store.rs` | New SQLite store for structured `user_model` and `operator_model` artifacts; these are not raw prompt blobs | `crates/app/src/memory_retrieval.rs`, sidecar workflows |
| `crates/memory/src/*` | Continues to own write policy, decay, recall/core/archive semantics, and should absorb ranking/assembly helpers instead of spreading them into delivery code | `crates/db`, `crates/agent` |
| `crates/cli/src/commands/start.rs` | Route wiring only. Add new control routes, but keep business logic in `crates/app` | `crates/app/src/*` |
| `sidecar/src/workflows/memory_maintenance.py` and new sidecar evaluators | Asynchronous summarization, candidate scoring, and maintenance workflows only | Rust internal memory/learning APIs via gateway or langbridge |
| `crates/cli/src/commands/enterprise_autonomy.rs` | Remains the God Mode policy owner: enable, disable, kill switch, event log, baseline restore | `crates/cli/src/commands/control.rs`, orchestration receipts |

## Stores

### Keep

| Store | Role | Why Keep It |
|-------|------|-------------|
| SQLite `memory_entries`, `core_memory`, `memory_archive` through `crates/db/src/{memory_store.rs,core_memory_store.rs}` | Durable memory tiers | Already matches the project rule that DB access goes through `crates/db` |
| File-backed control registry under `.claw/control` from `crates/cli/src/commands/control.rs` | Runtime mode, lessons, profile manifests | Existing routing and lesson matching already depend on it |
| `.claw/control/enterprise/full-autonomy.json` and `.claw/control/enterprise/full-autonomy-events.jsonl` | God Mode enablement and audit | Existing full-autonomy lane already uses these files cleanly |
| `.claw/control/orchestration-runs` and `.claw/control/orchestration-active` | Run receipts, events, checkpoints, reflection evidence | Best current evidence source for learning extraction |
| `.claw/skills/compiled` | Reviewed executable skill artifacts | Existing compiled-skill runtime and MCP surfaces already consume this |

### Add

| Store | Role | Why |
|-------|------|-----|
| `learning_candidates` table | Durable queue of reflection candidates, success-derived lessons, model updates, and skill proposals | Reflection promotion currently jumps straight from receipt to lesson; v1.43 needs a reviewable middle state |
| `learning_promotions` table | Audit trail for candidate -> artifact promotion, rejection, rollback, and supersession | Needed for explicit audit and recovery |
| `user_models` and `operator_models` tables | Structured cross-session traits, preferences, patterns, and stable summaries | These should not live as raw prompt text or ad hoc core-memory keys |
| `memory_retrieval_events` table | Explainability for ranking, projection, and prompt-pack assembly | Required if memory gets deeper and less obvious |
| `.claw/skills/proposals/` | Human-reviewable skill patches/specs before install or compile | Prevents hidden self-modification |

Recommendation: lessons stay file-backed in `.claw/control/lessons/*.yaml` because `crates/cli/src/commands/orchestrate.rs` already matches and injects them during routing. Candidate state belongs in SQLite because it is event-heavy, query-heavy, and should be filterable by status, source, and confidence.

## Control Surfaces

The safest path is to extend existing control families instead of inventing new top-level planes.

| Surface | Use |
|---------|-----|
| `/control/orchestration/runs/{receipt_id}/reflection-candidates/{index}/promote` in `crates/cli/src/commands/start.rs` | Keep as the seed promotion API, but route it through a new learning-review service that can promote to more than lessons |
| `/control/autonomy/lessons` in `crates/cli/src/commands/start.rs` | Keep for active lesson inspection and mutation |
| `/control/ui` and `crates/cli/src/commands/control_ui.html` | Primary operator surface for candidate review, user/operator model inspection, and God Mode warnings or toggles |
| `/internal/memory/*` in `crates/gateway/src/server.rs` | Keep as sidecar bridge for maintenance and summarization workflows; add learning-only internal endpoints rather than direct DB/file mutation from Python |
| `openrustclaw skills ...` and compiled-skill MCP surfaces | Read and inspect compiled skills; add proposal review and apply steps here instead of auto-updating live skills |
| MCP tools in `crates/mcp/src/server.rs` | Expose read-only learned-artifact inspection first; do not expose God Mode enablement or skill mutation by default |

Recommendation: God Mode activation should remain available only through enterprise-authenticated CLI/control surfaces, not MCP. MCP is too easy to widen accidentally.

## Data Flow

### Learning Loop

1. `crates/cli/src/commands/orchestrate.rs` writes the run receipt, reflection candidates, checkpoints, and active-run events.
2. A Rust learning-ingest service reads that receipt plus tool audit history from `crates/app/src/tool_execution_audit.rs`.
3. The service writes one or more `learning_candidates` rows with type `lesson`, `user_model_update`, `operator_model_update`, `archive_projection`, or `skill_proposal`.
4. Optional sidecar evaluators score or summarize those candidates asynchronously, but they only write back through Rust-owned internal APIs.
5. Operator review surfaces inspect candidates, accept or reject them, and promotion writes the final artifact into the correct store.
6. Every promotion writes a rollback pointer into `learning_promotions`.

### Deeper Memory Retrieval

1. Before a request reaches `AgentRuntime`, a Rust retrieval service queries:
   - `core_memory`
   - recall memory via `crates/db/src/memory_store.rs`
   - archive summaries
   - structured `user_models` and `operator_models`
2. The retrieval service assembles a bounded memory pack:
   - always-injected core projection
   - optional retrieval summary for the prompt
   - explainable ranked recall list for inspection
3. `AgentRuntime` continues to use the same prompt loop, but consumes a better assembled memory pack and still relies on `memory_search` for deeper fetches.
4. Retrieval explanations are logged so operators can inspect why a memory artifact was included.

### Skill Improvement

1. The learning loop emits a `skill_proposal` candidate, not a live skill mutation.
2. The proposal is stored under `.claw/skills/proposals/` with source receipt id, rationale, capability delta, and risk flags.
3. After explicit review, the proposal is applied through the existing skill mutation and compile path in `crates/app/src/skill_registry_mutation.rs` and `crates/skills/src/compiler.rs`.
4. The compiled artifact lands in `.claw/skills/compiled`, where existing runtime and MCP surfaces can inspect or run it.

### God Mode

1. Operator enables God Mode through the existing enterprise full-autonomy lane in `crates/cli/src/commands/enterprise_autonomy.rs`.
2. The manifest applies a run-scoped overlay to routing and policy from `crates/cli/src/commands/control.rs`.
3. Orchestration receipts record that the run used God Mode and which extra grants were active.
4. Kill switch and baseline restore stay exactly where they are today.

## Patterns to Follow

### Pattern 1: Candidate Before Promotion
**What:** Every learned artifact starts as a durable candidate with provenance, confidence, and rollback metadata.
**When:** Lessons, model updates, archive summaries, skill improvements.
**Example:**
```rust
pub trait LearningReviewSource {
    fn create_candidate(&self, candidate: LearningCandidate) -> Result<String>;
    fn approve_candidate(&self, id: &str, reviewer: &str) -> Result<PromotionReceipt>;
    fn reject_candidate(&self, id: &str, reviewer: &str, reason: &str) -> Result<()>;
}
```

### Pattern 2: Structured Model Artifacts, Tiny Prompt Projection
**What:** Store user/operator modeling as structured rows, then project only the stable high-signal subset into core memory or retrieval summaries.
**When:** Cross-session preferences, habits, operator style, recurring workflow patterns.
**Why:** This preserves the recall-only rule and avoids prompt bloat.

### Pattern 3: Same Runtime, Stronger Overlay
**What:** God Mode changes policy and grants, not the runtime architecture.
**When:** Full autonomy, broad tool access, approval bypass, or stronger learning permissions.
**Why:** The repo already has one coherent autonomy lane with receipts, events, and a kill switch.

### Pattern 4: Rust-Owned Durability, Sidecar-Owned Async Reasoning
**What:** Rust owns writes to durable stores and control manifests; sidecar runs async summarization/evaluation with persisted workflow state.
**When:** Memory maintenance, candidate scoring, skill proposal drafting.
**Why:** This matches the current Rust-first product contract and avoids silent Python-only drift.

## Policy Boundaries

| Boundary | Rule |
|----------|------|
| Recall vs prompt | Never inject raw recall or archive entries directly into the system prompt; only inject curated core memory and bounded retrieval summaries |
| Candidate vs artifact | Reflection candidates, user-model changes, and skill improvements are not live until promoted |
| Sidecar vs source of truth | Sidecar may summarize or score, but Rust persists final state |
| Default mode vs God Mode | God Mode is explicit, reversible, enterprise-gated, and separately audited |
| Memory learning vs self-modification | Memory and lessons may be promoted under policy; executable skill changes always require review |
| Read API vs mutation API | MCP and lightweight inspection routes should be read-only first; mutating surfaces stay in enterprise-authenticated control or CLI paths |

Recommended default policy:
- Auto-allow: archive summarization of existing episodic memories, ranking telemetry, access-count updates.
- Review-required: decision lessons, user/operator model promotions, core-memory projection changes, skill proposals, and all God Mode actions.
- Never auto-allow: live skill install or update, God Mode enablement, hidden policy changes, direct prompt-file mutation.

## Anti-Patterns to Avoid

### Anti-Pattern 1: Letting the Sidecar Own Learning State
**What:** Store candidates, summaries, or memory truth in Python globals or sidecar-local files.
**Why bad:** `sidecar/src/workflows/memory_maintenance.py` and `sidecar/src/workflows/scheduler.py` already show in-memory fallback behavior that is acceptable for experiments, not for system-of-record state.
**Instead:** Keep durable state in `crates/db` and call it through internal Rust APIs.

### Anti-Pattern 2: Promoting Reflection Directly Into Live Behavior Everywhere
**What:** Turn every reflection candidate into an active lesson or skill change immediately.
**Why bad:** Existing reflection candidates are heuristic and currently optimized for operator review, not autonomous activation.
**Instead:** Introduce the candidate store and explicit promotion flow first.

### Anti-Pattern 3: Building a Second “God Mode Runtime”
**What:** New runtime host, new route family, new policy file set.
**Why bad:** It duplicates the enterprise full-autonomy contract already present in `crates/cli/src/commands/enterprise_autonomy.rs`.
**Instead:** Extend the current manifest and event model with stronger grant metadata.

### Anti-Pattern 4: Hiding User/Operator Modeling in Ad Hoc Core Memory Keys
**What:** Stuff everything into `core_memory` because it is easy to render.
**Why bad:** It breaks the 500-token contract and makes memory inspection brittle.
**Instead:** Use structured model stores and project down into core memory selectively.

## Scalability Considerations

| Concern | At 100 users | At 10K users | At 1M users |
|---------|--------------|--------------|-------------|
| Memory retrieval | Current SQLite stores are fine once ranking and result assembly move into a dedicated service | Batch access-count updates, preserve FTS rank, and add retrieval-event telemetry | Revisit SQLite-only assumptions and isolate retrieval into its own service boundary |
| Learning candidates | One SQLite table is enough | Add status indexes and background cleanup for stale candidates | Separate ingestion from review serving |
| Sidecar workflows | Bounded maintenance workflows are acceptable | Persist checkpoints and avoid in-memory scheduler/idempotency state | Treat workflow execution as a separate deployable concern |
| God Mode audit | JSONL events plus run receipts are fine | Add indexed projections for filtering and export | Move audit search to dedicated storage while keeping manifests as control truth |

## Safest Build Order

1. **Retrieval And Inspection First**: add the Rust memory-retrieval service, retrieval-event logging, and deeper memory inspection surfaces before changing learning behavior.
2. **Candidate Store Second**: insert the durable `learning_candidates` and `learning_promotions` layer between reflection evidence and live lessons.
3. **Structured User/Operator Models Third**: add model stores and projection logic into bounded core-memory entries.
4. **Sidecar Evaluation Fourth**: move summarization/scoring into bounded async workflows only after Rust-owned stores and review APIs exist.
5. **Skill Proposal Lane Fifth**: generate reviewed skill proposals, compile them through the existing skill pipeline, and keep executable mutation gated.
6. **God Mode Expansion Last**: extend the current full-autonomy manifest into a clearer God Mode overlay only after the learning and audit surfaces are trustworthy.

This order is the safest because memory retrieval and auditability reduce error amplification before the system starts learning more aggressively, and God Mode multiplies the blast radius of every earlier architectural mistake.

## Sources

- Local project context:
  - `.planning/PROJECT.md`
  - `.planning/codebase/ARCHITECTURE.md`
  - `.planning/codebase/CONCERNS.md`
  - `.planning/codebase/STACK.md`
- Key implementation references:
  - `crates/cli/src/commands/orchestrate.rs`
  - `crates/app/src/orchestration_reporting.rs`
  - `crates/app/src/autonomy_lessons_control.rs`
  - `crates/cli/src/commands/control.rs`
  - `crates/cli/src/commands/enterprise_autonomy.rs`
  - `crates/gateway/src/server.rs`
  - `crates/db/src/memory_store.rs`
  - `crates/db/src/core_memory_store.rs`
  - `crates/memory/src/{policies.rs,context.rs,recall.rs,core_memory.rs,archive.rs}`
  - `crates/app/src/{skill_registry_mutation.rs,compiled_skill_mcp.rs,compiled_skill_overview.rs}`
  - `sidecar/src/{server.py,memory_bridge.py}`
  - `sidecar/src/workflows/{memory_maintenance.py,scheduler.py}`
- External verification:
  - LangGraph memory and persistence docs: https://docs.langchain.com/oss/javascript/langgraph/add-memory

# Phase 181: Hybrid Retrieval and Recall Inspection - Context

**Gathered:** 2026-04-07
**Status:** Ready for planning

<domain>
## Phase Boundary

Improve recall retrieval, recall assembly, and operator inspection so the runtime surfaces the most relevant prior context with explainable ranking and bounded output. This phase is about retrieval quality and visibility, not durable model artifacts, learning-candidate promotion, skill mutation, or God Mode enablement.

</domain>

<decisions>
## Implementation Decisions

### Retrieval ranking
- **D-01:** Phase 181 should harden the existing Rust-owned retrieval path instead of introducing a second memory subsystem.
- **D-02:** Ranking should combine lexical, vector, recency, confidence, and importance signals rather than flattening BM25 or relying on one opaque score.
- **D-03:** Retrieval quality and explainability come before retrieval breadth; do not widen default recall depth until ranking debt is reduced.

### Recall assembly and prompt boundaries
- **D-04:** Retrieved memory should be assembled into concise, deduplicated output with provenance, freshness, and artifact-type metadata.
- **D-05:** The recall-only memory contract remains fixed: raw memory files, raw archive blobs, and large undifferentiated recall dumps must not be injected into the system prompt.
- **D-06:** Recall output should stay tool-driven and bounded, with compact summaries rather than raw storage payloads.

### Inspection and observability
- **D-07:** Operators need an inspectable “why this memory appeared” lane that exposes ranking factors and contributing source artifacts.
- **D-08:** Retrieval changes should produce durable inspection or telemetry artifacts that make regressions debuggable across sessions.

### Scope guardrails
- **D-09:** Phase 181 should not add structured user/operator/project model stores; that belongs to Phase 182.
- **D-10:** Phase 181 should not activate learning-candidate promotion, skill proposals, or God Mode behavior; later phases depend on trustworthy retrieval first.

### the agent's Discretion
- Exact fusion formula, weighting, and normalization strategy across ranking signals
- The most appropriate operator-facing inspection surface shape across CLI, Control UI, or control APIs
- Whether retrieval telemetry lands as an existing memory inspection extension or a new dedicated retrieval-event artifact

</decisions>

<specifics>
## Specific Ideas

- Close the known ranking debt in `crates/db/src/memory_store.rs` before adding more memory depth.
- Reuse the repo’s existing trust-first posture: small always-loaded core memory, on-demand recall, and explicit archive boundaries.
- Keep the retrieval work compatible with future structured model artifacts and learning-candidate review flows.

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone and phase contract
- `.planning/PROJECT.md` — Current milestone intent and active requirements for v1.43
- `.planning/REQUIREMENTS.md` — `RETR-01` through `RETR-04` define the committed retrieval scope
- `.planning/ROADMAP.md` — Phase 181 goal, dependency boundary, and success criteria

### Research
- `.planning/research/SUMMARY.md` — Milestone synthesis and recommended 5-phase ordering
- `.planning/research/FEATURES.md` — Table stakes and anti-features for retrieval inspection
- `.planning/research/ARCHITECTURE.md` — Recommended service boundaries and storage ownership
- `.planning/research/PITFALLS.md` — Retrieval precision, privacy, and silent-degradation risks

### Existing codebase seams
- `.planning/codebase/ARCHITECTURE.md` — Current Rust-first runtime layering and memory boundaries
- `.planning/codebase/CONCERNS.md` — Known ranking and recall debt in the existing memory store
- `crates/db/src/memory_store.rs` — Current FTS/vector recall implementation and score-shaping hotspot
- `crates/memory/src/context.rs` — Retrieval and prompt-context assembly boundary
- `crates/agent/src/prompt.rs` — Prompt contract and recall-tool guidance
- `crates/agent/src/memory_tools.rs` — Current recall and store tools exposed to the runtime
- `crates/cli/src/commands/memory.rs` — Existing operator-facing memory inspection commands
- `crates/app/src/memory_views.rs` — Existing memory inspection and projection seams

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/db/src/memory_store.rs`: existing hybrid memory search path, FTS5 index, and access tracking logic that should be improved rather than replaced
- `crates/db/src/core_memory_store.rs`: stable core-memory persistence seam that must remain small and bounded
- `crates/memory/src/context.rs`: current memory-context assembly path that can host bounded retrieval summaries
- `crates/app/src/memory_views.rs`: existing app-layer memory inspection surface that may be extendable for retrieval explanations
- `crates/cli/src/commands/memory.rs`: current CLI inspection hooks for memory timeline and archives

### Established Patterns
- Rust owns durable state and policy; the sidecar may assist asynchronously but should not become the source of truth
- Memory writes are policy-gated through `crates/memory/src/policies.rs`
- Control and inspection behavior should prefer app services and typed surfaces over ad hoc direct file or DB inspection

### Integration Points
- Retrieval ranking changes should connect through `crates/db`, `crates/memory`, and `crates/agent` rather than only delivery-layer code
- Operator inspection likely lands through the current CLI/control memory surfaces, not a brand-new standalone subsystem

</code_context>

<deferred>
## Deferred Ideas

- Structured user, operator, project, and archive model artifacts — Phase 182
- Learning candidate capture, promotion, and rollback — Phase 183
- Skill proposal generation and verification — Phase 184
- God Mode overlay, audit, and recovery — Phase 185
- Graph-memory browsing or dashboard-heavy retrieval UX — out of scope for this milestone slice

</deferred>

---

*Phase: 181-hybrid-retrieval-and-recall-inspection*
*Context gathered: 2026-04-07*

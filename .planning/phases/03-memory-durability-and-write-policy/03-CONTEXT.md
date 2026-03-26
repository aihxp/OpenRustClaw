# Phase 3: Memory Durability and Write Policy - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Make assistant memory writes durable, policy-governed, and inspectable. This phase hardens the existing `memory_store` and related memory surfaces so the assistant only stores information under an explicit contract instead of vague "important information" heuristics.

</domain>

<decisions>
## Implementation Decisions

### Memory trust boundary
- **D-01:** The main production problem is not raw storage durability alone; it is uncontrolled write behavior. Phase 3 should tighten the assistant write boundary before broadening memory usage.
- **D-02:** `memory_store` should only write when the user clearly asked to remember something or when the fact is obviously durable and user-specific enough to justify recall value.
- **D-03:** Ephemeral conversation details, speculative inferences, and convenience summaries should not be silently persisted through the default assistant memory tool.
- **D-04:** The default assistant lane should prefer under-storing over over-storing when the policy basis is ambiguous.

### Enforcement shape
- **D-05:** Enforce the policy at the built-in tool boundary (`MemoryStoreTool`) instead of relying only on prompt wording.
- **D-06:** The tool contract should require an explicit policy basis or reason for the write so the model has to declare why the memory is durable enough to store.
- **D-07:** Stored memories should carry metadata explaining why the write was allowed, so operators can inspect the decision after the fact.
- **D-08:** Prompt guidance should reinforce the stricter policy, but prompt guidance alone is insufficient.

### Durability and inspection
- **D-09:** Phase 3 should preserve the existing persisted SQLite-backed memory path and focus on making it trustworthy rather than replacing the backend.
- **D-10:** Operators should be able to inspect memory entries and see not just content but also write-basis metadata such as explicit request versus durable fact.
- **D-11:** Existing search and timeline surfaces should become more useful for debugging memory writes without inventing a separate memory governance product.
- **D-12:** Cross-surface docs should explain the stricter memory behavior so the assistant does not appear to remember arbitrary conversation scraps.

### the agent's Discretion
- The exact policy field names and wording are at the agent's discretion as long as the contract is clear, enforceable, and inspectable.
- The boundary between recall-memory metadata inspection and broader operator UX can be split across multiple plans if the enforcement path lands first.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Memory write and prompt enforcement
- `crates/agent/src/memory_tools.rs` — built-in `memory_store` and `core_memory_update` tools, current schema, and write behavior
- `crates/agent/src/prompt.rs` — current prompt guidance for memory search and memory store usage
- `crates/core/src/traits.rs` — `ToolContext` and memory-store trait boundary

### Memory policy and shaping
- `crates/memory/src/policies.rs` — current scoring, TTL, dedupe, and policy primitives
- `crates/memory/src/recall.rs` — prepared memory entry shaping for recall memory
- `tests/integration/src/memory_workflow_test.rs` — current policy and recall-memory coverage

### Operator inspection and docs
- `crates/cli/src/commands/memory.rs` — CLI operator memory inspection surfaces
- `crates/cli/src/commands/start.rs` — control memory timeline and archive API handlers
- `docs/src/guides/memory.md` — current operator-facing memory guidance
- `docs/src/getting-started/quickstart.md` — current examples that show `memory_store` in the default assistant lane

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `MemoryPolicies` already centralizes dedupe, TTL, and importance concepts, so Phase 3 can extend policy semantics without inventing a new subsystem.
- `RecallMemory::prepare_entry` already maps source to importance and TTL semantics, which makes it a useful foundation for stricter write shaping.
- `MemoryStoreTool` is the main built-in write surface for the default assistant lane and currently sets minimal metadata with no explicit write-basis gate.
- Memory timeline and archive inspection surfaces already exist, so operator visibility can build on shipped control and CLI paths.

### Established Patterns
- Built-in assistant tool contracts live in `crates/agent/src/*_tools.rs` and are exposed through typed schemas plus prompt guidance.
- Operator trust improvements in earlier phases favored typed summaries and explicit metadata over hidden heuristics; Phase 3 should do the same for memory writes.
- The current assistant allowlist intentionally stays narrow (`memory_search`, `memory_store`), so tightening `memory_store` has outsized impact on first-run trust.

### Gaps to Close
- `memory_store` currently accepts freeform content and optional importance, then stores directly after dedupe.
- The prompt currently tells the model to remember "important information" without defining a durable-fact or explicit-request boundary.
- Stored memory metadata does not explain whether a write happened because the user explicitly asked or because the assistant judged the fact durable.
- Operator docs still imply more automatic storage than the tightened trust model should allow.

</code_context>

<specifics>
## Specific Ideas

- The strict default for the primary assistant lane should be: search freely, store sparingly, justify every durable write.
- Memory write metadata should be simple enough to inspect in raw JSON but structured enough for future UI rendering.
- A clean first slice is explicit write-policy enforcement for `memory_store`, followed by operator inspection and docs alignment.

</specifics>

<deferred>
## Deferred Ideas

- Broader memory summarization, archival compaction, and cross-model rehydration tuning are outside this phase unless directly needed by the write-policy contract.
- Enterprise retention policy, legal hold, and compliance-specific memory governance remain out of scope for MVP.
- More advanced learned memory extraction can wait until the explicit default policy is trustworthy.

</deferred>

---
*Phase: 03-memory-durability-and-write-policy*
*Context gathered: 2026-03-26*

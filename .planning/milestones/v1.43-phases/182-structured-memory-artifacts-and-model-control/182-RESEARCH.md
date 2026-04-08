# Phase 182 Research: Structured Memory Artifacts and Model Control

## Current state

OpenRustClaw already has:
- a Rust-owned recall store in `crates/db/src/memory_store.rs`
- a bounded always-loaded core-memory tier in `crates/db/src/core_memory_store.rs`
- a prompt boundary that only injects core memory in `crates/memory/src/context.rs`
- archive summaries in `memory_archive`
- operator-facing inspection through `crates/cli/src/commands/memory.rs`, `crates/cli/src/commands/inspect.rs`, and MCP handlers in `crates/cli/src/commands/start.rs`

OpenRustClaw does not yet have:
- a durable typed model-artifact store
- promotion policy for turning evidence into user/operator/project/archive summaries
- projection logic that materializes only a bounded subset of structured artifacts into active context
- typed correction/deactivation/removal paths for those artifacts

## Phase 182 decisions

### D-182-01: Add one Rust-owned durable artifact table

Phase 182 should add a first-class `memory_model_artifacts` table instead of reusing ad hoc file-backed views or stuffing structured artifacts into `core_memory`.

Why:
- `core_memory` is a projection/output tier, not a provenance-preserving source of truth
- `memory_archive` is specifically for consolidated archive summaries and does not cover user/operator/project artifacts
- Phase 182 needs lifecycle state and lineage that the current tables do not provide

### D-182-02: Keep promotion inside the memory store and policy seam

Promotion should enter through `SqliteMemoryStore` methods and be validated by `MemoryPolicies`.

Why:
- the repo instructions require memory writes to respect the memory policy seam
- promotion is a durable write and needs explicit gating plus consistent lineage handling

### D-182-03: Materialize projection into reserved core-memory keys

Instead of widening the prompt builder or adding a second projection subsystem, Phase 182 should sync active model artifacts into bounded reserved `core_memory` slots.

Why:
- the runtime already loads core memory safely
- the prompt boundary remains unchanged: only core memory enters the prompt
- this gives a truthful implementation of “projects only a bounded high-signal subset into active context”

### D-182-04: One active artifact per kind per namespace

Each namespace should have at most one active `user_model`, `operator_model`, `project_memory`, and `archive_summary` artifact at a time. New promotions supersede older active artifacts of the same kind.

Why:
- matches the “distinct durable artifact classes” requirement
- keeps projection bounded and legible
- preserves history while preventing blended profile blobs

### D-182-05: Operators manage artifacts through existing memory/inspect/MCP surfaces

Phase 182 should extend existing CLI and MCP memory surfaces before adding new control UI work.

Why:
- satisfies `MODL-04` without building a new dashboard
- keeps the first implementation testable and phase-scoped

## Implementation shape

### Durable model artifact contract

Add shared types for:
- artifact kind
- lifecycle status
- source lineage references
- promotion decisions
- projection snapshots

### Storage

Add `memory_model_artifacts` with:
- stable id
- namespace
- artifact kind
- summary
- status
- importance and confidence
- source lineage JSON
- promoted_by
- correction notes
- created_at / updated_at / deactivated_at

### Projection

Sync active artifacts into reserved keys:
- `model.user`
- `model.operator`
- `model.project`
- `model.archive`

Projection must:
- stay bounded to the four typed slots above
- clip summaries
- remove stale reserved keys when artifacts deactivate or are removed

### Operator controls

Expose:
- list/report
- promote
- correct summary
- deactivate
- remove

through CLI plus MCP handlers.

## Risks and mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Artifact projection silently widens prompt context | Breaks trust boundary | Keep projection materialized into reserved core-memory keys only |
| Promotion without evidence creates speculation | Lowers memory quality | Require non-empty lineage and policy approval for promotion |
| Correction/removal bypasses projection cleanup | Stale prompt context | Re-sync reserved core-memory keys after every artifact mutation |
| Namespace semantics drift from user_id semantics | Projection goes to wrong prompt | Phase 182 assumes namespace-scoped artifacts project into the same namespace/user core-memory lane; cross-workspace stitching stays out of scope |

## Recommended plan split

- `182-01`: durable typed artifact storage, promotion policy, and bounded core-memory projection
- `182-02`: operator-facing inspection and mutation surfaces through CLI and MCP

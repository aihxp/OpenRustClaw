# Phase 35: Operator, Deployment, and Planning Docs Sync - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Phase 35 rewrites the operator-facing and planning-facing docs so they match the shipped runtime, control surfaces, enterprise boundary, and current product story. The goal is to replace stale or sprawling operational prose with a compact but truthful operator handbook.

</domain>

<decisions>
## Implementation Decisions

### Keep operator docs compact and truthful
- Replace very long production, observability, and security guides with concise operator-first documents.
- Keep links to deeper canonical planning docs instead of embedding every implementation detail inline.

### Preserve the real governance and autonomy boundaries
- Production and security docs must mention bearer token auth, trusted proxy mode, enterprise operator headers, governance rules, audit export, and operator-gated full autonomy.
- Do not flatten enterprise/autonomy into a generic “enterprise ready” claim.

### Planning docs should support the product story, not fight it
- Reframe the canonical roadmap and positioning pages around the shipped self-hosted product baseline.
- Keep feature and surface matrices canonical, but tighten their introductory framing.

### the agent's Discretion
The exact level of detail can be reduced aggressively as long as the resulting docs remain accurate and point readers to the right canonical references.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `docs/roadmap.md`, `docs/product-positioning.md`, `docs/feature-matrix.md`, and `docs/surface-matrix.md` are already the canonical planning docs.
- Phase 32 already established the documentation contract that explains canonical ownership.

### Established Patterns
- mdBook deployment, operations, and guide pages are best used as concise operator guides.
- Root planning docs can carry the denser planning detail.

### Integration Points
- `docs/src/deployment/production.md`
- `docs/src/operations/observability.md`
- `docs/src/guides/security.md`
- `docs/roadmap.md`
- `docs/product-positioning.md`
- `docs/feature-matrix.md`
- `docs/surface-matrix.md`

</code_context>

<specifics>
## Specific Ideas

- turn production into a practical operator runbook
- turn security into a control-boundary guide
- turn observability into a short “what to inspect and why” page
- tighten planning docs intros so they read like one product set

</specifics>

<deferred>
## Deferred Ideas

- deeper per-channel operational playbooks
- external compliance documentation beyond the current enterprise baseline

</deferred>

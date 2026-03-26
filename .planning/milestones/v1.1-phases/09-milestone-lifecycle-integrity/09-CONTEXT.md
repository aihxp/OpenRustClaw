# Phase 9: Milestone Lifecycle Integrity - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 9 hardens the milestone-level lifecycle that sits on top of Phase 8. The immediate goal is not another per-phase verification gate; it is making milestone audit, completion, and later cleanup preserve the verification evidence that audit decisions depend on, so shipped milestone history remains reviewable without reconstructing state from scattered phase directories.

</domain>

<decisions>
## Implementation Decisions

### Preserve evidence at milestone boundaries
- **D-01:** Milestone completion should always archive a milestone-level verification snapshot, even when phase directories are not archived immediately.
- **D-02:** Archived milestone evidence should capture both passing verification state and any accepted verification debt so later milestone review does not depend on ad-hoc forensics.
- **D-03:** Cleanup may still move whole phase directories, but the milestone archive must remain intelligible even if those directories are absent or relocated later.

### Workflow alignment
- **D-04:** Audit, complete-milestone, and cleanup guidance should all talk about the same milestone-level verification archive instead of each describing its own archive assumptions.
- **D-05:** The new archive artifact should be simple Markdown first: easy to inspect in git, easy to reference from MILESTONES.md, and easy for later workflows to load.

### the agent's Discretion
- A small amount of lifecycle plumbing in `milestone.cjs` is preferable to a larger new command surface if the archive artifact can be generated cleanly during milestone completion.
- It is acceptable for Phase 9 to improve workflow guidance rather than building a full new audit orchestrator, as long as the shipped archive contract becomes truthful and reusable.

</decisions>

<canonical_refs>
## Canonical References

### Current lifecycle implementation
- `.codex/get-shit-done/bin/lib/milestone.cjs` — current milestone completion/archive behavior
- `.codex/get-shit-done/bin/lib/init.cjs` — milestone bootstrap/archive awareness
- `.codex/get-shit-done/bin/lib/verification-artifacts.cjs` — shared verification-readiness inspection from Phase 8

### Lifecycle workflows
- `.codex/get-shit-done/workflows/audit-milestone.md` — audit expectations around phase verification and requirement coverage
- `.codex/get-shit-done/workflows/complete-milestone.md` — milestone archive workflow and output expectations
- `.codex/get-shit-done/workflows/cleanup.md` — retroactive phase-directory archival flow

### Existing project evidence
- `.planning/milestones/v1.0-MILESTONE-AUDIT.md` — explicit record of missing verification artifacts as accepted audit debt
- `.planning/MILESTONES.md` — operator-facing milestone index
- `.planning/milestones/` — current archive layout used in the repo

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `cmdMilestoneComplete()` already archives roadmap, requirements, and audit files and updates `MILESTONES.md`.
- Phase 8 introduced reusable verification inspection logic that can summarize verification state consistently before archival.
- Archived phase directories already preserve raw verification files when they are moved, so the missing piece is a milestone-level summary that survives even when directories are not archived immediately.

### Gaps to Close
- Milestone completion currently archives roadmap and requirements, but not a verification summary or accepted debt ledger.
- Audit workflow docs talk about missing verification as a blocker, but the shipped archive format does not preserve that evidence in one durable place.
- Cleanup guidance preserves phase directories by moving them, but it does not explicitly anchor later review on a milestone-level verification archive.

</code_context>

<specifics>
## Specific Ideas

- Generate `.planning/milestones/vX.Y-VERIFICATIONS.md` during milestone completion with one row per phase, verification status, score, artifact path, and debt notes.
- Record accepted verification debt in that archive file using the same categories Phase 8 introduced (`missing_verification`, `stale_verification`, `verification_gaps`).
- Update lifecycle docs so audit produces debt explicitly, complete-milestone archives it, and cleanup points reviewers at the archived verification snapshot.

</specifics>

<deferred>
## Deferred Ideas

- A full standalone milestone-audit CLI command can wait if the archive artifact and workflow contract become truthful now.
- Retrofitting v1.0 with reconstructed per-phase verification files remains out of scope.
- Enterprise approval and audit-policy surfaces still belong to Phase 10.

</deferred>

---
*Phase: 09-milestone-lifecycle-integrity*
*Context gathered: 2026-03-26*

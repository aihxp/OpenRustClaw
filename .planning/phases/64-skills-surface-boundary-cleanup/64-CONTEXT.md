# Phase 64: Skills Surface Boundary Cleanup - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Create the first bounded service seam for `skills.rs` so future work stops treating that hotspot as the default home for new behavior.

## What We Know

- `crates/cli/src/commands/skills.rs` is still one of the largest legacy hotspots in the repo at more than 5,000 lines.
- A meaningful bounded slice already exists inside it: the read-only compiled-skill overview lane that loads compiled manifests and artifacts, derives executable components, and reads compiled references.
- That compiled-skill overview logic is also duplicated or re-derived in `start.rs` for MCP registration, which makes it a better seam candidate than a single CLI-only helper.
- Mutation-heavy skill install, auth-plugin, and voice-plugin paths are real but broader; they should remain explicitly deferred for now.

## Constraints

- This phase must create one stable seam, not attempt a broad `skills.rs` rewrite.
- The extracted slice should preserve the current compiled-skill CLI and MCP behavior.
- The remaining `skills.rs` cleanup queue must be preserved honestly in planning and contributor guidance.

## Implementation Direction

- add a compiled-skill overview service to `openrustclaw-app`
- move compiled manifest, compiled artifact, executable-component, and compiled reference reading helpers behind that service
- reduce `skills.rs` and `start.rs` to adapters over that shared read-only seam

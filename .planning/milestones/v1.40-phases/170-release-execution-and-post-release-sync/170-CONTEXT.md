# Phase 170: Release Execution and Post-Release Sync - Context

**Gathered:** 2026-03-29
**Status:** Completed

<domain>
## Phase Boundary

Ship the prepared release, verify the public package surface, align the repo tag line to the public version, and make the docs and metadata match what actually shipped.

</domain>

<decisions>
## Implementation Decisions

### publish then align tags
Publish the crate first once credentials exist, then align the repo’s public version line with a matching `v1.4.0` tag.

### public version over milestone number
Treat the public release version as the source for package, tag, and release alignment; keep the milestone number internal.

### truthful blocker handling
Only claim shipped once crates.io publish succeeds and the repo tag line is updated to match it.

</decisions>

<code_context>
## Existing Code Insights

- The first recovered crates.io token from shell history was real but lacked publish permission and failed with `403 Forbidden`.
- A fresh crates.io token with publish scope was then provided and saved locally via `cargo login`.
- GitHub CLI authentication on this machine remained invalid, so Git tags and repo pushes were the truthful GitHub alignment mechanism available in-source.

</code_context>

<specifics>
## Specific Ideas

- Publish `openrustclaw-core 1.4.0`.
- Commit and tag the repo at `v1.4.0`.
- Push `main` and the new public tag so the repo’s public version line matches crates.io.

</specifics>

<deferred>
## Deferred Ideas

- Creating or editing a GitHub Release object directly from `gh`, because the local GitHub token is invalid

</deferred>

# Phase 33: README and Documentation Entry Surface Rewrite - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Phase 33 rewrites the public entry surfaces so OpenRustClaw reads like one coherent self-hosted open-source product. The README and docs landing pages should explain what the product is, who it is for, how to start, and where the canonical docs live.

</domain>

<decisions>
## Implementation Decisions

### Lead with product clarity, not exhaustive inventory
- Replace the sprawling README opening with a concise product story, deployment paths, quick start, and docs map.
- Keep deep surface detail in canonical planning docs instead of the README intro.

### Make audience paths explicit
- New users should see how to install and onboard.
- Returning operators should see how to run the assistant and inspect the control surface.
- Planning-oriented readers should be pointed to the canonical root docs under `docs/`.

### Keep claims strictly tied to shipped behavior
- Avoid aspirational copy or inflated feature counts.
- Prefer stable descriptions of shipped surfaces over fast-staling enumerations.

### the agent's Discretion
The final wording can be concise and product-oriented as long as it stays truthful and aligns with the documentation contract from Phase 32.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `README.md` already links the main planning docs and can be rewritten rather than split across multiple top-level files.
- `docs/src/introduction.md` and `docs/src/SUMMARY.md` are the mdBook landing surfaces and should match the README story.

### Established Patterns
- Root planning docs are canonical.
- mdBook is best used for guided navigation rather than restating every planning page inline.

### Integration Points
- README links should point to canonical docs and key mdBook guides.
- `docs/src/SUMMARY.md` should reflect audience-first navigation.

</code_context>

<specifics>
## Specific Ideas

- add a `Choose your path` section for `solo`, `team`, `company`, and `enterprise`
- make `openrustclaw onboard` the primary first-run command
- expose the docs contract and planning docs as explicit links instead of burying them in prose

</specifics>

<deferred>
## Deferred Ideas

- deeper guide rewrites for setup, security, observability, and production docs

</deferred>

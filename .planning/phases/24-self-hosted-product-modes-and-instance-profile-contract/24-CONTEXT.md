# Phase 24: Self-Hosted Product Modes and Instance Profile Contract - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

OpenRustClaw already has strong runtime, enterprise, and autonomy surfaces, but it still reads like one broad platform instead of one self-hosted open-source product with clear deployment paths. This phase should define the durable product-mode contract behind that story so later onboarding and lifecycle transitions do not rely on ad hoc strings in docs or UI copy.

This phase should:

- make solo, multi-user team, company, and enterprise modes first-class product concepts
- persist the selected mode as durable workspace-owned state under the existing control root
- expose a typed summary that the runtime and Control UI can reuse instead of inventing another frontend-only model
- stop at the contract and operator-facing inspection surface, leaving onboarding branching and transition actions to later phases

</domain>

<decisions>
## Implementation Decisions

### Reuse the existing `.claw/control` ownership model
The product-mode contract should live beside the current control, enterprise, and runtime manifests instead of inventing a new storage root.

### Keep product mode separate from runtime execution mode
`solo_claw`, `task_assigned`, and `orchestrated` describe execution topology, not commercial or operator packaging. Product mode should be an adjacent contract that can recommend runtime defaults without replacing them.

### Treat self-hosted and open-source identity as explicit data, not marketing-only text
The manifest and summary should make that framing inspectable from shipped surfaces so docs, onboarding, and Control UI can stay aligned.

### Ship a read path before the write-heavy lifecycle path
This phase should establish the manifest and summary contract first. Upgrade or downgrade actions and event history can build on the same contract in a later phase.

</decisions>

<code_context>
## Existing Code Insights

- `crates/cli/src/commands/control.rs` already owns the `.claw/control` registry and is the natural home for neighboring product-state files.
- `crates/cli/src/commands/onboard.rs` already captures onboarding profile and runtime-mode choices, so a product-mode contract can feed it later without changing the whole wizard shape at once.
- `crates/cli/src/commands/inspect.rs` and `crates/cli/src/commands/start.rs` already publish typed runtime summaries and `/control/...` routes for enterprise and operator surfaces.
- `crates/cli/src/commands/control_ui.html` already consumes typed JSON reports and renders summary cards without needing a second UI-specific data layer.

</code_context>

<specifics>
## Specific Ideas

- add a file-backed self-hosted product-mode manifest under `.claw/control/`
- model four explicit deployment modes: solo, team, company, enterprise
- include per-mode guidance such as onboarding path label, operator footprint, runtime recommendation, and transition targets
- expose a `/control/self-hosted/product-mode` summary route and Control UI summary card
- add targeted tests so later onboarding and lifecycle work can trust the contract

</specifics>

<deferred>
## Deferred Ideas

- interactive upgrade or downgrade actions
- onboarding path branching and existing-workspace transition handling
- hosted SaaS packaging, billing, or licensing concepts

</deferred>

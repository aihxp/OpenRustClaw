# Product Positioning

OpenRustClaw is a self-hosted open-source assistant product, not a hosted SaaS wrapper and not just a loose codebase of experiments. Its product claim is a Rust-first runtime with durable operator surfaces, explicit setup and control boundaries, and a growing but truthfully described enterprise and autonomy lane.

## What It Ships Well

- self-hosted onboarding and setup with durable handoff
- persisted assistant continuity and policy-aware memory
- truthful direct-provider versus delegated-agent lane selection with bounded local-agent execution
- shared CLI, HTTP, MCP, and Control UI operator surfaces
- delegated routing visibility through trusted remote backends, route receipts, and a dedicated routing console
- bounded tools, browser, communications, voice, and mobile inspection paths
- enterprise access, policy, governance, audit export, and operator-gated autonomy controls

## Where It Is Intentionally Stronger

- Rust-first production ownership instead of a compatibility-first runtime core
- explicit operator evidence and inspection surfaces instead of hidden behavior
- stronger control over high-risk autonomy changes
- clearer delegated-agent truthfulness than products that blur installed CLIs, direct APIs, and hidden OAuth reuse
- tighter docs-to-runtime truthfulness as part of the shipped product contract

## Current Boundaries

OpenRustClaw is intentionally bounded in a few places:

- it is self-hosted, not a hosted assistant platform
- the sidecar is optional compatibility tooling, not the default runtime brain
- full autonomy is gated and reviewed, not a silent default
- enterprise foundations are real, but they do not yet equal complete IAM or compliance packaging
- the public crates.io lane is intentionally narrow, and planning milestone tags are not the same thing as the public semver package line

## What Counts as a Valid Claim

A claim belongs in the product story only when:

- the runtime supports it end to end
- an operator can inspect or control it through shipped surfaces
- the docs describe it truthfully

Everything else belongs in future milestone planning, not in the current product pitch.

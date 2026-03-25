# OpenRustClaw Roadmap

This roadmap tracks the shipped OpenRustClaw product surface, the production-runtime contract,
and the release-exit gates that now govern the repository.

This is no longer a migration roadmap. It is the maintained release record for the current
OpenRustClaw surface.

## Current Status

- [x] Phase 1: Product contract and surface inventory
- [x] Phase 2: Rust runtime contract and sidecar retirement
- [x] Phase 2.5: Rust-native autonomous optimization framework
- [x] Phase 3: Durable scheduler and eventing
- [x] Phase 4: Memory, sessions, context, and RAG
- [x] Phase 5: Channels and routing
- [x] Phase 6: Tools, MCP, CLI, and Control surfaces
- [x] Phase 7: Skills, media, voice, and nodes
- [x] Phase 8: Security, operations, observability, and release exit

## North Star

OpenRustClaw is considered complete for the declared shipped surface when all of the following are
true:

- every supported operator workflow runs through Rust-owned production paths
- Python remains optional compatibility tooling rather than required production runtime
- docs, CLI help, control APIs, Control UI, tests, and runtime behavior agree on what is shipped
- remaining gaps are deliberate product boundaries, not accidental omissions
- the release, observability, security, and operations surfaces are production-ready

## Planning Rules

- `[x]` means complete enough to rely on.
- `[ ]` means still open.
- no feature counts as done unless startup, runtime behavior, persistence, tests, and docs exist
- no product path may return fake success
- no release claim lands unless docs and operator surfaces match runtime truth

## Runtime Model

OpenRustClaw uses a three-tier execution model:

- Tier A: Rust-native production paths
- Tier B: bounded sidecar compatibility for migration-only workflows
- Tier C: LangGraph experimentation and authoring

Rust owns the production durability boundary:

- scheduler and event loop
- retries, leases, checkpoints, and resumability
- persistence and state inspection
- operator-facing control, diagnostics, and health surfaces

## Shipped Surface

The declared shipped surface is maintained in:

- [feature-matrix.md](feature-matrix.md)
- [surface-matrix.md](surface-matrix.md)
- [product-positioning.md](product-positioning.md)
- [docs-audit.md](docs-audit.md)

Together these documents define:

- what ships today
- what is intentionally stronger or more opinionated
- what remains intentionally bounded
- which tests and runtime paths back each major feature family

## Phase Summary

### Phase 1: Product contract and surface inventory

Completed:

- shipped-surface matrix and supporting planning pages
- explicit scope, owner, tests, and documentation mapping
- consistent release labeling for shipped-surface status

Exit result:

- OpenRustClaw has a stable, source-backed surface inventory

### Phase 2: Rust runtime contract and sidecar retirement

Completed:

- Rust-owned workflow registry and execution-tier policy
- Rust-owned scheduler, memory, and retrieval control paths
- Python removed from the default production-critical runtime path

Exit result:

- the default production runtime no longer depends on Python ownership

### Phase 2.5: Rust-native autonomous optimization framework

Completed:

- target registry, candidate store, evaluation history, and promotion flow
- bounded workflow and code-improvement lanes
- operator controls and inspection surfaces

Exit result:

- autonomous optimization exists as a Rust-owned, inspectable subsystem

### Phase 3: Durable scheduler and eventing

Completed:

- durable scheduler loop with retries, dead-letter handling, and events
- file-backed task manifests and registry
- operator-facing job and event inspection

Exit result:

- scheduled and event-triggered work is durable and inspectable

### Phase 4: Memory, sessions, context, and RAG

Completed:

- durable sessions and session-policy controls
- core, recall, and archive memory
- Rust-owned retrieval, budgeting, and context assembly
- migration, import/export, and maintenance flows

Exit result:

- operators can manage memory and sessions end to end from shipped surfaces

### Phase 5: Channels and routing

Completed:

- tier-1 channel runtime coverage across the declared shipped channel set
- channel account and binding registry
- direct, group, mention, and thread-aware routing rules
- media send and receive flows for the shipped channels

Exit result:

- the supported channel set is usable end to end through Rust-owned paths

### Phase 6: Tools, MCP, CLI, and Control surfaces

Completed:

- shared typed control/config/diagnostics APIs
- Web Control UI for the declared operator surface
- browser/tool-group lane
- onboarding, doctor, runtime config, and vault workflows
- orchestration, supervision, and runtime inspection surfaces

Exit result:

- operators can inspect and steer the shipped runtime through consistent CLI, MCP, HTTP, and UI

### Phase 7: Skills, media, voice, and nodes

Completed:

- compiled skill pipeline, manifests, sandboxed execution, and background services
- bounded auth and voice-call plugin lanes
- media inspection, extraction, and provider-backed description flows
- voice-note transcription, talk runtime inspection, and talk-mode runner
- mobile node pairing, command execution, receipts, metrics, and activity views

Exit result:

- OpenRustClaw ships a coherent Rust-owned extension, media, voice, and node surface

### Phase 8: Security, operations, observability, and release exit

Completed:

- Prometheus metrics endpoint and runtime metrics coverage
- OTLP trace export
- control-plane bearer-token and trusted-proxy auth gates
- control-plane origin allowlist checks
- runtime backup/restore, service install, log rotation, runtime locks, config migration,
  update planning, and rollback planning
- release artifact packaging and runtime budget checks
- channel health monitor restart policy
- scenario fixtures, docs audit, and release-exit validation

Exit result:

- release, security, observability, and operations gates are complete for the shipped surface

## Release Exit Gates

- [x] feature matrix is current
- [x] surface matrix is current
- [x] product-positioning page is current
- [x] docs audit is current
- [x] runtime, CLI help, and control surfaces match the documented shipped contract
- [x] shipped integration scenarios are covered by automated tests
- [x] release artifact packaging exists
- [x] runtime budget checks exist
- [x] production runtime is Rust-owned by default
- [x] security and control-plane gates are in place
- [x] observability coverage exists for operator-significant paths

## Ongoing Follow-Up Work

The roadmap is complete for the declared shipped surface. Future work should be tracked as new
product expansion or hardening initiatives, not by reviving old comparison framing.

Examples of valid future follow-up work:

- new channel families
- broader remote browser or MCP transport options
- richer operator polish in Control UI or onboarding
- deeper provider-specific UX
- additional release automation and hardening

## Release Statement

OpenRustClaw can now be described truthfully as:

- a Rust-owned operator runtime with durable scheduling, memory, routing, and control surfaces
- a shipped multi-channel product with bounded browser, skill, media, voice, and mobile lanes
- a product whose docs, runtime behavior, and release gates are aligned around the same surface

# OpenRustClaw

## What This Is

OpenRustClaw is a Rust-first version of OpenClaw: a general-purpose AI assistant platform intended to become clean, production-ready, and eventually enterprise-ready. It already contains a broad operator and runtime surface across chat, memory, tools, coding workflows, channels, voice, browser automation, and deployment, and the immediate goal is to turn that breadth into a trustworthy MVP rather than keep expanding unfinished surface area.

The MVP is for solo developers and small operator teams first. It should feel like a real OpenClaw-grade assistant platform that can be installed, configured, trusted, and used end-to-end for assistant chat, coding work, communications, and runtime operations without obvious rough edges.

## Core Value

Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## Requirements

### Validated

- ✓ Rust-first assistant runtime and CLI/operator surface exist — existing
- ✓ Multi-provider model integration and fallback infrastructure exist — existing
- ✓ Memory, scheduler, skills, channel, browser, voice, and control-plane primitives exist — existing
- ✓ Runtime/deployment/operator management surfaces already exist in code and docs — existing
- ✓ Optional Python sidecar compatibility lane exists for bounded workflows — existing

### Active

- [ ] Make onboarding and first-run setup production-ready
- [ ] Make the core assistant/chat experience reliable and coherent across persisted sessions
- [ ] Make memory durable, inspectable, and policy-driven enough for production use
- [ ] Make tool, MCP, and coding workflows dependable enough for day-to-day assistant work
- [ ] Make email and phone/voice communication lanes shippable as part of MVP
- [ ] Make runtime, deployment, recovery, and operator diagnostics production-ready
- [ ] Harden security, observability, and release readiness so the MVP is trustworthy in the real world

### Out of Scope

- Full unsupervised AGI that can autonomously run an entire business end-to-end — beyond MVP trust and safety boundary
- Enterprise multi-tenancy, compliance packaging, deep RBAC/SSO governance, and procurement-driven controls — targeted after MVP is stable
- Perfect parity across every OpenClaw surface, every channel, and every experimental lane in the first production release — stabilize core lanes first
- Bespoke vertical automations such as flight booking and full business back-office orchestration — build on top of a stable core platform later

## Context

This is a brownfield Rust monorepo with a very large existing surface area and strong documentation already in place. The codebase includes a Rust-first runtime, a broad CLI/operator surface, runtime control APIs, memory, tools, MCP, browser automation, channels, voice, mobile, and deployment assets, plus an optional Python sidecar compatibility path.

The project is not trying to discover what to build from scratch. It is trying to take an already ambitious and partially implemented OpenClaw-like platform and make it feel coherent, reliable, and production-worthy. The immediate job is to reduce rough edges, tighten contracts, close end-to-end gaps, and define a believable MVP boundary that reflects what OpenClaw should feel like when it is clean and ready to ship.

Long term, the platform should be able to support a wide range of assistant tasks: coding, communications, scheduling, tool use, business operations, and other agentic workflows. The near-term milestone is narrower: make the existing breadth dependable enough that the product feels real.

## Constraints

- **Tech stack**: Rust-first runtime must remain the primary production path; the Python sidecar remains a bounded compatibility lane, not the main system
- **Brownfield scope**: Prefer hardening and converging existing surfaces over adding new feature families unless a missing contract blocks MVP credibility
- **Reliability**: MVP workflows must be testable, diagnosable, restartable, and inspectable by operators
- **Security**: Auth, secret handling, sandboxing, and origin/runtime trust boundaries must stay on by default for production paths
- **Product focus**: Production-ready MVP comes before enterprise-ready expansion

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Treat OpenRustClaw as a Rust-first OpenClaw implementation | The repo already contains broad Rust runtime investment and should converge around that identity | — Pending |
| Define MVP as production readiness for core assistant workflows, not greenfield feature expansion | The existing codebase already covers a large surface and now needs coherence and trust | — Pending |
| Include onboarding, assistant/chat, memory, tools/MCP, coding workflow, email/phone, and runtime ops in the MVP boundary | These are the non-negotiable workflows the user named as required for a believable OpenClaw-like MVP | — Pending |
| Defer enterprise-ready concerns until after MVP stabilization | Enterprise packaging on top of an unstable MVP would create the wrong priorities | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `$gsd-transition`):
1. Requirements invalidated? -> Move to Out of Scope with reason
2. Requirements validated? -> Move to Validated with phase reference
3. New requirements emerged? -> Add to Active
4. Decisions to log? -> Add to Key Decisions
5. "What This Is" still accurate? -> Update if drifted

**After each milestone** (via `$gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check -> still the right priority?
3. Audit Out of Scope -> reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-03-26 after initialization*

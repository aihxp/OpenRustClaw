# OpenRustClaw

## What This Is

OpenRustClaw is a Rust-first OpenClaw-style assistant platform with a shipped v1.0 MVP. It now has a coherent production-ready baseline across onboarding, assistant continuity, memory policy, tool and coding evidence, communications, runtime operations, security posture, and release exit.

The product target remains broader than the MVP: a general-purpose assistant platform that can eventually support coding, communications, business operations, and deeper autonomous workflows. The difference after v1.0 is that the repo now has a believable trust baseline instead of a wide but loosely connected surface.

## Core Value

Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## Current State

- **Shipped milestone:** v1.0 Rust OpenClaw MVP on 2026-03-26
- **Archive:** `.planning/milestones/v1.0-ROADMAP.md`, `.planning/milestones/v1.0-REQUIREMENTS.md`, `.planning/milestones/v1.0-MILESTONE-AUDIT.md`
- **Planning state:** No active milestone is currently open
- **Known audit debt:** v1.0 shipped with a passing release gate, but the archived milestone audit records missing per-phase `VERIFICATION.md` artifacts

## Requirements

### Validated

- ✓ Operator can install OpenRustClaw and reach a validated onboarding path — v1.0
- ✓ Assistant chat and persisted session continuity are coherent across CLI and Control UI surfaces — v1.0
- ✓ Memory writes are policy-gated, durable, and inspectable — v1.0
- ✓ Tools, MCP, and coding workflows are bounded and auditable — v1.0
- ✓ Email and voice communications have operator-visible evidence and trust surfaces — v1.0
- ✓ Runtime deployment, recovery, and operator ops are documented and inspectable — v1.0
- ✓ Security posture and MVP release exit are exposed through shipped docs, control surfaces, and an automated release gate — v1.0

### Active

- [ ] Add a stronger milestone lifecycle contract so future phases always preserve `VERIFICATION.md` evidence automatically
- [ ] Define the next post-MVP milestone around enterprise readiness, broader autonomy controls, or deeper surface parity
- [ ] Decide which v2 lane matters first: enterprise governance, longer-running supervised automation, or broader OpenClaw parity

### Out of Scope

- Full unsupervised AGI that can autonomously run an entire business end-to-end — beyond the current trust and safety boundary
- Enterprise multi-tenancy, compliance packaging, deep RBAC/SSO governance, and procurement-driven controls — deferred until post-MVP milestone planning
- Perfect parity across every OpenClaw surface, every channel, and every experimental lane in the first production release — stabilize core lanes first
- Bespoke vertical automations such as flight booking and full business back-office orchestration — build on top of a stable core platform later

## Context

This remains a large brownfield Rust monorepo with broad runtime, CLI, control-plane, memory, tools, channel, voice, browser, and deployment surfaces. v1.0 converted that breadth into a cleaner MVP by making operator trust visible at the edges that matter: first run, session continuity, memory policy, execution evidence, communications evidence, runtime health, security posture, and release exit.

The next milestone should build on that baseline rather than reopen MVP-sprawl. The immediate planning question is not "what can the platform eventually do?" but "which post-MVP trust or expansion lane should be hardened next?"

## Constraints

- **Rust-first runtime:** The Rust runtime remains the primary production path
- **Trust-first expansion:** New milestones should preserve the inspectability and verification gains established in v1.0
- **Brownfield pragmatism:** Prefer converging existing surfaces over adding whole new product families unless a missing contract blocks the next milestone
- **Security baseline:** Production auth, secret handling, sandboxing, and origin/runtime trust boundaries stay on by default

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Treat OpenRustClaw as a Rust-first OpenClaw implementation | The repo already contained broad Rust runtime investment and needed a coherent identity | ✓ Good |
| Define MVP as production readiness for core assistant workflows, not greenfield feature expansion | Existing surface area needed trust and convergence more than new breadth | ✓ Good |
| Include onboarding, assistant/chat, memory, tools/MCP, coding workflow, email/phone, and runtime ops in the MVP boundary | These were the non-negotiable workflows required for a believable OpenClaw-grade MVP | ✓ Good |
| Defer enterprise-ready concerns until after MVP stabilization | Enterprise packaging on top of an unstable MVP would have created the wrong priorities | ✓ Good |
| Accept v1.0 milestone audit gaps around missing phase verification artifacts while preserving the gap explicitly in the archive | The shipped MVP passed its release gate, but lifecycle evidence was incomplete and needed to be recorded honestly | ⚠ Revisit |

## Next Milestone Goals

- Decide the post-v1.0 milestone boundary with `$gsd-new-milestone`
- Preserve verification artifacts automatically during future phase execution
- Choose one concrete expansion lane instead of reopening all deferred ambitions at once

---
*Last updated: 2026-03-26 after v1.0 milestone completion*

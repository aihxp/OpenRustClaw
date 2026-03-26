# OpenRustClaw

## What This Is

OpenRustClaw is a Rust-first OpenClaw-style assistant platform with a shipped v1.0 MVP, a completed v1.1 lifecycle and enterprise-foundations follow-up, and an active v1.2 parity push. It now has a coherent production-ready baseline across onboarding, assistant continuity, memory policy, tool and coding evidence, communications, runtime operations, security posture, release exit, milestone verification integrity, and a narrow enterprise approval/audit foundation.

The product target remains broader than the MVP: a general-purpose assistant platform that can eventually support coding, communications, business operations, and deeper autonomous workflows. The difference after v1.0 is that the repo now has a believable trust baseline instead of a wide but loosely connected surface.

## Core Value

Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## Current State

- **Shipped milestones:** v1.0 Rust OpenClaw MVP and v1.1 Lifecycle Integrity and Enterprise Foundations on 2026-03-26
- **Archive:** `.planning/milestones/v1.0-*`, `.planning/milestones/v1.1-*`
- **Planning state:** v1.2 Deeper OpenClaw Surface Parity is active
- **Known audit debt:** v1.0 archive still records missing phase `VERIFICATION.md` artifacts; v1.1 closes that workflow gap going forward

## Current Milestone: v1.2 Deeper OpenClaw Surface Parity

**Goal:** Deepen the highest-leverage OpenClaw parity surfaces without weakening the v1.1 trust and verification baseline.

**Target features:**
- richer browser automation depth and artifacts
- stronger multi-agent supervision and orchestration parity
- broader mobile runtime and operator parity
- deeper Control UI parity across shipped surfaces
- richer voice and call-handling parity

## Requirements

### Validated

- ✓ Operator can install OpenRustClaw and reach a validated onboarding path — v1.0
- ✓ Assistant chat and persisted session continuity are coherent across CLI and Control UI surfaces — v1.0
- ✓ Memory writes are policy-gated, durable, and inspectable — v1.0
- ✓ Tools, MCP, and coding workflows are bounded and auditable — v1.0
- ✓ Email and voice communications have operator-visible evidence and trust surfaces — v1.0
- ✓ Runtime deployment, recovery, and operator ops are documented and inspectable — v1.0
- ✓ Security posture and MVP release exit are exposed through shipped docs, control surfaces, and an automated release gate — v1.0
- ✓ Completed phases now require preserved `VERIFICATION.md` artifacts and milestone archives preserve that evidence truthfully — v1.1
- ✓ Approval-sensitive assistant actions now expose an enterprise foundations baseline with explicit policy and durable audit evidence — v1.1

### Active

- [ ] Deepen browser automation parity without regressing the existing browser policy and audit contract
- [ ] Expand multi-agent supervision, mobile parity, Control UI depth, and voice/call handling under the same trust-first baseline
- [ ] Preserve the v1.1 verification and archive contract while expanding deeper OpenClaw parity

### Out of Scope

- Full unsupervised AGI that can autonomously run an entire business end-to-end — beyond the current trust and safety boundary
- Enterprise multi-tenancy, compliance packaging, deep RBAC/SSO governance, and procurement-driven controls — deferred until post-MVP milestone planning
- Perfect parity across every OpenClaw surface, every channel, and every experimental lane in the first production release — stabilize core lanes first
- Bespoke vertical automations such as flight booking and full business back-office orchestration — build on top of a stable core platform later

## Context

This remains a large brownfield Rust monorepo with broad runtime, CLI, control-plane, memory, tools, channel, voice, browser, and deployment surfaces. v1.0 converted that breadth into a cleaner MVP by making operator trust visible at the edges that matter: first run, session continuity, memory policy, execution evidence, communications evidence, runtime health, security posture, and release exit.

The next milestone should build on that baseline rather than reopen MVP-sprawl. The current planning question is no longer which lane to choose; it is how to deepen the highest-value OpenClaw parity surfaces while keeping the lifecycle and trust gains from v1.1 intact.

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
| Archive milestone verification evidence explicitly during milestone completion | Later review should not depend on live phase directories or manual reconstruction | ✓ Good |
| Define the first enterprise slice around explicit approval boundaries plus durable audit evidence | The repo needed a truthful foundation before larger governance work like RBAC or compliance packaging | ✓ Good |
| Prioritize deeper OpenClaw parity through a focused top-five surface slice | Browser depth, supervision, mobile, Control UI, and voice/calls are the clearest next parity gains without scattering effort | ✓ Good |

## Next Milestone Goals

- Deepen the top five OpenClaw parity surfaces under one milestone
- Keep the verification/archive contract from v1.1 as a non-negotiable baseline
- Defer broader enterprise governance and business-ops autonomy until parity work proves the current surfaces are coherent

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `$gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `$gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-03-26 after starting milestone v1.2*

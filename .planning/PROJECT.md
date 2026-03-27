# OpenRustClaw

## What This Is

OpenRustClaw is a self-hosted open-source Rust-first OpenClaw-style assistant platform with shipped v1.0 through v1.5 milestones. It now has a coherent production-ready baseline across onboarding, assistant continuity, memory policy, tool and coding evidence, communications, runtime operations, security posture, release exit, milestone verification integrity, materially deeper operator parity across browser, orchestration, mobile, Control UI, and voice or call handling surfaces, an enterprise-first foundation for scoped operator identity, policy, audit export, supervised autonomy, governance, and an operator-gated full-autonomy lane, plus an explicit self-hosted product-mode contract across `solo`, `team`, `company`, and `enterprise` deployments.

The product target remains broader than the MVP: a general-purpose assistant platform that can eventually support coding, communications, business operations, and deeper autonomous workflows. The difference after v1.0 is that the repo now has a believable trust baseline instead of a wide but loosely connected surface.

## Core Value

Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## Current State

- **Shipped milestones:** v1.0 Rust OpenClaw MVP and v1.1 Lifecycle Integrity and Enterprise Foundations on 2026-03-26, plus v1.2 Deeper OpenClaw Surface Parity, v1.3 Enterprise Expansion and Supervised Autonomy Foundations, v1.4 Enterprise Governance and Operator-Gated Full Autonomy, and v1.5 Self-Hosted Product Modes and Lifecycle Packaging on 2026-03-27
- **Archive:** `.planning/milestones/v1.0-*`, `.planning/milestones/v1.1-*`, `.planning/milestones/v1.2-*`, `.planning/milestones/v1.3-*`, `.planning/milestones/v1.4-*`, `.planning/milestones/v1.5-*`
- **Planning state:** v1.6 Proper Onboarding and Setup is now active
- **Known audit debt:** v1.0 archive still records missing phase `VERIFICATION.md` artifacts; v1.1 closes that workflow gap going forward

## Current Milestone: v1.6 Proper Onboarding and Setup

**Goal:** Turn first install into a truthful, end-to-end setup journey that gets a self-hosted workspace from blank state to ready assistant and runtime handoff with repairable failures and explicit operator checkpoints.

**Target features:**
- durable setup-state tracking for first install, partial setup, and resumed onboarding
- a clear standard setup path plus an advanced or custom path for operators who need more control
- mode-aware provider, channel, and runtime setup flows that end in a truthful readiness handoff
- repair and retry paths for failed or partially configured workspaces
- docs and shipped operator surfaces that make setup progress and next actions legible

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
- ✓ Browser, orchestration, mobile, Control UI, and voice or call parity now expose materially deeper typed operator surfaces without weakening the trust-first runtime contract — v1.2
- ✓ Enterprise access now uses a bootstrapped organization or operator registry plus scoped headers on selected sensitive control actions, with a matching control summary and dashboard surface — v1.3 Phase 16
- ✓ Enterprise policy and audit controls now expose a unified typed surface with durable exportable evidence — v1.3 Phase 17
- ✓ Supervised autonomy now uses explicit escalation, rollback, and intervention evidence rather than opaque active-run state — v1.3 Phase 18
- ✓ Enterprise operators can manage the current access, policy, audit, and supervision baseline from one shipped admin/operator surface — v1.3 Phase 19
- ✓ Enterprise governance now applies stronger approval-chain and separation-of-duties controls to higher-risk scopes — v1.4 Phase 20
- ✓ Enterprise audit retention and review now preserve richer governance, supervision, and operator evidence — v1.4 Phase 21
- ✓ Full autonomy is now a separate operator-gated lane with budgets, kill switch, and durable lifecycle evidence — v1.4 Phase 22
- ✓ Operators can now inspect and control the full-autonomy lane from the shipped dashboard — v1.4 Phase 23
- ✓ Self-hosted product modes are now a first-class control-plane contract with a shipped runtime and Control UI summary — v1.5 Phase 24
- ✓ Onboarding now offers explicit self-hosted deployment paths with mode-aware defaults and first-start diagnostics — v1.5 Phase 25
- ✓ Product-mode upgrades and downgrades are now durable, warning-aware, and available through the shipped control surface — v1.5 Phase 26
- ✓ Docs and shipped control surfaces now align around the self-hosted open-source product story, deployment paths, and transition visibility — v1.5 Phase 27
- ✓ Onboarding now persists durable setup state and supports standard, advanced, and custom resume-aware setup paths — v1.6 Phase 28

### Active

- [ ] Ensure setup surfaces can repair or resume partial and failed installs instead of forcing manual state edits
- [ ] Keep setup docs and shipped operator surfaces aligned with the actual setup contract
- [ ] Make provider, runtime, and channel bootstrap mode-aware and truthful before setup claims readiness

### Out of Scope

- Full unsupervised AGI that can autonomously run an entire business end-to-end — beyond the current trust and safety boundary
- Enterprise multi-tenancy, compliance packaging, deep RBAC/SSO governance, and procurement-driven controls — deferred until post-MVP milestone planning
- Perfect parity across every OpenClaw surface, every channel, and every experimental lane in the first production release — stabilize core lanes first
- Bespoke vertical automations such as flight booking and full business back-office orchestration — build on top of a stable core platform later

## Context

This remains a large brownfield Rust monorepo with broad runtime, CLI, control-plane, memory, tools, channel, voice, browser, and deployment surfaces. v1.0 converted that breadth into a cleaner MVP by making operator trust visible at the edges that matter, v1.1 hardened the lifecycle and enterprise baseline around that trust, v1.2 deepened the most operator-visible OpenClaw parity surfaces without reopening MVP sprawl, v1.3 turned the first enterprise and supervised-autonomy contracts into a coherent operator loop, v1.4 extended that loop into explicit governance and operator-gated full autonomy, and v1.5 made the platform legible as one self-hosted open-source product with explicit deployment paths and transition visibility.

The next milestone should build on all six shipped layers rather than reopen foundational debt. Enterprise expansion, broader supervised autonomy, and any deeper parity work should now treat the v1.4 governance and full-autonomy contract plus the v1.5 self-hosted product-mode baseline as fixed infrastructure. The immediate gap is that onboarding and setup still need to feel like one complete product journey rather than a cluster of adjacent setup commands.

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
| Keep deeper parity work grounded in typed runtime summaries instead of frontend-only stitching | Browser, supervision, mobile, Control UI, and voice parity all landed more cleanly when the Rust-owned control plane aggregated the operator story first | ✓ Good |
| Sequence enterprise expansion ahead of another broad parity milestone | Identity, policy, audit, and supervised-autonomy controls are now the clearest blockers to enterprise readiness | ✓ Good |
| Layer enterprise operator identity on top of the existing control auth boundary | The control bearer and trusted-proxy transport boundary already exists, so enterprise identity should narrow sensitive operator actions instead of replacing the outer control contract | ✓ Good |
| Keep the v1.3 enterprise admin surface inside the shipped Control UI and typed runtime reports | The current goal was operator usability for the existing enterprise/autonomy contract, not a separate admin product or frontend-only state layer | ✓ Good |
| Interpret “god mode” as an explicit operator-gated full-autonomy lane rather than a removal of audit or control boundaries | The platform’s trust-first contract still needs to hold even when trusted operators deliberately enable a stronger autonomy mode | ✓ Good |

## Next Milestone Goals

- Make first install, partial setup recovery, and post-setup handoff feel like one coherent Rust-owned onboarding path
- Make setup depth explicit with a standard default path plus advanced or custom flows for operators who need deeper control
- Preserve the verification/archive contract, enterprise governance baseline, shipped autonomy control surface, and self-hosted product-mode contract as non-negotiable foundations
- Leave larger enterprise expansion and autonomy breadth for a later milestone after setup convergence is credible

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
*Last updated: 2026-03-28 for milestone v1.6*

# OpenRustClaw

## What This Is

OpenRustClaw is a self-hosted open-source Rust-first OpenClaw-style assistant platform with shipped v1.0 through v1.7 milestones. It now has a coherent production-ready baseline across onboarding, assistant continuity, memory policy, tool and coding evidence, communications, runtime operations, security posture, release exit, milestone verification integrity, materially deeper operator parity across browser, orchestration, mobile, Control UI, and voice or call handling surfaces, an enterprise-first foundation for scoped operator identity, policy, audit export, supervised autonomy, governance, and an operator-gated full-autonomy lane, an explicit self-hosted product-mode contract across `solo`, `team`, `company`, and `enterprise` deployments, a truthful setup lifecycle from first install through repair and handoff, and a canonical documentation contract that keeps the repo entrypoint, guided docs, and planning docs aligned.

The product target remains broader than the MVP: a general-purpose assistant platform that can eventually support coding, communications, business operations, and deeper autonomous workflows. After v1.7, the repo also has one clearer documentation story instead of several drifting versions of the product.

## Core Value

Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## Current State

- **Shipped milestones:** v1.0 Rust OpenClaw MVP and v1.1 Lifecycle Integrity and Enterprise Foundations on 2026-03-26, plus v1.2 Deeper OpenClaw Surface Parity, v1.3 Enterprise Expansion and Supervised Autonomy Foundations, v1.4 Enterprise Governance and Operator-Gated Full Autonomy, v1.5 Self-Hosted Product Modes and Lifecycle Packaging, and v1.7 Documentation Convergence and OpenClaw-Inspired Docs Rewrite on 2026-03-27, and v1.6 Proper Onboarding and Setup on 2026-03-28
- **Archive:** `.planning/milestones/v1.0-*`, `.planning/milestones/v1.1-*`, `.planning/milestones/v1.2-*`, `.planning/milestones/v1.3-*`, `.planning/milestones/v1.4-*`, `.planning/milestones/v1.5-*`, `.planning/milestones/v1.6-*`, `.planning/milestones/v1.7-*`
- **Planning state:** v1.8 is open and ready for Phase 37 planning
- **Known audit debt:** v1.0 archive still records missing phase `VERIFICATION.md` artifacts; v1.1 closed that workflow gap going forward

## Most Recent Milestone: v1.7 Documentation Convergence and OpenClaw-Inspired Docs Rewrite

**Result:** Shipped 2026-03-27. OpenRustClaw now has a canonical documentation ownership contract, rewritten repo and mdBook entry surfaces, aligned getting-started and operator guides, and an explicit docs maintenance and audit loop.

**Archive:** `.planning/milestones/v1.7-ROADMAP.md`, `.planning/milestones/v1.7-REQUIREMENTS.md`, `.planning/milestones/v1.7-MILESTONE-AUDIT.md`, `.planning/milestones/v1.7-VERIFICATIONS.md`

## Current Milestone: v1.8 Clean Codebase

**Goal:** Reduce brownfield structural debt so future feature work lands in a smaller, clearer, and safer codebase.

**Target features:**
- explicit cleanup inventory for oversized, deprecated, generated, or drifting surfaces
- removal of stale repo paths and contract mismatches across docs, CI, and workspace artifacts
- bounded decomposition of priority oversized command and control surfaces without breaking shipped behavior
- targeted cleanup verification and a maintained follow-up debt record

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
- ✓ Onboarding now persists durable setup state and supports Standard, Advanced, and Custom resume-aware setup paths — v1.6 Phase 28
- ✓ Onboarding now validates provider, runtime, and channel bootstrap through shipped health and probe surfaces and records those outcomes in setup state — v1.6 Phase 29
- ✓ Existing workspaces can now re-enter setup through an explicit repair path derived from setup state and doctor diagnostics — v1.6 Phase 30
- ✓ Onboarding, Control UI, and setup docs now share one explicit setup handoff contract — v1.6 Phase 31
- ✓ OpenRustClaw now has a canonical docs ownership contract, rewritten entry surfaces, aligned getting-started and operator guides, and a standing docs audit and maintenance workflow — v1.7 Phases 32-36

### Active

- [ ] Reduce dead, duplicated, stale, local-only, or drifting repo surfaces and make cleanup ownership explicit
- [ ] Split the highest-risk oversized modules into clearer bounded units without regressing the shipped operator contract
- [ ] Align CI, docs, and verification around the cleaned codebase so future refactors stay safe

### Out of Scope

- Full unsupervised AGI that can autonomously run an entire business end-to-end — beyond the current trust and safety boundary
- Enterprise multi-tenancy, compliance packaging, deep RBAC/SSO governance, and procurement-driven controls — deferred until a future milestone explicitly scopes them
- Perfect parity across every OpenClaw surface, every channel, and every experimental lane — stabilize the core product and its documentation contract first
- A pure marketing-site rewrite disconnected from the shipped docs and runtime surface — the goal is truthful product documentation, not branding alone

## Context

This remains a large brownfield Rust monorepo with broad runtime, CLI, control-plane, memory, tools, channel, voice, browser, deployment, and operator surfaces. v1.0 converted that breadth into a cleaner MVP by making operator trust visible at the edges that matter, v1.1 hardened the lifecycle and enterprise baseline around that trust, v1.2 deepened the most operator-visible OpenClaw parity surfaces without reopening MVP sprawl, v1.3 turned the first enterprise and supervised-autonomy contracts into a coherent operator loop, v1.4 extended that loop into explicit governance and operator-gated full autonomy, v1.5 made the platform legible as one self-hosted open-source product with explicit deployment paths and transition visibility, v1.6 turned onboarding and setup into one truthful lifecycle from first install through repair and handoff, and v1.7 made the documentation set legible enough to match the shipped product baseline.

The next milestone should build on all of those shipped layers by shrinking brownfield complexity instead of widening the surface again immediately. Current concerns already called out oversized command hubs, sidecar or contract drift risk, stale CI or docs references, and local-environment noise inside the repo tree. v1.8 treats that cleanup as first-class product-enabling work rather than opportunistic background tidying.

## Constraints

- **Rust-first runtime:** The Rust runtime remains the primary production path
- **Truth over gloss:** Documentation changes must describe shipped behavior and supported paths, not aspirational marketing claims
- **Brownfield pragmatism:** Prefer converging or deleting existing docs over creating another layer of parallel files
- **Cleanup without regressions:** Internal refactors must preserve the shipped operator, runtime, and control-surface behavior
- **Canonical ownership:** Every major docs surface should have one primary owner and clearly documented mirrors or derived views
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
| Treat documentation convergence as product work rather than post-hoc cleanup | The repo now has enough shipped depth that drift between README, docs mirrors, and docs-site guidance directly reduces usability and trust | ✓ Good |
| Use OpenClaw’s public docs style as inspiration for clarity and entry-point design, not as a content-copying exercise | The goal is a clearer self-hosted product story, tighter onboarding path, and better docs information architecture grounded in OpenRustClaw’s actual shipped behavior | ✓ Good |
| Treat codebase cleanup as a milestone in its own right | The repo is now broad enough that oversized modules, stale paths, and contract drift directly slow down safe product work | — Pending |

## Current Milestone Goals

- Inventory and prioritize cleanup targets so later refactors stop happening by intuition alone
- Remove stale or drifting repo surfaces that no longer match the shipped product contract
- Decompose the highest-risk oversized command or control surfaces behind stable behavior contracts
- Lock cleanup in with targeted verification and an explicit remaining-debt record

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
*Last updated: 2026-03-27 after starting v1.8*

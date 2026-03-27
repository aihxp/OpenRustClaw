# OpenRustClaw

## What This Is

OpenRustClaw is a self-hosted open-source Rust-first OpenClaw-style assistant platform with shipped v1.0 through v1.6 milestones. It now has a coherent production-ready baseline across onboarding, assistant continuity, memory policy, tool and coding evidence, communications, runtime operations, security posture, release exit, milestone verification integrity, materially deeper operator parity across browser, orchestration, mobile, Control UI, and voice or call handling surfaces, an enterprise-first foundation for scoped operator identity, policy, audit export, supervised autonomy, governance, and an operator-gated full-autonomy lane, plus an explicit self-hosted product-mode contract across `solo`, `team`, `company`, and `enterprise` deployments and a truthful setup lifecycle from first install through repair and handoff.

The current milestone is not about adding another major capability family. It is about rewriting the documentation so the product now reads as one coherent self-hosted open-source product, with one canonical source-of-truth structure and one aligned README, docs site, and operator story inspired by the clarity of OpenClaw’s public-facing docs.

## Core Value

Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## Current State

- **Shipped milestones:** v1.0 Rust OpenClaw MVP and v1.1 Lifecycle Integrity and Enterprise Foundations on 2026-03-26, plus v1.2 Deeper OpenClaw Surface Parity, v1.3 Enterprise Expansion and Supervised Autonomy Foundations, v1.4 Enterprise Governance and Operator-Gated Full Autonomy, v1.5 Self-Hosted Product Modes and Lifecycle Packaging on 2026-03-27, and v1.6 Proper Onboarding and Setup on 2026-03-28
- **Archive:** `.planning/milestones/v1.0-*`, `.planning/milestones/v1.1-*`, `.planning/milestones/v1.2-*`, `.planning/milestones/v1.3-*`, `.planning/milestones/v1.4-*`, `.planning/milestones/v1.5-*`, `.planning/milestones/v1.6-*`
- **Planning state:** active milestone `v1.7 Documentation Convergence and OpenClaw-Inspired Docs Rewrite`
- **Known audit debt:** v1.0 archive still records missing phase `VERIFICATION.md` artifacts; v1.1 closes that workflow gap going forward

## Most Recent Milestone: v1.6 Proper Onboarding and Setup

**Result:** Shipped 2026-03-28. OpenRustClaw now has a truthful setup lifecycle: durable setup state, mode-aware bootstrap validation, explicit repair and reset-with-backup recovery, and one shared setup handoff surface across onboarding, Control UI, and docs.

**Archive:** `.planning/milestones/v1.6-ROADMAP.md`, `.planning/milestones/v1.6-REQUIREMENTS.md`, `.planning/milestones/v1.6-MILESTONE-AUDIT.md`, `.planning/milestones/v1.6-VERIFICATIONS.md`

## Current Milestone: v1.7 Documentation Convergence and OpenClaw-Inspired Docs Rewrite

**Goal:** Rewrite and converge the documentation so OpenRustClaw presents one clear self-hosted product story, one canonical information architecture, and one truthful operator path from README through deployment docs.

**Target features:**
- Rewrite the top-level README and docs landing surfaces around one product narrative, one guided entry path, and one audience-aware navigation model.
- Merge overlapping docs, delete stale or redundant pages, and make canonical ownership explicit across `README.md`, `docs/`, and `docs/src/`.
- Sync setup, operator, deployment, security, feature-matrix, roadmap, and product-positioning docs with the shipped runtime and control surfaces.
- Add a documentation maintenance contract so future feature milestones keep the canonical docs in sync instead of recreating parallel drift.

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

### Active

- [ ] Define one canonical documentation contract across `README.md`, repo-root docs mirrors, and the docs-site source tree
- [ ] Rewrite the public product story and top-level entry surfaces so OpenRustClaw reads like one coherent self-hosted product for solo users through enterprises
- [ ] Merge, delete, and sync redundant docs so setup, operator, deployment, and planning guidance no longer drift across parallel pages
- [ ] Add a docs maintenance rule so future milestones update canonical docs instead of reintroducing duplication

### Out of Scope

- Full unsupervised AGI that can autonomously run an entire business end-to-end — beyond the current trust and safety boundary
- Deep enterprise IAM, SCIM, tenant-aware RBAC, and procurement packaging — deferred while this milestone focuses on documentation convergence
- Another broad parity or feature-expansion milestone — this cycle is about making the shipped product legible, not widening scope again
- A pure marketing-site rewrite disconnected from the shipped docs and runtime surface — the goal is truthful product documentation, not branding alone

## Context

This remains a large brownfield Rust monorepo with broad runtime, CLI, control-plane, memory, tools, channel, voice, browser, deployment, and operator surfaces. v1.0 converted that breadth into a cleaner MVP by making operator trust visible at the edges that matter, v1.1 hardened the lifecycle and enterprise baseline around that trust, v1.2 deepened the most operator-visible OpenClaw parity surfaces without reopening MVP sprawl, v1.3 turned the first enterprise and supervised-autonomy contracts into a coherent operator loop, v1.4 extended that loop into explicit governance and operator-gated full autonomy, v1.5 made the platform legible as one self-hosted open-source product with explicit deployment paths and transition visibility, and v1.6 turned onboarding and setup into one truthful lifecycle from first install through repair and handoff.

The documentation now lags that product maturity. The repo currently spreads overlapping product, setup, operator, and planning narratives across `README.md`, repo-root planning mirrors under `docs/`, and the docs-site source under `docs/src/`. This milestone treats documentation drift as real product debt: merge what overlaps, delete what is stale, and rebuild the entry surfaces around a clearer OpenClaw-inspired model of concise positioning, guided onboarding, and canonical docs ownership.

## Constraints

- **Rust-first runtime:** The Rust runtime remains the primary production path
- **Truth over gloss:** Documentation changes must describe shipped behavior and supported paths, not aspirational marketing claims
- **Brownfield pragmatism:** Prefer converging or deleting existing docs over creating another layer of parallel files
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

## Next Milestone Goals

- Define and enforce one canonical documentation structure across `README.md`, `docs/`, and `docs/src/`
- Rewrite the public product story and top-level docs surfaces so new users can understand what OpenRustClaw is, who it is for, and where to start
- Merge or delete stale documents and sync setup, operator, deployment, security, and planning guidance to the shipped runtime surface
- Preserve the trust-first, enterprise, autonomy, self-hosted product-mode, and setup-handoff baselines while making them substantially easier to discover

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
*Last updated: 2026-03-27 after starting v1.7*

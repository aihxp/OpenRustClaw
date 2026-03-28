# OpenRustClaw

## What This Is

OpenRustClaw is a self-hosted open-source Rust-first OpenClaw-style assistant platform with shipped v1.0 through v1.10 milestones and an active v1.11 publishing milestone. It now has a coherent production-ready baseline across onboarding, assistant continuity, memory policy, tool and coding evidence, communications, runtime operations, security posture, release exit, milestone verification integrity, materially deeper operator parity across browser, orchestration, mobile, Control UI, and voice or call handling surfaces, an enterprise-first foundation for scoped operator identity, policy, audit export, supervised autonomy, governance, and an operator-gated full-autonomy lane, an explicit self-hosted product-mode contract across `solo`, `team`, `company`, and `enterprise` deployments, a truthful setup lifecycle from first install through repair and handoff, a canonical documentation contract that keeps the repo entrypoint, guided docs, and planning docs aligned, a cleaner codebase baseline with explicit cleanup inventory, repo-hygiene guardrails, an extracted control-auth boundary, a repaired tagged-release workflow that now produces truthful downloadable binary artifacts on GitHub, and a new push toward a truthful public Rust package surface on crates.io and docs.rs.

The product target remains broader than the MVP: a general-purpose assistant platform that can eventually support coding, communications, business operations, and deeper autonomous workflows. After v1.7, the repo also has one clearer documentation story instead of several drifting versions of the product. After v1.8, the repo also has a more explicit cleanup contract for keeping that surface maintainable.

## Core Value

Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## Current State

- **Shipped milestones:** v1.0 Rust OpenClaw MVP, v1.1 Lifecycle Integrity and Enterprise Foundations, v1.2 Deeper OpenClaw Surface Parity, v1.3 Enterprise Expansion and Supervised Autonomy Foundations, v1.4 Enterprise Governance and Operator-Gated Full Autonomy, v1.5 Self-Hosted Product Modes and Lifecycle Packaging, v1.6 Proper Onboarding and Setup, v1.7 Documentation Convergence and OpenClaw-Inspired Docs Rewrite, v1.8 Clean Codebase, v1.9 GitHub Repository Presence and Actions Recovery, and v1.10 Release Binaries Workflow Recovery
- **Archive:** `.planning/milestones/v1.0-*` through `.planning/milestones/v1.10-*`
- **Planning state:** v1.11 is active
- **Known audit debt:** v1.0 archive still records missing phase `VERIFICATION.md` artifacts; v1.1 closed that workflow gap going forward

## Most Recent Milestone: v1.10 Release Binaries Workflow Recovery

**Result:** Shipped 2026-03-28. OpenRustClaw now has a truthful tagged binary release path: the repaired `Release Binaries` workflow passes on supported GitHub-hosted runners, the publish job succeeds on live tags, and the public `v1.10-rc1` release exposes tarball plus checksum assets for all four supported targets.

**Archive:** `.planning/milestones/v1.10-ROADMAP.md`, `.planning/milestones/v1.10-REQUIREMENTS.md`, `.planning/milestones/v1.10-MILESTONE-AUDIT.md`, `.planning/milestones/v1.10-VERIFICATIONS.md`

## Current Milestone: v1.11 Crates.io and Docs.rs Publication Foundation

**Goal:** Turn the current workspace into a truthful first public Rust package surface by selecting the initial publishable crates, normalizing package metadata, hardening docs.rs documentation, and defining the publish-and-verify operator path for crates.io.

**Target features:**
- define which OpenRustClaw crates are public publish targets first, instead of trying to publish the whole workspace blindly
- normalize crate metadata needed for crates.io and public package discoverability
- make the selected public crates build and present correctly on docs.rs
- define and verify the first crates.io publication path, including dry-runs, publish order, and post-publish checks

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
- ✓ The public GitHub repo entry surface, discovery metadata, and GitHub workflow-health contract now match the shipped product and can be revalidated through repeatable admin helpers — v1.9 Phases 41-44
- ✓ Tagged GitHub releases now publish truthful tarball and checksum assets for all supported targets, with one repeatable operator validation path — v1.10 Phases 45-48

### Active

- [ ] A first public OpenRustClaw crate set is explicitly selected, with package boundaries and publishability rules documented for the workspace — v1.11
- [ ] Selected publishable crates expose truthful crates.io-ready metadata and packaging inputs — v1.11
- [ ] Selected public crates build cleanly on docs.rs with a documented docs contract — v1.11
- [ ] Operators have one documented and verifiable crates.io publication path for the first public release — v1.11

### Out of Scope

- Full unsupervised AGI that can autonomously run an entire business end-to-end — beyond the current trust and safety boundary
- Enterprise multi-tenancy, compliance packaging, deep RBAC/SSO governance, and procurement-driven controls — deferred until a future milestone explicitly scopes them
- Perfect parity across every OpenClaw surface, every channel, and every experimental lane — stabilize the core product and its documentation contract first
- A pure marketing-site rewrite disconnected from the shipped docs and runtime surface — the goal is truthful product documentation, not branding alone

## Context

This remains a large brownfield Rust monorepo with broad runtime, CLI, control-plane, memory, tools, channel, voice, browser, deployment, and operator surfaces. v1.0 converted that breadth into a cleaner MVP by making operator trust visible at the edges that matter, v1.1 hardened the lifecycle and enterprise baseline around that trust, v1.2 deepened the most operator-visible OpenClaw parity surfaces without reopening MVP sprawl, v1.3 turned the first enterprise and supervised-autonomy contracts into a coherent operator loop, v1.4 extended that loop into explicit governance and operator-gated full autonomy, v1.5 made the platform legible as one self-hosted open-source product with explicit deployment paths and transition visibility, v1.6 turned onboarding and setup into one truthful lifecycle from first install through repair and handoff, v1.7 made the documentation set legible enough to match the shipped product baseline, v1.8 converted cleanup debt into an explicit maintained contract instead of leaving it as background churn, v1.9 repaired the public GitHub repo surface, and v1.10 restored the final broken public automation lane around tagged binary releases.

The most recent milestone closed a trust gap at the public release edge: GitHub tags now produce truthful downloadable binary artifacts again, and the repo has one repeatable operator validation loop for future release tags. The next milestone can build on a working release-distribution baseline instead of first repairing it.

The current milestone extends that public distribution story from GitHub release binaries into the Rust ecosystem itself. The workspace is large and mixed-purpose, so the main risk is pretending every crate is ready for crates.io or docs.rs before the package surface, metadata, and documentation boundaries are explicit. v1.11 therefore focuses first on choosing the initial public crate set and making that surface truthful, then on the actual publication loop.

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
| Treat codebase cleanup as a milestone in its own right | The repo is now broad enough that oversized modules, stale paths, and contract drift directly slow down safe product work | ✓ Good |
| Treat GitHub repo metadata, topics, and Actions as part of the shipped product surface | The repo page and automation are the first operator touchpoints, so stale metadata or broken workflows undermine trust before users even clone the code | ✓ Good |
| Treat tagged binary release automation as part of the shipped public trust surface | A public tag that cannot produce downloadable artifacts undermines the repo's production-ready story even if the runtime itself is healthy | ✓ Good |
| Treat crates.io and docs.rs as a curated public package surface, not an automatic dump of the whole workspace | The workspace mixes internal crates, binaries, tests, and potential public libraries, so publishability has to be explicit and truthful | ✓ Good |

## Next Milestone Goals

- define the first publishable OpenRustClaw crate set and package contract
- align selected crate metadata and docs.rs rendering with the public product story
- establish the first repeatable crates.io publish and verification loop

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
*Last updated: 2026-03-28 after starting v1.11*

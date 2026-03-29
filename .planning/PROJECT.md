# OpenRustClaw

## What This Is

OpenRustClaw is a self-hosted open-source Rust-first OpenClaw-style assistant platform with shipped v1.0 through v1.28 milestones. It now has a coherent production-ready baseline across onboarding, assistant continuity, memory policy, tool and coding evidence, communications, runtime operations, security posture, release exit, milestone verification integrity, materially deeper operator parity across browser, orchestration, mobile, Control UI, and voice or call handling surfaces, an enterprise-first foundation for scoped operator identity, policy, audit export, supervised autonomy, governance, and an operator-gated full-autonomy lane, an explicit self-hosted product-mode contract across `solo`, `team`, `company`, and `enterprise` deployments, a truthful setup lifecycle from first install through repair and handoff, a canonical documentation contract that keeps the repo entrypoint, guided docs, and planning docs aligned, a cleaner codebase baseline with explicit cleanup inventory, repo-hygiene guardrails, an extracted control-auth boundary, a repaired tagged-release workflow that now produces truthful downloadable binary artifacts on GitHub, a first truthful public Rust package surface on crates.io and docs.rs, a bounded remote-connectivity story around node-first guidance with SSH tunnel and reverse-proxy fallbacks preserved explicitly in setup state and operator surfaces, and a greenfield application lane that now owns the full ranked seam inventory behind `openrustclaw-app`, with the retired historical ledger closed truthfully at `18/18` migrated seams, the adapter-only program closed at `6/6`, and the native-delivery roadmap now advanced to `4/8`.

The product target remains broader than the MVP: a general-purpose assistant platform that can eventually support coding, communications, business operations, and deeper autonomous workflows. After v1.7, the repo also has one clearer documentation story instead of several drifting versions of the product. After v1.8, the repo also has a more explicit cleanup contract for keeping that surface maintainable.

## Core Value

Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## Current State

- **Shipped milestones:** v1.0 Rust OpenClaw MVP, v1.1 Lifecycle Integrity and Enterprise Foundations, v1.2 Deeper OpenClaw Surface Parity, v1.3 Enterprise Expansion and Supervised Autonomy Foundations, v1.4 Enterprise Governance and Operator-Gated Full Autonomy, v1.5 Self-Hosted Product Modes and Lifecycle Packaging, v1.6 Proper Onboarding and Setup, v1.7 Documentation Convergence and OpenClaw-Inspired Docs Rewrite, v1.8 Clean Codebase, v1.9 GitHub Repository Presence and Actions Recovery, v1.10 Release Binaries Workflow Recovery, v1.11 Crates.io and Docs.rs Publication Foundation, v1.12 Secure Node Connectivity and SSH Tunnel Revisit, v1.13 Brownfield-to-Greenfield Transition, v1.14 Continued Greenfield Conversion, v1.15 Deeper Greenfield Conversion, v1.16 Greenfield Conversion: Skills and Runtime Hotspots, v1.17 Greenfield Conversion: Completion Metrics and Remaining Hotspots, v1.18 Greenfield Conversion: Final Ranked Seam and 100% Completion Path, v1.19 Full Greenfield Conversion: Control Plane Route Families I, v1.20 Full Greenfield Conversion: Control Plane Route Families II, v1.21 Full Greenfield Conversion: Mobile and Voice Runtime Services, v1.22 Full Greenfield Conversion: Orchestration and Browser Services, v1.23 Full Greenfield Conversion: Setup and Secondary Command Surfaces, v1.24 Full Greenfield Conversion: Adapter-Only Exit and Enforcement, v1.25 Native Delivery Layer: Port Contracts and Legacy Inventory, v1.26 Native Delivery Layer: Control, MCP, and Gateway Delivery, v1.27 Native Delivery Layer: CLI Core Dispatch and Operator Commands I, and v1.28 Native Delivery Layer: CLI Operator Commands II and UI-Adjacent Flows
- **Archive:** `.planning/milestones/v1.0-*` through `.planning/milestones/v1.28-*`
- **Planning state:** active milestone `v1.29 Native Delivery Layer: Runtime Hosts and Background Workers`
- **Known audit debt:** v1.0 archive still records missing phase `VERIFICATION.md` artifacts; v1.1 closed that workflow gap going forward
- **Greenfield conversion baseline:** canonical `18/18` ranked seams migrated, or `100%` complete, across the retired historical transition inventory; the follow-on full-conversion roadmap is also closed at `6/6` shipped milestones, or `100%`; the native-delivery roadmap now stands at `4/8` shipped milestones, or `50%`, with `v1.29` targeting `5/8`, or about `63%`

## Most Recent Milestone: v1.28 Native Delivery Layer: CLI Operator Commands II and UI-Adjacent Flows

**Result:** Shipped 2026-03-28. OpenRustClaw closed the fourth native-delivery milestone by defining the remaining large operator CLI families, the secondary operator and utility families, the CLI dependency-removal rules, and the UI-adjacent delivery alignment needed for the second CLI implementation slice. The native-delivery roadmap now stands at `4/8` shipped milestones, or `50%`.

**Archive:** `.planning/milestones/v1.28-ROADMAP.md`, `.planning/milestones/v1.28-REQUIREMENTS.md`, `.planning/milestones/v1.28-MILESTONE-AUDIT.md`, `.planning/milestones/v1.28-VERIFICATIONS.md`

## Current Milestone: v1.29 Native Delivery Layer: Runtime Hosts and Background Workers

**Goal:** Continue replacing the legacy delivery layer by defining the runtime-host and background-worker entrypoints over app ports so startup and worker lifecycle ownership can leave the legacy command layer.

**Target features:**
- dedicated runtime-host and background-worker entrypoints over app ports
- service-manager, probe-runner, runtime-maintenance, and scheduler startup boundaries
- mobile, voice, and orchestration worker boot contracts aligned with native delivery
- removal of legacy command ownership for worker lifecycle startup

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
- ✓ Node-first remote connectivity, SSH tunnel fallback, reverse-proxy last resort, and the saved remote-connectivity profile are now aligned across onboarding, setup state, operator handoff, and docs — v1.12 Phases 53-56
- ✓ The repo now has a canonical greenfield transition contract with explicit containment rules, target layers, and a chosen first proving slice around setup handoff reporting — v1.13 Phase 57
- ✓ The repo now has a real greenfield application shell in `openrustclaw-app` plus a stable setup-handoff service boundary for future adapters — v1.13 Phase 58
- ✓ The setup handoff proving slice now runs through `openrustclaw-app`, with CLI code reduced to a bounded adapter that preserves the existing runtime and Control UI contract — v1.13 Phase 59
- ✓ Contributor and planning defaults now make `openrustclaw-app` the default application lane, preserve compatibility-only exceptions for legacy hotspots, and keep the next migration queue explicit — v1.13 Phase 60
- ✓ A second typed operator summary family now runs through `openrustclaw-app`, with self-hosted product-mode report composition moved out of `inspect.rs` and into the greenfield lane — v1.14 Phase 61
- ✓ The `/control/self-hosted/product-mode` route family now runs through a greenfield application service seam, with `start.rs` reduced to the HTTP adapter for the migrated transition path — v1.14 Phase 62
- ✓ The mobile node operator summary report now runs through `openrustclaw-app`, with `mobile.rs` reduced to the workspace adapter for the migrated operator-facing mobile report — v1.14 Phase 63
- ✓ The compiled-skill overview lane now runs through `openrustclaw-app`, with `skills.rs` and `start.rs` reduced to adapters for the shipped compiled-skill CLI and MCP/runtime surface — v1.14 Phase 64
- ✓ The enterprise admin aggregation now runs through `openrustclaw-app`, with `inspect.rs` reduced to the workspace adapter for enterprise access, policy, autonomy, and supervision inputs — v1.15 Phase 65
- ✓ The enterprise access write route family now runs through `openrustclaw-app`, with `start.rs` reduced to the HTTP adapter for bootstrap, operator-upsert, and governance-rule-upsert orchestration — v1.15 Phase 66
- ✓ The install, update, and uninstall skills mutation lane now runs through `openrustclaw-app`, with `skills.rs` reduced to the adapter around workspace files, DB state, registry operations, compile attempts, and plugin-event publication — v1.15 Phase 67
- ✓ The runtime switch-provider and switch-model lane now runs through `openrustclaw-app`, with `runtime.rs` reduced to the adapter around config loading, provider validation, and config persistence with backup — v1.15 Phase 68
- ✓ The voice-plugin bind mutation lane now runs through `openrustclaw-app`, with `skills.rs` reduced to the adapter around compiled-skill details, registry persistence, and plugin-event publication — v1.16 Phase 69
- ✓ The runtime vault set and delete mutation lane now runs through `openrustclaw-app`, with `runtime.rs` reduced to the workspace vault I/O adapter — v1.16 Phase 70
- ✓ The runtime reload-plan seam now runs through `openrustclaw-app`, with `runtime.rs` reduced to the snapshot and reload-state adapter — v1.16 Phase 71
- ✓ The `/control/runtime/vault` route family now runs through `openrustclaw-app`, with `start.rs` reduced to the HTTP adapter for the migrated runtime vault control flow — v1.16 Phase 72
- ✓ Greenfield conversion progress is now measured against a canonical ranked seam inventory instead of milestone count or anecdotal status — v1.17 Phase 73
- ✓ One remaining auth-plugin lifecycle lane now runs through `openrustclaw-app`, with `skills.rs` reduced to the adapter around compiled artifacts, registry persistence, and plugin-event publication — v1.17 Phase 74
- ✓ Runtime upgrade, self-update, and rollback planning now run through `openrustclaw-app`, with `runtime.rs` reduced to the adapter around status, health, locks, service-manager support, and artifact metadata — v1.17 Phase 75
- ✓ `/control/runtime/maintenance` now reports the canonical greenfield score and remaining queue through `openrustclaw-app`, with the post-milestone baseline now at `17/18` migrated seams — v1.17 Phase 76
- ✓ The remaining channel-extension and background workflow lifecycle lane now runs through `openrustclaw-app`, with `skills.rs` reduced to the adapter around background workflow scheduling, binding persistence, and plugin-event publication — v1.18 Phase 77
- ✓ The canonical seam inventory and shipped progress surface now report `18/18` migrated seams and `100%` completion from the maintained ledger — v1.18 Phase 78
- ✓ The current ranked seam inventory now retires explicitly at `18/18`, with the closure rule recorded before any deeper follow-on queue can exist — v1.18 Phase 79
- ✓ Shipped progress and contributor-facing planning surfaces now preserve the retired-ledger decision and require an explicit new canonical queue for future deeper greenfield work — v1.18 Phase 80
- ✓ The remaining control config route family now runs through `openrustclaw-app`, with `start.rs` reduced to the HTTP adapter for `/control/config`, `/control/config/validate`, and `/control/config/update` — v1.19 Phase 81
- ✓ The diagnostics summary and live operator event route family now run through `openrustclaw-app`, with `start.rs` reduced to the transport adapter for those surfaces — v1.19 Phase 82
- ✓ Channel registry account and binding lifecycle mutations now run through `openrustclaw-app`, with `start.rs` reduced to the HTTP adapter for the shipped registry mutation routes — v1.19 Phase 83
- ✓ The remaining read-heavy runtime, voice, talk, and mobile status route family now runs through `openrustclaw-app`, with `start.rs` reduced to the HTTP adapter for those status surfaces — v1.19 Phase 84
- ✓ The autonomy lesson and lesson-mutation route families now run through `openrustclaw-app`, with `start.rs` reduced to the HTTP adapter for the shipped lesson-control surfaces — v1.20 Phase 85
- ✓ The remaining non-voice-call skill-control route families now run through `openrustclaw-app`, with `start.rs` reduced to the HTTP adapter for those skill-control surfaces — v1.20 Phase 86
- ✓ Voice-call lifecycle and channel-extension control routes now run through `openrustclaw-app`, with `start.rs` reduced to the HTTP adapter for those control surfaces — v1.20 Phase 87
- ✓ The migrated control-plane route map is now reduced behind focused route-builder helpers, which simplifies shared `start.rs` registration and state wiring after the second control-plane queue — v1.20 Phase 88
- ✓ The targeted mobile notification, outbound-message, dispatch, approval, wake, and rehydrate lifecycle lane now runs through `openrustclaw-app`, with `mobile.rs` reduced to the adapter around workspace persistence and bounded runtime execution — v1.21 Phase 89
- ✓ The targeted mobile heartbeat, push, sync, activity, node summary, and metrics aggregation lane now runs through `openrustclaw-app`, with `mobile.rs` reduced to the adapter around runtime receipts and summary inputs — v1.21 Phase 90
- ✓ The targeted voice provider-resolution and session lifecycle mutation lane now runs through `openrustclaw-app`, with `voice_runtime.rs` reduced to the adapter around synthesis, metadata, and session-file persistence — v1.21 Phase 91
- ✓ The targeted voice transcript, event, artifact, metrics, and outcome composition lane now runs through `openrustclaw-app`, with `voice_runtime.rs` reduced to the adapter around session loading and artifact metadata probing — v1.21 Phase 92
- ✓ The targeted orchestration route-selection, override-validation, and intervention-transition lane now runs through `openrustclaw-app`, with `orchestrate.rs` reduced to the adapter around registry, model, and active-run persistence inputs — v1.22 Phase 93
- ✓ The targeted orchestration reporting, reflection, and supervision-summary lane now runs through `openrustclaw-app`, with `orchestrate.rs` reduced to the adapter around receipt and active-run reads — v1.22 Phase 94
- ✓ The targeted browser backend-policy and audit-shaping lane now runs through `openrustclaw-app`, with `browser.rs` reduced to the adapter around audit-log file reads and appends — v1.22 Phase 95
- ✓ The targeted browser session-record and workflow-bookkeeping lane now runs through `openrustclaw-app`, with `browser.rs` reduced to the adapter around browser automation and session-file persistence — v1.22 Phase 96
- ✓ The targeted onboarding, repair, and resume orchestration lane now runs through `openrustclaw-app`, with `onboard.rs` reduced to the adapter around setup state, workspace I/O, and bounded runtime probes — v1.23 Phase 97
- ✓ The targeted secondary lifecycle seams now run through `openrustclaw-app`, with `channels.rs`, `schedule.rs`, `services.rs`, and `control.rs` reduced toward adapters around metadata, persistence, and bounded side effects — v1.23 Phase 98
- ✓ The targeted secondary operator-helper, media, tools, and memory seams now run through `openrustclaw-app`, with the affected secondary command modules reduced toward adapters around artifact, runtime, and workspace inputs — v1.23 Phase 99
- ✓ Transition-era helper duplication introduced during the migration is now reduced and bounded, leaving the affected secondary command modules with clearer adapter-only ownership — v1.23 Phase 100
- ✓ The final targeted helper-owned continuity, audit, voice-call reporting, and compiled-skill MCP seams now run through `openrustclaw-app`, further shrinking the remaining legacy hotspots — v1.24 Phase 101
- ✓ The final targeted persistence and external-integration boundaries now use named adapter seams in the remaining hotspots instead of mixed helper ownership — v1.24 Phase 102
- ✓ Source-level guardrails and contributor-facing enforcement defaults now preserve the adapter-only contract after the full-conversion program ships — v1.24 Phase 103
- ✓ The full-conversion roadmap now closes at `6/6`, or `100%`, with a truthful exit audit, verification bundle, and scorecard — v1.24 Phase 104
- ✓ The remaining legacy delivery surface is now inventoried explicitly, with each major family mapped to a target native home instead of being left as vague “future cleanup” — v1.25 Phase 105
- ✓ The repo now has a canonical app-port catalog for native CLI, control, MCP, runtime-host, and repository-facing delivery work — v1.25 Phase 106
- ✓ The native-delivery roadmap now defines a successor topology around `openrustclaw-app`, `openrustclaw-cli`, `openrustclaw-gateway`, `openrustclaw-mcp`, and planned runtime-host plus infrastructure layers — v1.25 Phase 107
- ✓ The native-delivery roadmap now has an explicit denominator, deletion gates, compatibility states, and shutdown rules before any future milestone claims legacy retirement — v1.25 Phase 108
- ✓ The native control HTTP delivery layer is now mapped explicitly to `openrustclaw-gateway` over `ControlPlanePort`, with route ownership separated from the `start.rs` hotspot — v1.26 Phase 109
- ✓ The native MCP server delivery layer is now mapped explicitly to `openrustclaw-mcp` over `McpServerPort`, with tool-catalog and invocation ownership separated from `start.rs` — v1.26 Phase 110
- ✓ The gateway bootstrap split and first truthful `start.rs` retirement slice are now defined explicitly, with control, websocket, webhook, and MCP startup ownership routed to native delivery entrypoints — v1.26 Phase 111
- ✓ The Control UI serving and wiring path is now aligned to the native gateway delivery contract, with compatibility rules stated for the shipped UI while legacy bootstrap ownership is reduced — v1.26 Phase 112
- ✓ The native CLI dispatch layer is now mapped explicitly so `main.rs` can shrink toward binary bootstrap while app-port-based delivery modules take over routing ownership — v1.27 Phase 113
- ✓ Assistant, chat, session, and inspect flows now have an explicit first core native CLI delivery family over `AssistantConversationPort`, `SessionManagementPort`, and `InspectionPort` — v1.27 Phase 114
- ✓ Control and runtime CLI entrypoints now have an explicit native delivery ownership path over `ControlPlanePort` and `RuntimeOperationsPort` instead of defaulting to legacy command hubs — v1.27 Phase 115
- ✓ Parsing, rendering, app invocation boundaries, and the temporary compatibility shim plan are now explicit enough to implement the first native CLI slice safely — v1.27 Phase 116
- ✓ The remaining large operator-facing command families now have an explicit native CLI delivery path over app ports instead of defaulting to legacy command ownership — v1.28 Phase 117
- ✓ The remaining secondary operator and utility command families now have an explicit native delivery path grouped by app-port and delivery concerns rather than legacy file layout — v1.28 Phase 118
- ✓ The roadmap now defines how the remaining CLI families avoid command-to-command orchestration dependencies as native delivery modules take over — v1.28 Phase 119
- ✓ UI-adjacent operator surfaces now align to native entrypoints explicitly enough to keep the second CLI slice end to end and truthful — v1.28 Phase 120

### Active

- `NDL-17` Dedicated runtime-host and background-worker entrypoints over app ports
- `NDL-18` Service-manager, probe-runner, runtime-maintenance, and scheduler startup boundaries
- `NDL-19` Mobile, voice, and orchestration worker boot contracts aligned with native delivery
- `NDL-20` Removal of legacy command ownership for worker lifecycle startup

### Out of Scope

- Full unsupervised AGI that can autonomously run an entire business end-to-end — beyond the current trust and safety boundary
- Enterprise multi-tenancy, compliance packaging, deep RBAC/SSO governance, and procurement-driven controls — deferred until a future milestone explicitly scopes them
- Perfect parity across every OpenClaw surface, every channel, and every experimental lane — stabilize the core product and its documentation contract first
- A pure marketing-site rewrite disconnected from the shipped docs and runtime surface — the goal is truthful product documentation, not branding alone

## Context

This remains a large brownfield Rust monorepo with broad runtime, CLI, control-plane, memory, tools, channel, voice, browser, deployment, and operator surfaces. v1.0 converted that breadth into a cleaner MVP by making operator trust visible at the edges that matter, v1.1 hardened the lifecycle and enterprise baseline around that trust, v1.2 deepened the most operator-visible OpenClaw parity surfaces without reopening MVP sprawl, v1.3 turned the first enterprise and supervised-autonomy contracts into a coherent operator loop, v1.4 extended that loop into explicit governance and operator-gated full autonomy, v1.5 made the platform legible as one self-hosted open-source product with explicit deployment paths and transition visibility, v1.6 turned onboarding and setup into one truthful lifecycle from first install through repair and handoff, v1.7 made the documentation set legible enough to match the shipped product baseline, v1.8 converted cleanup debt into an explicit maintained contract instead of leaving it as background churn, v1.9 repaired the public GitHub repo surface, v1.10 restored the final broken public automation lane around tagged binary releases, and v1.11 extended that distribution story into the Rust ecosystem through the first truthful crates.io and docs.rs publication path.

The most recent milestone changed the implementation posture instead of adding another wide product surface. v1.13 created a greenfield-style lane inside the existing repo, proved it with a shipped setup handoff slice, and turned that lane into the default contribution contract for follow-on work.

The most recent milestone broadened that work into the next ranked migration queue. Instead of stopping at one proving slice, `v1.14` extended the greenfield lane across inspection summaries, selected control routes, mobile operator reporting, and the first bounded `skills.rs` cleanup seam.

The most recent milestone deepened that same migration strategy without changing the contract. `v1.15` closed its planned queue: another inspection aggregate, another route family, the first mutation-heavy `skills.rs` seam, and the first bounded runtime command seam are now all completed.

The most recent milestone kept that same posture and closed the next ranked hotspot queue. `v1.16` moved the remaining voice-plugin binding lane, the runtime vault mutation seam, the runtime reload-plan seam, and the `/control/runtime/vault` route family behind `openrustclaw-app`, which further reduced direct legacy coupling in `skills.rs`, `runtime.rs`, and `start.rs`.

The most recent milestone finished the current ranked migration queue. `v1.18` moved the last remaining channel-extension and background workflow lifecycle seam behind `openrustclaw-app`, advanced the canonical baseline to `18/18`, and retired the current ranked ledger with an explicit rule that any deeper follow-on queue must be defined separately.

The most recent milestones turned that follow-on decision into a real execution program. `v1.19` started the broader full-conversion roadmap by extracting the first four remaining control-plane route families from `start.rs`, `v1.20` completed the second control-plane queue while advancing the roadmap baseline to `2/6` shipped milestones, `v1.21` completed the next operator-runtime queue around `mobile.rs` and `voice_runtime.rs`, `v1.22` completed the orchestration and browser queue, `v1.23` completed the setup plus secondary command-surface queue, and `v1.24` closed the program by extracting the final targeted helper seams, formalizing named adapter boundaries, and shipping the guardrails plus exit audit that make the adapter-only claim durable.

The next follow-on queue is stricter than adapter-only completion. `v1.25` started the native-delivery roadmap, which treats `main.rs`, `crates/cli/src/commands/*`, `start.rs`, and the worker bootstraps as legacy delivery surfaces to be replaced by native entrypoints built directly on app ports instead of being kept indefinitely as compatibility shells.

The next native-delivery queue moves from planning the replacement to defining the first real execution slice. `v1.26` targets the control HTTP layer, MCP server delivery, gateway bootstrap split, and Control UI serving alignment so the repo can start retiring `start.rs` as the central bootstrap hotspot.

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
| Revisit nodes and SSH tunnel connectivity as product work rather than leaving it as operator improvisation | The repo already has distributed and mobile node surfaces, but the setup story still treats remote exposure as an external workaround instead of a supported deployment contract | ✓ Good |
| Make remote connectivity node-first with an SSH tunnel fallback instead of treating the tunnel as the primary transport | The product should preserve the cleaner node model where available, but operators still need a durable recovery path when direct node connectivity is broken or unsupported | ✓ Good |
| Keep reverse proxy as a bounded third-tier fallback instead of promoting it to the default remote path | Reverse proxy exposure can help recovery in difficult self-hosted environments, but it should remain a clearly constrained last resort behind the node-first and SSH tunnel paths | ✓ Good |
| Treat the brownfield-to-greenfield shift as a staged carve-out inside the shipped repo rather than a rewrite-from-scratch reset | The product already has real users and operator surfaces, so the safer path is to create a clean lane and migrate into it with compatibility boundaries | ✓ Good |
| Use setup handoff reporting as the first greenfield proving slice | It crosses setup state, report composition, route exposure, and Control UI rendering while already having bounded regression tests | ✓ Good |
| Use `openrustclaw-app` as the first application shell instead of extending the CLI crate into a second mixed-responsibility hub | The transition needs one bounded home for services, but the new lane should not immediately inherit transport and command concerns from `openrustclaw-cli` | ✓ Good |
| Keep durable setup-state persistence in the CLI onboarding module for the first migrated slice while moving report composition into `openrustclaw-app` | The proving slice needed to shrink report ownership first without expanding migration scope into storage or onboarding behavior | ✓ Good |
| Treat large command modules as compatibility surfaces unless a migration phase explicitly targets them | The transition only works if future contributors stop treating legacy hotspots as the default home for every new behavior | ✓ Good |
| Keep persisted self-hosted product-mode storage and transition receipts in the existing CLI adapter module while moving summary composition into `openrustclaw-app` | The second summary extraction needed to broaden the greenfield lane without mixing storage migration into the same bounded slice | ✓ Good |
| Use the self-hosted product-mode transition route as the first bounded `start.rs` route-family extraction | It already had a matching greenfield summary service, one GET and one POST contract, and shipped Control UI coverage, which made it the safest truthful proving route | ✓ Good |
| Use the compiled-skill overview lane as the first `skills.rs` seam instead of attempting a broad skills rewrite | Compiled manifests, artifacts, reference previews, and executable-component discovery already powered both CLI and MCP/runtime surfaces, so extracting that read-only lane created a real shared boundary with bounded risk | ✓ Good |
| Use the enterprise admin surface as the next inspection aggregation extraction instead of trying to move every enterprise summary at once | It is a real shipped aggregation over access, policy, autonomy, and supervision, so moving that composition first reduces `inspect.rs` ownership without forcing a broad enterprise rewrite in one phase | ✓ Good |
| Use the enterprise access write family as the next bounded `start.rs` route extraction | The bootstrap, operator-upsert, and governance-rule-upsert handlers all shared the same mutation-and-report pattern, so extracting them together reduced real route coupling without changing the shipped enterprise summary contract | ✓ Good |
| Use the install, update, and uninstall lane as the first mutation-heavy `skills.rs` extraction | That lane already powered the shipped control API and combined registry calls, policy checks, DB writes, compile attempts, and mutation-result shaping, so moving it first created a real service seam instead of another read-only helper split | ✓ Good |
| Use the provider or model switch lane as the first bounded runtime command extraction | That lane already powered the shipped CLI and control API, owned real config mutation plus backup persistence, and stayed narrow enough to migrate without reopening the larger backup, reload, or upgrade surfaces in the same phase | ✓ Good |
| Continue greenfield conversion by prioritizing the remaining `skills.rs` plugin lanes and larger `runtime.rs` seams before reopening broad new migration targets | The next honest hotspots are now the remaining mutation-heavy skills behavior and the larger runtime mutation and recovery lanes, with another route extraction only where it materially reduces coupling around those seams | ✓ Good |
| Keep the v1.16 queue focused on remaining plugin-binding plus runtime mutation, reload-planning, and follow-on runtime control seams | Those were the highest-value remaining hotspots that could shrink `skills.rs`, `runtime.rs`, and `start.rs` further without broadening the milestone into another rewrite | ✓ Good |
| Measure greenfield conversion against a ranked seam inventory instead of raw phase count or line count | Phase count alone overstates progress, while file size alone hides shipped boundary wins; a seam inventory is the most honest way to report conversion completion | ✓ Good |
| Ship the greenfield progress surface from the same canonical inventory used for milestone planning | The percentage surface would drift immediately if it duplicated state or derived progress from milestone count instead of the maintained seam ledger | ✓ Good |
| Retire the current ranked seam inventory at `18/18` and require any deeper follow-on work to define a new canonical queue explicitly | Preserving a fixed completed denominator keeps historical progress truthful and prevents future work from silently rewriting the meaning of the shipped `100%` baseline | ✓ Good |
| Treat post-`18/18` full conversion as an adapter-only architecture program rather than as an attempt to maximize lines moved into one crate | The real target is ownership of business logic and stable boundaries, not raw line migration or a cosmetic crate split | ✓ Good |
| Close the full-conversion program only after the remaining hotspots expose named adapter seams plus durable source-level guardrails | Reaching `6/6` is only truthful if the repo can defend the adapter-only claim after shipment instead of relying on milestone memory alone | ✓ Good |
| Start a separate native-delivery roadmap after `v1.24` instead of pretending adapter-only completion equals full legacy retirement | Replacing the remaining delivery layer is a different kind of work, needs a new denominator, and must include `main.rs`, control or MCP bootstrap, worker entrypoints, and repository wiring | ✓ Good |
| Replace the legacy command tree with native delivery layers built directly around app ports before deleting the old modules | A clean greenfield product needs stable successor entrypoints first; deleting compatibility shells before the native delivery layer exists would trade architectural cleanliness for regressions | ✓ Good |

## Current Program Status

- the retired historical greenfield ledger remains closed at `18/18`, or `100%`
- the broader adapter-only full-conversion roadmap is now also closed at `6/6`, or `100%`
- the native-delivery roadmap now stands at `4/8`, or `50%`, with `v1.29` targeting `5/8`, or about `63%`
- contributor defaults and source-level tests still preserve `openrustclaw-app` as the default home for new business logic while `v1.29` starts the runtime-host and background-worker replacement slice
- future legacy-retirement work now routes through `.planning/codebase/NATIVE-DELIVERY-LAYER-ROADMAP.md` instead of quietly extending the finished denominators

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
*Last updated: 2026-03-28 after starting v1.29 milestone*

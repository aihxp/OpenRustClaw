# Native Delivery Layer Roadmap

**Created:** 2026-03-28
**Purpose:** Canonical follow-on roadmap for replacing the remaining legacy delivery layer with native delivery surfaces built directly around `openrustclaw-app` ports and explicit infrastructure adapters.
**Status:** Active after `v1.25` at `1/8` shipped milestones, or about `13%`
**Baselines preserved:** historical greenfield seam ledger closed at `18/18`; full-conversion roadmap closed at `6/6`

## What "Move Completely Off Legacy" Means

OpenRustClaw should treat legacy exit as a delivery-architecture outcome, not as a promise that every old file disappears overnight.

The repo is fully off legacy when:

- top-level product entrypoints no longer depend on `crates/cli/src/commands/*` as the primary delivery surface
- CLI, control HTTP, MCP, and runtime-worker delivery paths call `openrustclaw-app` ports directly
- workspace persistence, process control, audit logs, registries, and external integrations sit behind explicit infrastructure adapters instead of command-local helpers
- `crates/cli/src/main.rs` becomes a thin bootstrap or disappears behind dedicated delivery crates or binaries
- legacy command modules are deleted, frozen as compatibility shims, or isolated outside the main product path

## v1.25 Outcome

`v1.25` did not delete legacy delivery code yet. It did the prerequisite work needed to retire it safely:

- inventoried the remaining legacy delivery surface instead of treating it as a vague follow-on idea
- defined the app-port families needed to replace the legacy command tree
- chose a successor native delivery topology that reuses `openrustclaw-gateway` and `openrustclaw-mcp` while planning new runtime-host and infrastructure layers
- established explicit shutdown gates so future milestones can delete or isolate legacy delivery code without guessing when the replacement path is ready

## Remaining Legacy Delivery Inventory

### Target Map

| Delivery Family | Current Surface | Current Signal | Native Home |
| --- | --- | --- | --- |
| Top-level binary dispatch | `crates/cli/src/main.rs` (`6129` lines), `crates/cli/src/commands/mod.rs` | the main binary still routes almost every shipped flow through the legacy command tree | thin `openrustclaw-cli` bootstrap plus native CLI delivery modules over app ports |
| Control and MCP bootstrap | `crates/cli/src/commands/start.rs` (`15786` lines), `control_ui.rs`, `control_ui.html`, `mcp2cli.rs` | control routes, MCP registration, and server startup still originate in one legacy hotspot | `openrustclaw-gateway` for control and UI delivery, `openrustclaw-mcp` for MCP delivery |
| Primary operator command delivery | `skills.rs` (`5479`), `mobile.rs` (`6230`), `voice_runtime.rs` (`3125`), `orchestrate.rs` (`4559`), `browser.rs` (`3269`), `runtime.rs` (`3511`), `inspect.rs` (`2169`), `onboard.rs` (`2600`) | request parsing, rendering, and workspace adaptation still live in legacy modules even after app-side logic extraction | native CLI delivery modules grouped by domain over app ports |
| Secondary operator and utility delivery | `services.rs`, `memory.rs`, `media.rs`, `tools.rs`, `channels.rs`, `control.rs`, `schedule.rs`, plus smaller utility commands | these files still use the legacy command-tree contract even where app services already exist | native CLI delivery modules or dedicated delivery helpers over app ports |
| Channel and communication delivery | `gmail.rs`, `google_chat.rs`, `imessage.rs`, `matrix.rs`, `meet.rs`, `signal.rs`, `webhooks.rs`, `whatsapp.rs` | external channel entrypoints still assume the command-tree layout | native communication delivery modules over app ports plus infrastructure gateways |
| Runtime and worker bootstraps | `start.rs`, `runtime.rs`, `services.rs`, `schedule.rs`, `mobile.rs`, `voice_runtime.rs` | service start, worker startup, probes, and runtime maintenance still pass through legacy delivery surfaces | dedicated `openrustclaw-runtime-host` entrypoints over app ports |
| Persistence and integration adaptation | `inspect.rs`, `skills.rs`, `runtime.rs`, `services.rs`, `channels.rs`, `memory.rs`, `control.rs` | command files still know too much about filesystem layout, sqlite access, registries, and runtime side effects | explicit repositories and infrastructure adapters behind app ports |

### Delivery Families Still Driving the Product Path

These are the remaining large delivery hotspots the native roadmap must replace in future milestones:

- `crates/cli/src/main.rs`
- `crates/cli/src/commands/start.rs`
- `crates/cli/src/commands/mobile.rs`
- `crates/cli/src/commands/skills.rs`
- `crates/cli/src/commands/orchestrate.rs`
- `crates/cli/src/commands/runtime.rs`
- `crates/cli/src/commands/browser.rs`
- `crates/cli/src/commands/voice_runtime.rs`
- `crates/cli/src/commands/onboard.rs`
- `crates/cli/src/commands/inspect.rs`

## App Port Contract Catalog

The native delivery layer needs named port families before the legacy command tree can be retired truthfully.

### Delivery Ports

| Port Family | Used By | Responsibility |
| --- | --- | --- |
| `AssistantConversationPort` | CLI assistant and chat delivery | conversation requests, streaming, model selection, and reply shaping |
| `SessionManagementPort` | CLI session delivery, gateway sessions, inspection surfaces | list, show, spawn, send, close, and resume persisted sessions |
| `InspectionPort` | CLI inspect delivery, control inspection routes | operator summaries, assistant continuity, tool audit history, setup or product inspection, and derived reports |
| `ControlPlanePort` | native control HTTP delivery | control-plane reads and writes for runtime, skills, setup, enterprise, channels, and operator state |
| `McpServerPort` | native MCP delivery | tool catalog, tool invocation, compiled-skill MCP exposure, and external MCP bridge surfaces |
| `RuntimeOperationsPort` | CLI runtime delivery, worker hosts | runtime config mutation, vault, reload, maintenance, status, and runtime process coordination |
| `SkillLifecyclePort` | CLI skills delivery, control mutation routes, MCP compiled-skill flows | install, update, bind, compile, inspect, call, and schedule skill workflows |
| `MobileOperationsPort` | CLI mobile delivery, worker hosts | notification, dispatch, approval, wake, heartbeat, sync, and runtime node summaries |
| `VoiceRuntimePort` | CLI voice delivery, worker hosts | provider resolution, session lifecycle, transcript, artifact, outcome, and metrics flows |
| `OrchestrationPort` | CLI orchestration delivery, control surfaces, workers | routing, intervention, checkpoint, reflection, supervision, and trace summary flows |
| `BrowserAutomationPort` | CLI browser delivery, control surfaces | backend policy, audit, session lifecycle, workflow execution, and inspection |
| `SetupLifecyclePort` | CLI onboarding delivery, inspect or control setup surfaces | onboarding, repair, resume, setup state, probes, and handoff lifecycle |
| `ChannelOperationsPort` | CLI channel delivery, services, schedule | channel bindings, schedules, probes, health, and channel-runtime lifecycle |
| `ArtifactMediaPort` | CLI media, tools, memory-adjacent flows | inspect, describe, extract, render, and artifact helper surfaces |
| `MemoryOperationsPort` | CLI memory delivery | export, import, search, recall views, artifact syncing, and namespace or archive operations |
| `SecurityAuditPort` | CLI security and doctor delivery | posture summaries, diagnostics, readiness, audit, and repair guidance |
| `CommunicationChannelPort` | channel-specific delivery surfaces | message send, receive, webhook, and provider-specific operator actions across Gmail, Matrix, Signal, WhatsApp, and peers |

### Infrastructure and Repository Ports

| Port Family | Responsibility |
| --- | --- |
| `WorkspaceRepository` | workspace paths, control files, compiled-skill caches, filesystem layout, and artifact roots |
| `SessionRepository` | session persistence, status transitions, and history access |
| `RuntimeRepository` | runtime config, vault, reload state, maintenance artifacts, and runtime status persistence |
| `SkillRepository` | skill registry state, compiled artifacts, binding state, plugin metadata, and compile receipts |
| `ChannelRepository` | channel accounts, bindings, probe state, and schedule or health persistence |
| `MemoryRepository` | memory store, archive store, views, and artifact-sync persistence |
| `AuditLogRepository` | tool audit history, control audit trails, browser audits, and operator evidence surfaces |
| `SetupRepository` | onboarding or repair state, handoff summaries, and diagnostics receipts |
| `SchedulerGateway` | durable job scheduling, background workflow startup, and next-run computation |
| `ServiceManagerGateway` | systemd or service-manager interactions, runtime process control, and worker lifecycle hooks |
| `ExternalServiceGateway` | outbound provider or channel integrations that should no longer be wired in legacy command helpers |

## Successor Delivery Layer Topology

### Target Layers

| Layer | Target Home | Responsibility |
| --- | --- | --- |
| Application ports and orchestration | `openrustclaw-app` | request or response contracts, use-case orchestration, policy decisions, reporting, and port traits |
| Native CLI delivery | `openrustclaw-cli` rewritten as a thin delivery crate | argument parsing, output rendering, and dispatch into app ports instead of command-module cross-calls |
| Control HTTP and UI delivery | `openrustclaw-gateway` | control routes, websocket or gateway delivery, UI serving, and HTTP transport over app ports |
| MCP delivery | `openrustclaw-mcp` | MCP server and client transport, tool registry, and tool invocation over app ports |
| Runtime host delivery | new `openrustclaw-runtime-host` crate or equivalent binary layer | scheduler startup, runtime workers, mobile workers, voice workers, orchestration workers, and service boot |
| Infrastructure adapters | new `openrustclaw-infra` crate or equivalent extracted module family | filesystem, sqlite, repositories, registries, audit logs, service-manager hooks, and external integration adapters |
| Temporary compatibility shims | bounded forwarding modules in `openrustclaw-cli` only where needed | forward deprecated entrypoints to native delivery layers during transition, without regaining business logic ownership |

### Bootstrap Contract

The native delivery layer should converge on these entrypoints:

- `openrustclaw` binary: parse CLI input, select native CLI delivery module, call app ports, render output
- `openrustclaw-gateway` or equivalent gateway startup path: own control HTTP, UI, websocket, and webhook delivery
- `openrustclaw-mcp-server` or equivalent MCP startup path: own MCP transport, tool registry, and tool invocation delivery
- `openrustclaw-runtime-host` or equivalent worker host: own scheduler, runtime maintenance, mobile, voice, and orchestration worker startup
- legacy subcommands may forward temporarily, but they should never remain the primary bootstrap path once their native replacement ships

### Why This Topology

- the workspace already has `openrustclaw-gateway` and `openrustclaw-mcp`, so the roadmap can reuse existing crates instead of forcing all delivery through `openrustclaw-cli`
- `openrustclaw-app` already owns the business-rule lane, so the missing piece is delivery and infrastructure, not another orchestration crate
- worker startup and repository wiring are different concerns from CLI parsing and should not be hidden inside the same legacy command tree

## Legacy Shutdown Rules

### Inventory Status Model

| Status | Meaning | Allowed State |
| --- | --- | --- |
| `legacy` | the product still depends on the legacy delivery surface | no deletion allowed |
| `native-shimmed` | a native delivery path exists and the legacy entrypoint only forwards | deletion may be planned next |
| `retired` | the native delivery path is primary and the legacy surface is deleted, archived, or isolated outside the main product path | no new work may land in the retired surface |

### Deletion Gates

A legacy delivery family cannot be marked `retired` until all of the following are true:

1. the replacement app port family exists and is documented
2. a native delivery entrypoint exists for the shipped flow
3. required repositories and infrastructure adapters exist for that flow
4. verification covers the app port, the native delivery path, and any temporary forwarding shim
5. contributor guidance and CI or guardrails point new work at the native delivery path instead of the legacy one

### Compatibility Rules

- temporary forwarding shims are allowed only after a native delivery path exists
- new features may not land first in shims
- shims must declare which native delivery path replaces them
- if a legacy file cannot be retired yet, the blocker must be documented as a missing port, adapter, or delivery host rather than buried as “future cleanup”

## Measurement Model

| Dimension | Target State | Failure Signal |
| --- | --- | --- |
| Port coverage | app ports exist for each major delivery family | new delivery work still reaches into command modules for core orchestration |
| Delivery ownership | CLI, control, MCP, and worker entrypoints use native delivery layers | `main.rs` or `commands/*` remains the main runtime entrypoint path |
| Infrastructure isolation | filesystem, sqlite, registries, service-manager, and channel integrations live behind adapters | app services or new delivery layers depend on command-local helper code |
| Legacy retirement | command modules are deleted, shimmed, or bypassed in hot paths | shipped flows still require the legacy command tree as the primary route |
| Guardrails | CI or tests fail when retired delivery ownership returns | legacy reintroduction depends on human review memory alone |

## Program Milestones

### v1.25 Native Delivery Layer: Port Contracts and Legacy Inventory

Status after shipment: complete. This milestone defined the remaining legacy delivery inventory, the app-port catalog, the successor native topology, and the shutdown rules for retiring the legacy command tree.

### v1.26 Native Delivery Layer: Control, MCP, and Gateway Delivery

Primary target: replace the `start.rs` control and MCP bootstrap monopoly with native delivery surfaces.

- native control HTTP delivery layer over app ports
- native MCP server delivery layer over app ports
- gateway or server bootstrap split out of the legacy command tree
- control UI serving and wiring contract aligned to the new delivery layer

### v1.27 Native Delivery Layer: CLI Core Dispatch and Operator Commands I

Primary target: break the top-level binary and core operator flows away from `main.rs` plus the legacy command tree.

- new CLI dispatch layer over app ports
- native delivery for assistant, chat, session, inspect, control, and runtime entrypoints
- parsing, rendering, and app invocation boundaries separated explicitly
- temporary compatibility shim plan for any still-live legacy command paths

### v1.28 Native Delivery Layer: CLI Operator Commands II and UI-Adjacent Flows

Primary target: finish the large operator-facing command families that still sit inside the legacy command tree.

- native delivery for browser, orchestration, mobile, voice runtime, onboarding, skills, and self-hosted operator flows
- native delivery for channels, services, schedule, tools, media, and memory operator flows
- removal of command-to-command orchestration dependencies between those families
- UI-adjacent delivery alignment so operator surfaces point to native entrypoints first

### v1.29 Native Delivery Layer: Runtime Hosts and Background Workers

Primary target: move worker startup and runtime-host entrypoints off the legacy command layer.

- dedicated runtime-host and background-worker entrypoints over app ports
- service-manager, probe-runner, runtime-maintenance, and scheduler startup boundaries
- mobile, voice, and orchestration worker boot contracts aligned with native delivery
- removal of legacy command ownership for worker lifecycle startup

### v1.30 Native Delivery Layer: Repositories and Integration Adapters

Primary target: remove command-module knowledge of persistence layout and external integration wiring.

- repository or gateway adapters for sqlite, workspace files, audit logs, runtime config, compiled-skill cache, and registries
- infrastructure boundaries for channel providers and external services
- app services depend on adapter traits or repositories instead of command-local file helpers
- direct repository and adapter tests replace command-local persistence verification

### v1.31 Native Delivery Layer: Legacy Module Retirement and Compatibility Shutdown

Primary target: delete, freeze, or isolate the superseded legacy command tree.

- remove legacy command modules from the main product path
- replace any remaining live legacy surfaces with thin compatibility shims or hard deletes
- shrink `main.rs` to binary bootstrap only or replace it entirely
- CI and guardrails prove new product entrypoints no longer depend on retired delivery files

### v1.32 Native Delivery Layer: Native Product Exit Audit and Packaging

Primary target: close the native-delivery program with a truthful product-level exit claim.

- final scorecard proving the main product entrypoints are native delivery surfaces
- docs, packaging, and contributor guidance updated to the native architecture
- final audit of any remaining compatibility shims or exceptions
- milestone archive states explicitly whether the repo can now claim clean native or greenfield delivery ownership

## Sequence Rationale

The order is deliberate:

1. define the full inventory and port map first so future deletion work does not invent scope mid-flight
2. replace control and MCP delivery before broader CLI retirement because `start.rs` remains the central bootstrap hotspot
3. replace top-level CLI dispatch before deleting command modules so product entrypoints stay stable during migration
4. move worker bootstraps and repositories after delivery paths exist so infrastructure contracts serve real native entrypoints
5. delete or isolate legacy modules only after native delivery paths and repositories are already proven
6. close with an explicit exit audit so the native-product claim is evidence-backed

## Exit Criteria

OpenRustClaw should only claim complete legacy exit when all of the following are true:

- the main CLI binary, control HTTP surface, MCP server, and runtime-worker entrypoints use native delivery layers instead of the legacy command tree
- `crates/cli/src/commands/*` is deleted, archived, or reduced to clearly temporary shims outside the main product path
- app ports define the dominant product entry contracts
- infrastructure adapters own persistence and integration details that used to live in command helpers
- CI, tests, and contributor guidance block the reintroduction of legacy delivery ownership
- a canonical audit says the native-delivery claim is warranted without hand-waving over remaining exceptions

## Companion Documents

- `.planning/codebase/GREENFIELD.md` — original greenfield transition contract and containment rules
- `.planning/codebase/GREENFIELD-INVENTORY.md` — retired historical `18/18` seam ledger from `v1.13` through `v1.18`
- `.planning/codebase/GREENFIELD-FULL-CONVERSION.md` — completed adapter-only roadmap from `v1.19` through `v1.24`
- `.planning/ROADMAP.md` — active milestone phases
- `.planning/PROJECT.md` — project-level milestone context and decisions

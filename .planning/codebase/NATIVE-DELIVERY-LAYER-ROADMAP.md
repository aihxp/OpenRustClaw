# Native Delivery Layer Roadmap

**Created:** 2026-03-28
**Purpose:** Canonical follow-on roadmap for replacing the remaining legacy delivery layer with native delivery surfaces built directly around `openrustclaw-app` ports and explicit infrastructure adapters.
**Status:** Active after `v1.30` at `6/8` shipped milestones, or `75%`
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

## v1.26 Outcome

`v1.26` did not implement the native delivery layers yet. It made the first execution slice concrete enough to build without rediscovering boundaries:

- defined the target native control HTTP delivery ownership around `openrustclaw-gateway` plus `ControlPlanePort`
- defined the target native MCP delivery ownership around `openrustclaw-mcp` plus `McpServerPort`
- split the future gateway bootstrap contract away from the `start.rs` hotspot so native startup can become the primary path
- aligned Control UI serving and wiring to the native gateway-delivery contract instead of leaving the UI tied to the legacy bootstrap monopoly

## v1.27 Outcome

`v1.27` did not replace the CLI in code yet. It made the first CLI-retirement slice concrete enough to implement without rediscovering delivery boundaries:

- defined the target native CLI dispatch ownership that will replace `main.rs` as the permanent routing owner
- defined the first core operator CLI delivery family for assistant, chat, session, and inspect entrypoints over app ports
- defined the native CLI delivery ownership for control and runtime entrypoints instead of leaving those flows inside legacy command hubs
- separated parsing, rendering, and app invocation responsibilities explicitly, with a bounded compatibility shim plan for any still-live legacy paths

## v1.28 Outcome

`v1.28` did not replace the second CLI slice in code yet. It made the remaining operator-family CLI work concrete enough to implement without rediscovering ownership:

- defined the native delivery ownership for the remaining large operator command families over app ports
- defined the native delivery ownership for the secondary operator and utility command families over app ports
- defined how the remaining CLI families stop depending on legacy command-to-command orchestration
- aligned UI-adjacent operator surfaces to native entrypoints so the second CLI slice has an end-to-end delivery story

## v1.29 Outcome

`v1.29` did not implement the runtime hosts in code yet. It made the runtime-host replacement slice concrete enough to build without rediscovering startup ownership:

- defined dedicated runtime-host and background-worker entrypoints over app ports instead of preserving legacy command bootstraps as the implicit long-term owner
- defined the startup boundaries for service-manager, probes, runtime maintenance, and scheduler flows needed by the native runtime-host path
- aligned mobile, voice, and orchestration worker boot contracts to native runtime-host delivery instead of command-local startup ownership
- made legacy command ownership over worker lifecycle startup explicitly temporary, bounded, and removable in future implementation milestones

## v1.30 Outcome

`v1.30` did not implement the repository and integration adapters in code yet. It made the adapter-replacement slice concrete enough to build without rediscovering persistence and side-effect ownership:

- defined the repository and gateway adapter inventory for sqlite, workspace files, audit logs, runtime config, compiled-skill cache, and registries instead of preserving command-local persistence ownership as the long-term default
- defined the infrastructure gateway boundaries for channel providers and external services so side-effect wiring is no longer described as command-local helper work
- aligned app-port ownership with repository and gateway contracts instead of leaving app services conceptually dependent on command-file filesystem or registry helpers
- made adapter verification and ownership-exit rules explicit so the roadmap can measure when command modules stop owning persistence and integration behavior

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

### v1.26 Control HTTP Delivery Contract

The native control HTTP delivery path should ship through `openrustclaw-gateway` instead of adding more route ownership to `crates/cli/src/commands/start.rs`.

| Concern | Native Owner | Notes |
| --- | --- | --- |
| route registration | `openrustclaw-gateway` | the route table should be assembled in gateway-native modules instead of in `start.rs` |
| request parsing and HTTP shaping | `openrustclaw-gateway` | transport concerns stay in the gateway layer, preserving current HTTP compatibility where needed |
| business-use orchestration | `ControlPlanePort` in `openrustclaw-app` | route handlers should call app ports directly rather than command-module helpers |
| stateful persistence and external side effects | infrastructure or repository ports | filesystem, sqlite, registries, and runtime interactions should arrive through adapter traits instead of gateway-local helpers |

The first retirement slice is to move future control-route additions and selected migrated families into gateway-native route modules while leaving temporary forwarding shims in `start.rs` only where startup compatibility still requires them.

### v1.26 MCP Delivery Contract

The native MCP server path should converge on `openrustclaw-mcp` as the transport owner, with `openrustclaw-app` defining the tool and invocation contract.

| Concern | Native Owner | Notes |
| --- | --- | --- |
| MCP server bootstrap | `openrustclaw-mcp` | MCP startup should no longer require `start.rs` once the native bootstrap lands |
| tool catalog and capability exposure | `McpServerPort` in `openrustclaw-app` | compiled-skill exposure, tool metadata, and capability selection should flow through named app ports |
| tool invocation and result shaping | `McpServerPort` plus downstream app ports | invocation should route through app-layer contracts instead of command-local tool helpers |
| bridge or compatibility forwarding | bounded shim in `openrustclaw-cli` or gateway bootstrap only when required | shims are allowed only after the native MCP path exists and must point back to `openrustclaw-mcp` explicitly |

This keeps MCP transport and control HTTP delivery as separate native ownership lanes even when both used to originate in the same `start.rs` hotspot.

### v1.26 Gateway Bootstrap Split

The gateway bootstrap split is the first concrete retirement slice for `start.rs`.

| Bootstrap Responsibility | Current Owner | Native Target |
| --- | --- | --- |
| control HTTP startup | `crates/cli/src/commands/start.rs` | `openrustclaw-gateway` native bootstrap |
| websocket, webhook, and related gateway startup | `crates/cli/src/commands/start.rs` plus adjacent legacy helpers | `openrustclaw-gateway` native bootstrap modules |
| MCP startup coupling | `crates/cli/src/commands/start.rs` | `openrustclaw-mcp` startup entrypoint |
| temporary compatibility forwarding | legacy bootstrap shell | thin forwarding layer only while native startup is rolled out |

The retirement slice is truthful only if startup ownership changes before route deletion. `start.rs` can remain as a bounded compatibility shell temporarily, but it should stop being the place where the main gateway lifecycle is assembled.

### v1.26 Control UI Alignment

Control UI transport should align with the gateway-native delivery path instead of depending on the legacy `start.rs` bootstrap contract.

| UI Concern | Native Owner | Compatibility Rule |
| --- | --- | --- |
| static asset and HTML serving | `openrustclaw-gateway` | preserve the shipped UI route contract while moving serving ownership into gateway-native modules |
| UI-to-control API wiring | `openrustclaw-gateway` plus `ControlPlanePort` | UI actions should target the same gateway-native control routes that the native delivery layer owns |
| websocket or live-event wiring | `openrustclaw-gateway` | avoid reintroducing route ownership into `start.rs` as live UI surfaces evolve |
| temporary legacy entrypoints | bounded forwarding shell only | UI compatibility shims must document the gateway-native replacement path |

This keeps the UI story honest: the UI is not “off legacy” merely because app logic is greenfielded; it is only off legacy when the UI-serving and route-ownership path is also native.

### v1.27 CLI Dispatch Contract

The native CLI dispatch path should make `openrustclaw` a thin bootstrap over app-facing delivery modules instead of a permanent `main.rs` routing monopoly.

| Concern | Native Owner | Notes |
| --- | --- | --- |
| top-level argument routing | `openrustclaw-cli` native dispatch layer | `main.rs` should narrow toward binary bootstrap while route selection moves into delivery modules |
| command-family selection | native CLI delivery modules over app ports | dispatch should choose a delivery module by capability family instead of command-to-command cross-calls |
| app invocation | app ports in `openrustclaw-app` | delivery modules call app contracts directly rather than reusing legacy command hubs as orchestration layers |
| temporary entrypoint forwarding | bounded compatibility shims only where needed | shims are allowed only after native dispatch exists and must point to the replacement module explicitly |

This keeps the binary stable while changing ownership: the `openrustclaw` executable may remain, but `main.rs` should stop being where the product path is assembled.

### v1.27 Operator Delivery Family I

The first core operator CLI family to move behind native delivery modules is assistant, chat, session, and inspect.

| Flow Family | Port Family | Native Delivery Target |
| --- | --- | --- |
| assistant and chat requests | `AssistantConversationPort` | native CLI assistant and chat delivery module |
| session lifecycle operations | `SessionManagementPort` | native CLI session delivery module |
| inspection and continuity summaries | `InspectionPort` | native CLI inspect delivery module |

The reason to group these first is that they are core operator entrypoints with strong app-port definitions already available, and they force the roadmap to separate parsing and rendering from app-use orchestration before broader command-family retirement begins.

### v1.27 Control and Runtime CLI Delivery

The next CLI slice in this milestone is control and runtime command delivery, because those entrypoints still default to legacy command hubs even after the control HTTP and runtime app seams exist.

| Flow Family | Port Family | Native Delivery Target |
| --- | --- | --- |
| control CLI entrypoints | `ControlPlanePort` | native CLI control delivery module |
| runtime CLI entrypoints | `RuntimeOperationsPort` | native CLI runtime delivery module |
| compatibility forwarding | bounded shims in `openrustclaw-cli` only | any legacy forwarding must declare the native module it points to and must not regain orchestration ownership |

This keeps the CLI roadmap honest: HTTP control routes alone do not retire legacy delivery if the operator CLI still defaults to command-local orchestration.

### v1.27 CLI Boundary Separation

The CLI replacement slice is only implementable if parsing, rendering, and app invocation are treated as separate responsibilities.

| Responsibility | Native Owner | Legacy-Retirement Rule |
| --- | --- | --- |
| argument parsing | native CLI delivery modules | parsing must not imply orchestration ownership |
| output rendering | native CLI delivery modules | rendering stays at the edge and should not pull domain decisions back into CLI helpers |
| app invocation | app ports in `openrustclaw-app` | use-case orchestration lives behind ports, not inside render or parse helpers |
| compatibility shims | bounded forwarding modules only | shims may translate old entrypoints, but they may not become the long-term home for new behavior |

This boundary split gives future milestones a defensible deletion path for `main.rs` and the command tree instead of another round of mixed-responsibility helpers.

### v1.28 Large Operator Family Delivery

The second CLI slice starts with the remaining large operator command families that still dominate the legacy delivery tree.

| Flow Family | Port Family | Native Delivery Target |
| --- | --- | --- |
| browser operator flows | `BrowserAutomationPort` | native CLI browser delivery module |
| orchestration operator flows | `OrchestrationPort` | native CLI orchestration delivery module |
| mobile operator flows | `MobileOperationsPort` | native CLI mobile delivery module |
| voice runtime flows | `VoiceRuntimePort` | native CLI voice-runtime delivery module |
| onboarding and repair flows | `SetupLifecyclePort` | native CLI onboarding delivery module |
| skills and self-hosted operator flows | `SkillLifecyclePort` plus adjacent control or inspection ports | native CLI skill or self-hosted delivery modules |

These families stay grouped because they remain the largest operator-facing hotspots after the first CLI slice and require explicit app-port ownership before any truthful retirement work can continue.

### v1.28 Secondary Operator and Utility Delivery

The milestone also defines the remaining secondary utility and operator families so the CLI replacement story covers more than the largest command hubs.

| Flow Family | Port Family | Native Delivery Target |
| --- | --- | --- |
| channels and schedules | `ChannelOperationsPort` | native CLI channel and schedule delivery modules |
| services and control-adjacent utilities | `ControlPlanePort` plus runtime or service-facing ports | native CLI services and control utility modules |
| tools, media, and memory flows | `ArtifactMediaPort` plus `MemoryOperationsPort` | native CLI tools, media, and memory delivery modules |

This keeps the second CLI slice truthful: the secondary command families are part of the product surface and cannot be left as unowned cleanup behind the larger operator modules.

### v1.28 CLI Dependency Removal

The remaining CLI families should no longer rely on command-to-command orchestration once native delivery modules exist.

| Dependency Problem | Native Rule |
| --- | --- |
| one command module reuses another command module for orchestration | native modules call app ports directly instead |
| shared helper ownership drifts back into legacy files | shared concerns move to app ports or explicit delivery helpers, not cross-file command calls |
| compatibility shims silently become permanent routing layers | shims must point to native delivery modules and stay bounded to forwarding or translation only |

This makes future implementation safer because module relationships are defined by ports and delivery concerns, not by inherited file topology from the legacy tree.

### v1.28 UI-Adjacent Delivery Alignment

UI-adjacent operator surfaces should align with the native entrypoints defined across the first and second CLI slices.

| Surface | Native Alignment |
| --- | --- |
| operator-facing UI actions that mirror CLI capabilities | point to native gateway or native CLI ownership paths instead of legacy command hubs |
| shared summaries and entry contracts | derive from the same app-port-backed delivery modules that native CLI and gateway surfaces use |
| temporary UI-facing compatibility bridges | allowed only as bounded forwarding layers that name the native replacement explicitly |

This avoids a split-brain architecture where the CLI moves toward native delivery while UI-adjacent operator surfaces still depend conceptually on legacy command ownership.

### v1.29 Runtime Host Entry Points

The native runtime-host path should become the permanent owner for worker startup and background lifecycle delivery instead of continuing to route those concerns through legacy command entrypoints.

| Runtime Concern | Native Owner | Notes |
| --- | --- | --- |
| runtime-host bootstrap | `openrustclaw-runtime-host` or equivalent runtime-host binary layer | this entrypoint should own long-lived runtime lifecycle startup instead of `start.rs` or `runtime.rs` |
| background-worker bootstrap | `openrustclaw-runtime-host` worker subcommands or dedicated worker binaries | worker startup should no longer require command-family routing as the permanent bootstrap path |
| app orchestration | `RuntimeOperationsPort`, `MobileOperationsPort`, `VoiceRuntimePort`, and `OrchestrationPort` in `openrustclaw-app` | runtime-host entrypoints should call app ports directly instead of composing through command-local startup helpers |
| temporary compatibility forwarding | bounded compatibility shims only | any remaining legacy startup path must point explicitly to the native runtime-host entrypoint it forwards to |

This keeps the runtime-host slice honest: background execution is only off legacy when the worker entrypoints themselves stop being assembled in the command tree.

### v1.29 Runtime Startup Boundaries

The runtime-host implementation slice needs explicit startup contracts for the background systems that are still assembled implicitly in legacy command helpers.

| Startup Boundary | Native Owner | Notes |
| --- | --- | --- |
| service-manager lifecycle | `ServiceManagerGateway` behind native runtime-host delivery | service boot, restart, health, and shutdown hooks should be adapter-backed rather than embedded in command helpers |
| probe runner and readiness checks | `SetupLifecyclePort` plus infrastructure probe adapters | runtime health and readiness startup should be callable from the runtime host without reaching back into CLI command ownership |
| runtime-maintenance and reload lifecycle | `RuntimeOperationsPort` plus `RuntimeRepository` | maintenance windows, reload work, and recovery paths should be orchestrated from app ports over explicit adapters |
| scheduler startup and durable jobs | `SchedulerGateway` plus app scheduling ports | background job registration and run ownership should belong to native runtime-host startup rather than `schedule.rs` or `services.rs` |

These startup boundaries make the runtime-host milestone implementable because they describe which concerns remain delivery-level wiring and which are app-port contracts.

### v1.29 Worker Boot Alignment

Mobile, voice, and orchestration workers should align to the same runtime-host delivery model instead of each retaining separate legacy command startup assumptions.

| Worker Family | App Port | Native Delivery Target | Compatibility Rule |
| --- | --- | --- | --- |
| mobile worker startup | `MobileOperationsPort` | runtime-host worker module for mobile notification, dispatch, heartbeat, and sync flows | temporary shims may forward from `mobile.rs`, but they must name the runtime-host target explicitly |
| voice worker startup | `VoiceRuntimePort` | runtime-host worker module for voice provider, session, transcript, artifact, and outcome flows | legacy voice startup can remain only as a forwarding shell during rollout |
| orchestration worker startup | `OrchestrationPort` | runtime-host worker module for checkpoints, reflections, interventions, and supervision flows | orchestration boot may not remain permanently inside CLI command helpers once the runtime-host path exists |

This alignment keeps the worker story end to end instead of treating each worker family as a separate startup exception.

### v1.29 Legacy Startup Ownership Removal

The runtime-host slice should reduce legacy startup ownership by changing what the command tree is allowed to do once native worker entrypoints exist.

| Legacy Surface | Allowed Transitional State | Removal Rule |
| --- | --- | --- |
| `start.rs` startup helpers | bounded compatibility shell only | route or startup assembly must move to native runtime-host modules before any claim of runtime legacy exit |
| `runtime.rs`, `services.rs`, and `schedule.rs` startup helpers | translation or forwarding only | these files may invoke native startup contracts temporarily, but they must stop owning lifecycle composition |
| `mobile.rs` and `voice_runtime.rs` worker startup paths | compatibility forwarding only | worker-family commands may remain as operator entrypoints temporarily, but native worker boot must be the real owner |
| new worker behavior | native runtime-host path only | no new lifecycle ownership may land in legacy startup helpers after this milestone |

This makes the milestone measurable: the runtime-host roadmap only advances to `5/8` when the legacy command tree is explicitly downgraded from owner to bounded compatibility surface for worker startup.

### v1.30 Repository Adapter Inventory

The repository-adapter slice should make persistence ownership explicit enough that command modules stop being the default place where filesystem layout and sqlite-backed state are assembled.

| Persistence Concern | Native Owner | Notes |
| --- | --- | --- |
| workspace layout and control files | `WorkspaceRepository` behind infrastructure adapters | workspace paths, control files, artifact roots, and compiled-skill cache paths should move behind named repository contracts instead of command-file path helpers |
| runtime config and maintenance state | `RuntimeRepository` | runtime config, vault, reload state, maintenance artifacts, and runtime status persistence should no longer be assembled directly in command modules |
| skill registry and compiled artifact state | `SkillRepository` | registries, compiled artifacts, bindings, plugin metadata, and compile receipts should be owned by adapter-backed repositories |
| session, audit, and inspection persistence | `SessionRepository` plus `AuditLogRepository` | session history and audit evidence should stop depending on command-local file or sqlite wiring |
| channel and memory persistence | `ChannelRepository` plus `MemoryRepository` | bindings, schedules, health state, memory store, archive state, and derived views should be repository-backed rather than command-local |

This inventory makes the persistence slice implementable because it names which repositories replace the remaining command-owned filesystem and sqlite helpers.

### v1.30 Integration Gateway Boundaries

The integration-adapter slice should make external side-effect ownership explicit instead of leaving provider wiring in command-local helper clusters.

| Integration Concern | Native Owner | Notes |
| --- | --- | --- |
| channel-provider APIs | `ExternalServiceGateway` plus `CommunicationChannelPort` | Gmail, Matrix, Signal, WhatsApp, webhooks, and related providers should move behind gateway-backed contracts instead of command-local provider wiring |
| scheduler and background integration hooks | `SchedulerGateway` | durable scheduling, next-run computation, and workflow startup should be injected through gateways rather than command modules |
| service-manager hooks | `ServiceManagerGateway` | runtime process interactions should stay behind explicit gateways once repository-backed runtime services land |
| registry and external metadata sources | repository or gateway adapters as appropriate | registry refresh, remote metadata access, and provider capability lookups should no longer be hidden inside command-local helpers |

These gateway boundaries keep integration work distinct from app orchestration and make later side-effect replacement measurable.

### v1.30 App Port to Repository Contract Alignment

The repository-adapter slice should align app services to repositories and gateways directly so delivery layers stop depending on command-owned persistence helpers.

| App Port Family | Repository or Gateway Contract | Alignment Rule |
| --- | --- | --- |
| `RuntimeOperationsPort` | `RuntimeRepository`, `ServiceManagerGateway`, `SchedulerGateway` | runtime services should read and mutate state through repositories and gateways instead of `runtime.rs`, `services.rs`, or `schedule.rs` helper ownership |
| `SkillLifecyclePort` | `SkillRepository`, `WorkspaceRepository`, `ExternalServiceGateway` where needed | compiled-skill and registry flows should stop depending on `skills.rs` filesystem and registry helper orchestration |
| `InspectionPort`, `SessionManagementPort`, and `ControlPlanePort` | `SessionRepository`, `AuditLogRepository`, `WorkspaceRepository` | inspection and control flows should consume durable state through repository contracts instead of command-local sqlite or file access |
| `ChannelOperationsPort`, `MemoryOperationsPort`, and adjacent ports | `ChannelRepository`, `MemoryRepository`, `ExternalServiceGateway` | channel, memory, and external integration flows should align to adapter-backed ownership rather than command-file coupling |

This keeps the adapter story honest: app services are only off legacy persistence ownership when they depend on repositories and gateways directly.

### v1.30 Adapter Verification and Ownership Exit Criteria

The repository-adapter slice should define how progress is verified and what the command tree is no longer allowed to own once adapter-backed paths exist.

| Verification Concern | Native Rule | Failure Signal |
| --- | --- | --- |
| repository-backed behavior tests | verification should target repositories, gateways, and app services directly | command-local persistence verification remains the main evidence source |
| adapter ownership boundaries | command modules may translate or forward, but they may not remain the place where sqlite, file layout, or registry rules are assembled | new persistence or integration behavior lands first in command helpers |
| compatibility shims | allowed only after repository-backed and gateway-backed paths exist | legacy files stay primary because the replacement adapter path is not explicit |
| roadmap advancement | the milestone only advances to `6/8` when adapter ownership and verification are explicit end to end | progress is claimed from inventory alone without verification or exit rules |

This makes the milestone measurable: the repository-adapter roadmap only advances to `6/8` when command-module persistence and integration ownership is explicitly downgraded from primary owner to bounded compatibility surface.

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

Status after shipment: complete. This milestone defined the native control HTTP ownership path in `openrustclaw-gateway`, the native MCP ownership path in `openrustclaw-mcp`, the first truthful gateway-bootstrap split away from `start.rs`, and the Control UI alignment rules needed for the next implementation milestones.

- native control HTTP delivery layer over app ports
- native MCP server delivery layer over app ports
- gateway or server bootstrap split out of the legacy command tree
- control UI serving and wiring contract aligned to the new delivery layer

### v1.27 Native Delivery Layer: CLI Core Dispatch and Operator Commands I

Status after shipment: complete. This milestone defined the native CLI dispatch path that will replace `main.rs` as the routing owner, the first core operator CLI delivery family over app ports, the native CLI ownership for control and runtime entrypoints, and the boundary plus compatibility-shim rules needed for the first CLI implementation slice.

- new CLI dispatch layer over app ports
- native delivery for assistant, chat, session, inspect, control, and runtime entrypoints
- parsing, rendering, and app invocation boundaries separated explicitly
- temporary compatibility shim plan for any still-live legacy command paths

### v1.28 Native Delivery Layer: CLI Operator Commands II and UI-Adjacent Flows

Status after shipment: complete. This milestone defined the remaining large operator CLI families, the secondary operator and utility families, the CLI dependency-removal rules, and the UI-adjacent alignment needed for the second CLI implementation slice.

- native delivery for browser, orchestration, mobile, voice runtime, onboarding, skills, and self-hosted operator flows
- native delivery for channels, services, schedule, tools, media, and memory operator flows
- removal of command-to-command orchestration dependencies between those families
- UI-adjacent delivery alignment so operator surfaces point to native entrypoints first

### v1.29 Native Delivery Layer: Runtime Hosts and Background Workers

Status after shipment: complete. This milestone defined the dedicated runtime-host and background-worker entrypoints, the explicit startup-boundary contracts, the aligned worker boot model, and the bounded removal rules needed to move worker lifecycle startup off the legacy command layer in the next implementation slices.

- dedicated runtime-host and background-worker entrypoints over app ports
- service-manager, probe-runner, runtime-maintenance, and scheduler startup boundaries
- mobile, voice, and orchestration worker boot contracts aligned with native delivery
- bounded removal of legacy command ownership for worker lifecycle startup

### v1.30 Native Delivery Layer: Repositories and Integration Adapters

Status after shipment: complete. This milestone defined the repository and gateway adapter inventory, the integration gateway boundaries, the app-port-to-repository contract alignment, and the adapter verification or ownership-exit rules needed to move persistence and side-effect ownership off the legacy command layer in the next implementation slices.

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

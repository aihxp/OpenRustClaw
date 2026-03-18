# OpenRustClaw Roadmap

This roadmap replaces the old re-engineering backlog.

The target is no longer just "make the current shipped surface honest." The target is:

- reach feature parity with the currently documented OpenClaw surface where parity makes product sense,
- keep OpenRustClaw's unique strengths where they are better,
- implement the parity surface in Rust-first infrastructure,
- retire non-Rust runtime requirements from the critical path over time.

This is a feature-parity roadmap, not a source-port roadmap. We do not need JavaScript, TypeScript, or Python implementation parity. We need behavior parity, operational parity, and operator-facing parity.

Checked against official OpenClaw sources on 2026-03-17:

- [OpenClaw docs home](https://docs.openclaw.ai/)
- [OpenClaw features](https://docs.openclaw.ai/concepts/features)
- [OpenClaw CLI reference](https://docs.openclaw.ai/cli)
- [OpenClaw multi-agent routing](https://docs.openclaw.ai/concepts/multi-agent)
- [OpenClaw streaming and chunking](https://docs.openclaw.ai/concepts/streaming)
- [OpenClaw plugin agent tools](https://docs.openclaw.ai/plugins/agent-tools)
- [OpenClaw Deepgram provider](https://docs.openclaw.ai/providers/deepgram)
- [OpenClaw repository](https://github.com/openclaw/openclaw)
- [OpenClaw site](https://openclaw.ai/)

## North Star

OpenRustClaw is done when all of the following are true:

- OpenRustClaw can match the documented OpenClaw gateway feature set across channels, routing, sessions, memory, media, tools, plugins, nodes, and operator surfaces, except where OpenRustClaw intentionally keeps a stronger Rust-native implementation.
- Every shipped feature is implemented end to end in Rust-owned runtime paths.
- Python sidecar logic is either retired or reduced to optional compatibility tooling, not required production runtime.
- Docs, feature matrix, CLI help, health endpoints, and runtime behavior all match.
- The remaining non-parity items are explicit, justified, and tracked as intentional divergences.

## Planning Rules

- `[x]` means complete enough to rely on.
- `[ ]` means still open.
- No feature counts as done unless startup, runtime behavior, persistence, tests, and docs exist.
- No product path may return fake success.
- No phase closes until its parity tasks are either complete or explicitly deferred with a reason.
- OpenClaw parity is measured against documented user-facing behavior, not internal implementation language or framework choices.

## Workflow Execution Tiers

OpenRustClaw will use a three-tier execution model rather than a single permanent orchestration backend.

### Tier A: Rust-native production path

Use for:

- channel message handling,
- scheduler jobs and event-triggered work,
- memory maintenance,
- RAG retrieval and context assembly,
- approvals and resumable operator flows,
- anything durability-critical, restart-sensitive, or part of the shipped surface.

Rules:

- Rust owns persistence, checkpoints, leases, retries, trace correlation, and operator inspection.
- This is the default target for production parity work.
- A feature is not considered fully landed if the durable truth still lives outside Rust.

### Tier B: Rust runtime with temporary sidecar compatibility

Use for:

- shipped workflows still being migrated,
- legacy workflow definitions,
- complex paths that are real but not yet ported,
- compatibility during staged cutovers.

Rules:

- Rust still owns the outer loop, persistence, retries, and durability.
- The sidecar may execute a bounded workflow run, but it does not own the scheduler, forever loop, or source of truth.
- Every shipped compatibility workflow should have a planned Rust-port destination.

### Tier C: LangGraph rapid experimentation lane

Use for:

- prototyping new agent flows,
- evaluation of orchestration ideas,
- prompt/control-flow experiments before productization.

Rules:

- Experimental LangGraph flows do not count as shipped parity by themselves.
- No infinite graphs; each run should be bounded.
- If an experiment proves valuable, it should promote to Tier B first and then Tier A.

### Decision Tree

- If the workflow is production-critical, persistent, user-facing, or restart-sensitive: use Tier A.
- If the workflow is shipped but not yet ported to Rust: use Tier B.
- If the workflow is experimental: use Tier C.
- If a Tier C workflow proves useful: promote to Tier B, then port to Tier A.

### Architectural Consequence

- Rust owns the outer scheduler/event loop.
- Rust owns due-job discovery, leases, retries, dead letters, checkpoints, and operator visibility.
- LangGraph may remain as an authoring/prototyping or compatibility layer, but not as the sole durability boundary.

## Current Program Status

- [x] Feature matrix exists and the shipped surface is mostly honest.
- [x] Durable scheduler, MCP stdio, core memory, recall memory, archive maintenance, and tier-1 outbound channel paths are real.
- [x] Telegram inbound runtime, Slack HTTP ingress, and Discord interactions plus gateway message ingress exist.
- [x] Skills install and verification flow, marketplace sync, and a real WASM executor exist.
- [x] Rust-owned RAG storage, retrieval controls, and MCP inspection exist.
- [ ] Full OpenClaw parity across channels, Control UI, nodes, plugin ecosystem, media flows, session tools, and all operator workflows does not exist yet.
- [ ] The Python sidecar is still part of the production execution path.

## Parity Scope

The roadmap is explicitly scoped to the OpenClaw feature families currently documented:

- Multi-channel gateway
- Plugin channels and extension surface
- Multi-agent routing
- Sessions and session tools
- Memory and context
- Streaming and chunking
- Media support
- Web Control UI
- Mobile nodes
- CLI onboarding and channel account management
- Plugins, tools, and provider auth extensions
- Security and operator controls

OpenRustClaw-specific strengths to preserve while pursuing parity:

- Multi-provider LLM support
- Rust-native durability and resource control
- MCP and mcp2-cli integration
- Stronger capability enforcement around skills and tool execution
- Rust-native autonomous optimization infrastructure instead of depending on a Python self-improvement loop

## Phase 1: Product Contract and Parity Inventory

Goal: make the parity target explicit and freeze the product contract before more implementation churn.

Status: complete

Completed:

- [x] Create and maintain a shipped-surface feature matrix.
- [x] Align the current runtime, README, and docs around the actually shipped surface.
- [x] Gate non-shipping features rather than reporting false success.

Remaining:

- [x] Build a full OpenClaw parity matrix that maps each documented OpenClaw feature to:
  - current OpenRustClaw status,
  - owning crate/module,
  - tests,
  - docs page,
  - parity gap severity.
- [x] Split parity targets into:
  - core parity,
  - plugin parity,
  - intentional divergence,
  - out of scope.
- [x] Add a `docs/parity-matrix.md` artifact generated or maintained alongside [feature-matrix.md](feature-matrix.md).
- [x] Add parity labels to CI and release notes so "green" means "green for the declared shipped surface."
- [x] Add a single page that explains which OpenClaw features are matched, stronger in Rust, or intentionally different.

Exit criteria:

- [x] OpenRustClaw has a stable, source-backed parity inventory.
- [x] Every roadmap item can be traced to a documented OpenClaw feature or an explicit OpenRustClaw divergence.

## Phase 2: Rust Runtime Contract and Sidecar Retirement

Goal: preserve current behavior while making the three-tier execution model explicit, with Rust owning the production-critical path.

Completed:

- [x] Typed Rust-to-sidecar workflow metadata contract exists.
- [x] Rust-owned loopback services exist for memory and RAG paths.
- [x] LangSmith trace ids can flow back through the runtime boundary.

Remaining:

- [ ] Build a unified workflow registry with explicit execution tier metadata:
  - `rust_native`,
  - `compat_sidecar`,
  - `experimental_langgraph`.
- [ ] Define routing policy for workflow dispatch based on:
  - durability requirements,
  - ship status,
  - operator visibility,
  - migration stage.
- [ ] Replace scheduler workflow execution with Rust-native workflow/state-machine implementations.
- [ ] Replace memory maintenance workflow execution with Rust-native workflow/state-machine implementations.
- [ ] Replace RAG orchestration with Rust-native retrieval and context-assembly services.
- [ ] Replace sidecar agent orchestration with Rust-native graph/state execution.
- [ ] Keep shipped sidecar-backed workflows bounded and compatibility-only.
- [ ] Ban sidecar-owned infinite loops and scheduler-owned durable truth outside Rust.
- [ ] Keep LangSmith via direct API integration from Rust, not Python framework dependency.
- [ ] Decide whether LangGraph remains:
  - an authoring format only,
  - an optional compatibility bridge,
  - or is removed entirely from production.
- [ ] Build a Rust-native workflow abstraction that covers:
  - node execution,
  - retries,
  - checkpointing,
  - human approval,
  - resumability,
  - trace correlation.
- [ ] Remove Python sidecar as a required production dependency from the default deployment path.
- [ ] Keep a compatibility harness only if needed for migration or legacy workflow imports.

Exit criteria:

- [ ] `openrustclaw start` can run the full shipped surface in Tier A without requiring Python.
- [ ] Tier B exists only for bounded compatibility workflows.
- [ ] Tier C is explicitly experimental and not confused with shipped parity.

## Phase 2.5: Rust-Native Autonomous Optimization Framework

Goal: build a Rust-native improvement loop inspired by `karpathy/autoresearch`, but generalized for OpenRustClaw and integrated with the execution-tier model.

This subsystem is not limited to model-training experiments. It should support bounded improvement loops across skills, prompts, RAG, policies, workflows, and selected code surfaces.

Principles:

- Rust owns experiment orchestration, persistence, evaluation history, and promotion policy.
- LLMs propose candidate changes; Rust acts as the lab, referee, and rollback owner.
- Experimental self-improvement does not count as shipped behavior until it passes the promotion pipeline.
- No unrestricted repo-wide self-modification.

Allowed optimization lanes:

- skills and skill instructions,
- prompt and response-format policies,
- RAG retrieval and context-assembly parameters,
- memory write policies,
- scheduler and routing heuristics,
- bounded workflow definitions,
- bounded code surfaces,
- original autoresearch-style training or tuning experiments where useful.

Required safety classes:

- `safe_config`: prompts, thresholds, retrieval parameters, formatting
- `bounded_workflow`: agent policies, memory policies, routing policies
- `bounded_code`: tightly scoped file/module optimization with mandatory tests
- `human_review_only`: security, auth, persistence, protocol-critical code

Status: complete

Completed:

- [x] Create a new Rust-owned optimization subsystem with a clear crate boundary.
- [x] Define optimization targets with typed metadata:
  - target id,
  - target kind,
  - execution tier,
  - allowed mutation surface,
  - required eval suite,
  - promotion policy.
- [x] Build a mutation-policy layer that enforces:
  - file allowlists,
  - field allowlists,
  - max diff size,
  - forbidden paths,
  - mandatory tests.
- [x] Build an experiment runner that:
  - creates a candidate,
  - applies changes in a temporary workspace,
  - runs bounded evals,
  - captures metrics,
  - stores artifacts and diffs.
- [x] Build an evaluator that scores:
  - task success,
  - regressions,
  - safety,
  - latency,
  - token cost,
  - tool correctness,
  - grounding quality.
- [x] Build a promotion engine that can:
  - reject,
  - keep as candidate,
  - promote to Tier C,
  - promote to Tier B,
  - queue for Tier A human-reviewed merge.
- [x] Build an experiment/audit history model with:
  - candidate diffs,
  - hypotheses,
  - eval metrics,
  - traces,
  - winner/loser decisions,
  - rollback references.
- [x] Add support for the first optimization targets:
  - skills,
  - RAG retrieval params,
  - context assembly,
  - prompt/tool policies.
- [x] Add a second wave of targets:
  - memory policies,
  - scheduler heuristics,
  - routing heuristics,
  - bounded workflow definitions.
- [x] Decide which bounded code surfaces are eligible for automated optimization and keep core security/auth/storage out of automatic promotion.
- [x] Add operator controls to inspect, approve, reject, and promote candidates.
- [x] Add MCP and CLI surfaces for experiment inspection and promotion control.
- [x] Add LangSmith/OpenTelemetry integration for experiment traces and eval lineage.
- [x] Support original autoresearch-style research loops as one target class, not the whole subsystem.

Shipped in this phase:

- [x] `openrustclaw-optimization` crate with typed targets, candidate change-sets, evaluation records, and promotion history.
- [x] SQLite schema for optimization targets, candidates, evaluations, and promotions.
- [x] Temp-workspace experiment runner with bounded file mutation and eval execution.
- [x] CLI surfaces for target registration, candidate submission, candidate execution, approval, rejection, and promotion.
- [x] MCP tools for target registration/listing, candidate submission/listing/inspection, candidate execution, and promotion control.
- [x] LangSmith-aware tracing for optimization candidate runs and evaluation lineage when enabled.

Exit criteria:

- [x] OpenRustClaw has a Rust-native autonomous optimization framework.
- [x] The framework can improve skills, RAG, and prompt/policy surfaces without uncontrolled self-modification.
- [x] Production-critical code remains protected by explicit promotion and review rules.

## Phase 3: Durable Scheduler and Eventing

Goal: achieve OpenClaw-style background workflow reliability, but with Rust-owned eventing and job execution.

Completed:

- [x] SQLite-backed due-job polling, leases, retries, and dead-letter handling exist.
- [x] Scheduler dispatch is durable and traced.
- [x] CLI schedule management exists.

Remaining:

- [ ] Implement a first-class internal event bus for:
  - message.received,
  - message.sent,
  - memory.stored,
  - reminder.triggered,
  - node.paired,
  - plugin events.
- [ ] Support event-triggered workflows in Rust, not just time-triggered jobs.
- [ ] Add reminder delivery policy controls across channels:
  - quiet hours,
  - fallback channel order,
  - retries per channel,
  - per-agent delivery rules.
- [ ] Add operator controls for:
  - pause job,
  - resume job,
  - replay job,
  - inspect attempts,
  - inspect dead letters.
- [ ] Add durable workflow checkpoints for long-running background tasks.
- [ ] Add idempotent event reprocessing and crash recovery tests at the workflow boundary.
- [ ] Add scheduler APIs for Control UI and MCP introspection parity.

Exit criteria:

- [ ] All scheduled and event-driven automations are Rust-owned, durable, restart-safe, and operator-visible.

## Phase 4: Memory, Sessions, Context, and RAG

Goal: match OpenClaw's memory and session ergonomics while keeping a stronger Rust-native persistence model.

Completed:

- [x] Rust-owned core memory, recall memory, archive maintenance, and RAG chunk storage exist.
- [x] Sidecar-era memory access paths already route through Rust-owned services.
- [x] RAG retrieval supports budgeting, source controls, filtering, and inspection.

Remaining:

- [ ] Implement session tools parity:
  - list sessions,
  - inspect history,
  - send into session,
  - spawn session,
  - archive/close session.
- [ ] Implement OpenClaw-style direct-vs-group session semantics as explicit configurable policy:
  - direct chats can collapse into shared `main`,
  - groups isolate session state,
  - thread scope can override channel scope.
- [ ] Add Markdown/QMD-style workspace memory files or a Rust-native equivalent with import/export parity.
- [ ] Add file-backed memory views that are operator-readable and editable from the Control UI.
- [ ] Add targeted memory lookup parity beyond broad search:
  - memory get,
  - namespace reads,
  - recent memory timeline,
  - archive inspection.
- [ ] Add memory write policies for:
  - user facts,
  - project facts,
  - agent facts,
  - session summaries.
- [ ] Add context compaction parity for long threads and high-volume group chats.
- [ ] Add stronger retrieval quality:
  - chunkers for code/docs/media transcripts,
  - embeddings or hybrid rankers where justified,
  - benchmark datasets and regression scoring.
- [ ] Add import/export and migration tools from OpenClaw-style memory/session data where feasible.

Exit criteria:

- [ ] Operators can manage sessions and memory with the same practical power as OpenClaw.
- [ ] Context assembly is deterministic, inspectable, and Rust-owned.

## Phase 5: Channels and Routing Parity

Goal: reach practical parity with OpenClaw's documented channel and routing surface.

Completed:

- [x] WebChat baseline exists.
- [x] Telegram outbound and polling ingress exist.
- [x] Slack outbound and HTTP Events ingress exist.
- [x] Discord outbound, interactions ingress, and gateway message ingress exist.

Core routing remaining:

- [ ] Implement full multi-agent binding parity:
  - per-channel binding,
  - per-account binding,
  - per-workspace binding,
  - fallback precedence rules.
- [ ] Implement pairing approval flows consistently across supported channels.
- [ ] Implement OpenClaw-style group mention activation rules.
- [ ] Implement richer account management parity from the CLI and Control UI.
- [ ] Implement reply/edit/thread/reaction semantics per channel where the platform supports them.
- [ ] Implement streaming and chunking parity per channel:
  - preview modes,
  - block streaming,
  - message coalescing,
  - pacing.
- [ ] Implement higher-fidelity channel UX parity where it materially affects operator experience:
  - Telegram forum topics, topic targeting, reactions, and polls,
  - Discord forwarded-attachment downloads and thread-parent binding inheritance,
  - Slack draft-stream replies, stream-mode controls, thread ownership, and attachment download actions.
- [ ] Implement media in/out parity per channel:
  - images,
  - audio,
  - documents,
  - captions,
  - file references.

Channel completion remaining:

- [ ] WhatsApp parity:
  - multi-account login,
  - pairing/QR flow,
  - direct/group routing,
  - mentions,
  - media,
  - replies,
  - reconnect behavior.
- [ ] iMessage parity:
  - local bridge integration,
  - send/receive,
  - attachment handling,
  - contact/group mapping.
- [ ] Mattermost parity via Rust-native plugin/channel implementation.
- [ ] Google Chat parity.
- [ ] Gmail inbound automation parity for mail-triggered workflows.
- [ ] Matrix parity.
- [ ] Microsoft Teams parity.
- [ ] Signal parity if kept in scope.
- [ ] Feishu/Lark parity if kept in scope:
  - docs/tables actions,
  - rich-text embedded media extraction.
- [ ] Additional plugin-channel parity where OpenClaw currently documents active support or plugin support.

Exit criteria:

- [ ] OpenRustClaw supports the same practical operator channel set targeted by OpenClaw, with Rust-owned runtime paths.

## Phase 6: Tools, MCP, CLI, and Control Surfaces

Goal: match OpenClaw's operator UX and tooling surface while keeping Rust-native interfaces.

Completed:

- [x] MCP stdio server is real.
- [x] MCP exposes memory, scheduling, and RAG inspection.
- [x] `mcp2-cli` exists as an operator/debug tool.

Remaining:

- [ ] Implement full session-tool parity exposed through MCP, CLI, and runtime APIs.
- [ ] Implement richer agent tool groups parity:
  - memory tools,
  - session tools,
  - filesystem tools,
  - browser/web tools,
  - media tools,
  - node tools.
- [ ] Build the Web Control UI with parity for:
  - live chat,
  - configuration,
  - sessions,
  - memory inspection,
  - scheduled jobs,
  - nodes,
  - channel accounts,
  - logs,
  - extensions/plugins,
  - secrets/service status.
- [ ] Build typed HTTP and WebSocket APIs that the Control UI and external clients share.
- [ ] Add onboarding parity:
  - guided setup,
  - daemon install,
  - provider selection,
  - first channel login/pairing,
  - remote access guidance.
- [ ] Add secure credential-vault parity:
  - encrypted storage for provider keys and channel tokens at rest,
  - CLI/UI secret management,
  - migration from plaintext legacy configs where feasible.
- [ ] Add config and personality hot-reload:
  - runtime config reload without restart,
  - editable persona/DNA/system prompt artifacts that reload cleanly,
  - provider/channel updates that do not require process restarts when safe.
- [ ] Add runtime provider and account switching without full restart where the active runtime can rebind safely.
- [ ] Add channel account CRUD parity to the CLI and Control UI.
- [ ] Add operator-grade diagnostics:
  - gateway health,
  - channel status,
  - auth status,
  - job status,
  - trace links,
  - config validation,
  - secrets/service state.
- [ ] Decide whether remote MCP transport belongs in the parity surface or remains an OpenRustClaw-specific deferred feature.

Exit criteria:

- [ ] An operator can configure, inspect, and drive the system from CLI, MCP, or Control UI without dropping into internal-only tools.

## Phase 7: Skills, Plugins, Media, Voice, and Nodes

Goal: reach OpenClaw's extension and device-command surface with Rust-native implementation choices.

Completed:

- [x] Skills install/update/remove/verify lifecycle exists.
- [x] Marketplace sync exists.
- [x] A real WASM executor with capability-aware enforcement exists.

Remaining:

- [ ] Define the long-term Rust-native extension model:
  - WASI component plugins,
  - manifest-driven capability declarations,
  - background services,
  - command hooks,
  - tool injection.
- [ ] Replace or supersede JavaScript/TypeScript plugin-host expectations with Rust/WASM plugin parity.
- [ ] Add plugin parity for:
  - agent tools,
  - auth plugins,
  - voice-call plugins,
  - channel extensions,
  - background workflows.
- [ ] Add media pipeline parity:
  - image receive/send,
  - audio receive/send,
  - document receive/send,
  - voice-note transcription,
  - transcript injection,
  - provider pluggability.
- [ ] Add richer provider support for media parity where OpenClaw documents active integrations.
- [ ] Add voice runtime parity:
  - wake/talk flows,
  - STT,
  - TTS,
  - voice notes,
  - call/phone plugin support if kept in scope,
  - call lifecycle health (stale-call reaping, reconnect, greeting/prewarm behavior) if call surfaces stay in scope.
- [ ] Add node pairing/runtime parity:
  - iOS node pairing,
  - Android node pairing,
  - Canvas,
  - camera,
  - screen recording,
  - location,
  - notifications,
  - device actions,
  - contacts/calendar,
  - photos,
  - SMS where applicable,
  - push-wake / disconnected-node rehydration where mobile nodes need it.
- [ ] Add typed node protocols in Rust rather than ad hoc compatibility layers.
- [ ] Add secure device capability gating and operator approval for node commands.

Exit criteria:

- [ ] OpenRustClaw has a Rust-native extension and node model that can do what OpenClaw documents, without depending on non-Rust runtime ownership.

## Phase 8: Security, Operations, Observability, and Full-Parity Exit

Goal: close the remaining product gaps, prove parity, and harden production operations.

Completed:

- [x] A substantial part of the shipped runtime now reports honest health and trace state.
- [x] LangSmith tracing exists across several major runtime paths.
- [x] Security posture is materially stronger than the early placeholder state.

Remaining:

- [ ] Extend observability to all operator-significant paths:
  - every channel ingress,
  - every outbound send,
  - every workflow/job run,
  - memory maintenance,
  - media processing,
  - plugin execution,
  - node commands.
- [ ] Add OpenTelemetry/Prometheus coverage for parity-critical runtime metrics.
- [ ] Add full auth and access-control parity where OpenClaw documents it:
  - tokens,
  - allowlists,
  - origin checks,
  - pairing approval,
  - per-agent restrictions,
  - operator roles for Control UI if introduced.
- [ ] Add production ops parity:
  - service installation,
  - backups,
  - restore,
  - log rotation,
  - config migration,
  - upgrade playbooks,
  - self-update and rollback,
  - PID/gateway lock semantics,
  - launchd/systemd integration,
  - network modes for loopback/LAN/remote deployment,
  - trusted-proxy auth mode for reverse proxies.
- [ ] Add channel/service runtime resilience features:
  - channel health monitor with configurable auto-restart,
  - presence and liveness beacons for operator surfaces,
  - readiness probes that reflect real channel connectivity.
- [ ] Add binary-first Rust operations strengths as first-class release goals:
  - precompiled binaries for major targets,
  - cross-compile support for x86_64/ARM64 and constrained devices where feasible,
  - resource-budget regression checks for idle RAM, startup latency, and binary size.
- [ ] Build a parity test suite that validates behavior against documented OpenClaw scenarios.
- [ ] Build fixture-based integration suites for:
  - channel routing,
  - pairing,
  - group mention rules,
  - media workflows,
  - session tools,
  - node commands.
- [ ] Run a final docs audit so every supported feature has:
  - user docs,
  - operator docs,
  - troubleshooting,
  - test coverage.
- [ ] Remove or demote any remaining runtime dependency that prevents "all Rust in production" from being true.

Exit criteria:

- [ ] The parity matrix is green for the declared target surface.
- [ ] Production runtime is Rust-owned end to end.
- [ ] Remaining gaps are only intentional divergences, not missing parity.

## Recommended Execution Order

The phases stay in order, but implementation should happen in these vertical slices:

1. Rust workflow runtime replacement for scheduler, memory maintenance, and RAG.
2. Rust-native autonomous optimization framework for skills, prompts, RAG, and bounded workflow improvements.
3. Session tools and direct/group/thread routing parity.
4. WhatsApp and iMessage parity.
5. Control UI and typed operator APIs.
6. Media pipeline and transcription parity.
7. Plugin and extension model completion.
8. Node pairing and device-command parity.
9. Final observability, ops, and full parity validation.

## Release Gates

Do not claim "OpenClaw parity" until all of the following are true:

- [ ] Control UI exists with practical operator parity.
- [ ] WhatsApp, Telegram, Discord, Slack, and iMessage parity targets are complete.
- [ ] Session tooling parity is complete.
- [ ] Media send/receive and voice-note transcription parity is complete.
- [ ] Multi-agent routing and pairing flows match documented behavior.
- [ ] Plugin and node capability surface is implemented or intentionally excluded with clear product rationale.
- [ ] Python is not required for the shipped runtime path.

## Intentional Divergences We Should Preserve

These are not parity failures if they stay stronger than OpenClaw's current surface:

- Rust-native persistence and scheduling instead of JS-first runtime ownership.
- Multi-provider LLM support instead of a Pi-only coding-agent path.
- Stronger capability enforcement and safer extension execution.
- MCP and mcp2-cli as first-class native tooling.
- More explicit docs/runtime truthfulness and stricter CI gates.
- Binary-first distribution, self-update, and tighter memory/startup budgets where Rust gives a real operational advantage.

## Definition of Done

This roadmap is complete when:

- [ ] the parity matrix is complete and green for the declared target surface,
- [ ] the production runtime is Rust-owned end to end,
- [ ] no shipped feature relies on placeholder behavior,
- [ ] docs, tests, CLI, and runtime agree,
- [ ] OpenRustClaw can be described as "OpenClaw feature parity in Rust, plus OpenRustClaw-native improvements" without caveats that matter to operators.

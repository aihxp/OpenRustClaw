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
- [x] The Python sidecar is no longer part of the production-critical execution path; it remains an optional compatibility/experimental lane only.

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

## External Leverage Tracks

The roadmap can also selectively absorb concepts from adjacent projects where they strengthen OpenRustClaw without turning it into a thin wrapper around someone else's runtime.

Currently worth leveraging:

- `openclaw-persona`
  - workspace-readable identity/persona artifacts,
  - operator-editable memory views,
  - memory lifecycle automation,
  - sub-agent profile concepts.
- `openclaw-rfcs`
  - lifecycle hooks,
  - native vector-memory ergonomics,
  - first-class agent profiles,
  - sub-agent supervision/dashboard patterns,
  - memory consolidation hooks.
- `cureclaw`
  - structured event-stream adapter pattern for subprocess-backed agents,
  - optional external CLI wrapping as a local API,
  - optional cloud-agent access for delegated repo work,
  - optional remote workstation execution registry.

Guardrails:

- These are leverage tracks, not new parity baselines unless explicitly promoted into the parity matrix.
- External-agent bridges must remain optional and must not become the durable source of truth.
- Cloud-agent access is an operator capability, not a replacement for Rust-owned runtime parity.

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

- [x] Build a unified workflow registry with explicit execution tier metadata:
  - `rust_native`,
  - `compat_sidecar`,
  - `experimental_langgraph`.
- [x] Define routing policy for workflow dispatch based on:
  - durability requirements,
  - ship status,
  - operator visibility,
  - migration stage.
- [x] Replace scheduler workflow execution with Rust-native workflow/state-machine implementations.
- [x] Replace memory maintenance workflow execution with Rust-native workflow/state-machine implementations.
- [x] Replace RAG orchestration with Rust-native retrieval and context-assembly services.
- [x] Replace sidecar agent orchestration with Rust-native graph/state execution.
- [x] Keep shipped sidecar-backed workflows bounded and compatibility-only.
- [x] Ban sidecar-owned infinite loops and scheduler-owned durable truth outside Rust.
- [x] Keep LangSmith via direct API integration from Rust, not Python framework dependency.
- [x] Decide whether LangGraph remains:
  - [x] keep it as an authoring/prototyping format,
  - [x] keep it as an optional bounded compatibility bridge,
  - [x] remove it from the production-critical runtime path.
- [x] Make the decision explicit in runtime/configuration policy:
  - sidecar role is `compatibility`, `experimental`, or `disabled`,
  - compatibility dispatch only exists when explicitly configured,
  - experimental LangGraph usage is operator-visible and does not count as shipped parity by itself.
- [x] Build a Rust-native workflow abstraction that covers:
  - node execution,
  - retries,
  - checkpointing,
  - human approval,
  - resumability,
  - trace correlation.
- [x] Remove Python sidecar as a required production dependency from the default deployment path.
- [x] Keep a compatibility harness only if needed for migration or legacy workflow imports.

Exit criteria:

- [x] `openrustclaw start` can run the full shipped surface in Tier A without requiring Python.
- [x] Tier B exists only for bounded compatibility workflows.
- [x] Tier C is explicitly experimental and not confused with shipped parity.

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
- [x] A durable Rust-owned event bus and queued event-triggered workflow dispatch path now exist.
- [x] Operator controls exist for pause, resume, replay, attempt inspection, dead-letter inspection, and MCP-based scheduler introspection.
- [x] Rust-native workflow checkpoints exist for timed and event-triggered work.
- [x] Control/runtime management events now cover scheduler and plugin operator actions.

- [x] Expand the internal event bus coverage across the current shipped parity-critical sources:
  - message.received,
  - message.sent,
  - memory.stored,
  - memory.searched,
  - reminder.triggered,
  - reminder.delivered,
  - reminder.delivery_failed.
- [x] Extend the event bus to broader current operator/runtime-management sources:
  - plugin events,
  - Control UI/runtime management events,
  - scheduler operator actions.
- [ ] Handle `node.paired` and other distributed/mobile lifecycle events in the later node/distributed phases, not in Phase 3.
- [x] Support event-triggered workflows in Rust, not just time-triggered jobs.
- [x] Add reminder delivery policy controls across channels:
  - fallback channel order,
  - first-success vs broadcast delivery modes,
  - per-channel metadata/route targeting,
  - bounded multi-channel fan-out.
- [x] Extend reminder policies with:
  - quiet hours,
  - retries per channel,
  - per-agent delivery rules.
- [x] Add explicit lifecycle hooks on top of the Rust event bus for:
  - `session.start`,
  - `session.pre_compaction`,
  - `session.post_turn`.
- [x] Finish lifecycle hook coverage for shipped channel sessions with:
  - `session.end`,
  - durable payload delivery to internal event-triggered handlers.
- [x] Extend lifecycle hook coverage to additional non-channel/runtime-managed paths.
- [x] Add explicit hook policy semantics for event-triggered jobs:
  - enable/disable,
  - session-required filtering,
  - allowed hook lists.
- [x] Extend hook policies with:
  - timeout budgets,
  - failure isolation controls,
  - ordering/priority rules.
- [x] Add durable workflow checkpoints for long-running background tasks.
- [x] Add idempotent event reprocessing and crash recovery tests at the workflow boundary.
- [x] Add scheduler APIs for Control UI and MCP introspection parity.

Exit criteria:

- [x] All current shipped scheduled and event-driven automations are Rust-owned, durable, restart-safe, and operator-visible.

## Phase 3.5: Task Registry, File-Backed Task Manifests, and Operator Task Control

Goal: give operators a visible, standard, versionable task surface on top of the durable Rust scheduler without falling back to OS cron as the source of truth.

Status: complete for shipped CLI/MCP surfaces; Control UI reuse stays in Phase 6.

Principles:

- SQLite remains the durable scheduler source of truth.
- A file/folder task surface exists for visibility, review, import/export, and repo-level configuration.
- Task manifests sync into the scheduler rather than replacing it.
- Operators must be able to see where tasks live, how often they run, what priority they have, what triggered them, and what happened last.

Completed:

- [x] Define a standard workspace task path:
  - `.claw/tasks/` as the default hidden control-plane location,
  - optional project-local alias/import path for repo-visible checked-in task manifests where desired,
  - clear separation between task manifests, run artifacts, and generated state.
- [x] Define a Rust-native task manifest format for scheduled and event-triggered work:
  - stable task id,
  - human-readable name,
  - workflow target,
  - trigger definition,
  - priority,
  - delivery policy,
  - enable/disable state,
  - tags/ownership metadata,
  - concurrency and retry policy,
  - approval requirements,
  - channel/session routing metadata.
- [x] Add task priority semantics to the scheduler:
  - explicit numeric or named priorities,
  - tie-break ordering rules,
  - starvation protection,
  - operator-visible run-order reasoning.
- [x] Add filesystem-to-scheduler sync:
  - import task manifests into SQLite,
  - detect drift between manifest and persisted state,
  - safe apply/update/remove flow,
  - validation before activation,
  - dry-run preview of changes.
- [x] Add scheduler-to-filesystem export:
  - export active jobs into manifest files,
  - preserve comments/operator metadata where feasible,
  - support repo bootstrap and backup use cases.
- [x] Add operator-visible task status views across CLI and MCP:
  - where the task came from,
  - last run,
  - next run,
  - trigger type,
  - effective schedule,
  - priority,
  - state,
  - retries,
  - dead-letter state,
  - owning workflow/agent.
- [x] Defer Control UI views to Phase 6 so the same task registry is reused there rather than reimplemented.
- [x] Add richer task inspection and control:
  - pause/resume,
  - run now,
  - reprioritize,
  - disable until timestamp,
  - rebind target workflow,
  - inspect checkpoints,
  - inspect event subscriptions.
- [x] Add task folders for related artifacts without making them the source of truth:
  - per-task notes or instructions,
  - operator annotations,
  - run receipts,
  - exported logs/screenshots/artifacts where relevant.
- [x] Add task templates and generated scaffolds:
  - reminders,
  - recurring maintenance,
  - event-triggered hooks,
  - digests,
  - multi-channel delivery tasks.
- [x] Add explicit replacement language in docs and UI:
  - this is not OS cron,
  - this is Rust-owned durable scheduling with file-backed operator manifests,
  - explain how frequency, retries, priority, and triggers are interpreted.
- [x] Add evaluation and regression coverage for:
  - manifest import/export,
  - priority ordering,
  - task drift detection,
  - enable/disable semantics,
  - schedule changes without task loss.

Exit criteria:

- [x] Operators can manage tasks from a standard folder/manifests path without losing the durability guarantees of the Rust scheduler.
- [x] Task priority, frequency, ownership, and status are visible and controllable from CLI and MCP.
- [x] Control UI integration is deferred to Phase 6 rather than being treated as a separate task system.

## Phase 4: Memory, Sessions, Context, and RAG

Goal: match OpenClaw's memory and session ergonomics while keeping a stronger Rust-native persistence model.

Completed:

- [x] Rust-owned core memory, recall memory, archive maintenance, and RAG chunk storage exist.
- [x] Sidecar-era memory access paths already route through Rust-owned services.
- [x] RAG retrieval supports budgeting, source controls, filtering, and inspection.

Completed in this phase:

- [x] Implement session tools parity:
  - list sessions,
  - inspect history,
  - send into session,
  - spawn session,
  - archive/close session.
- [x] Implement OpenClaw-style direct-vs-group session semantics as explicit configurable policy:
  - direct chats can collapse into shared `main`,
  - groups isolate session state,
  - thread scope can override channel scope.
- [x] Add Markdown/QMD-style workspace memory files or a Rust-native equivalent with import/export parity.
- [x] Add file-backed memory views that are operator-readable and editable from the Control UI and CLI.
- [x] Add model-aware workspace artifact support so OpenRustClaw can manage the per-model file conventions that different coding/reasoning models expect:
  - `AGENTS.md`,
  - `AI.md`,
  - `CONTEXT.md`,
  - `ARCHITECTURE.md`,
  - `SKILL.md`,
  - `CLAUDE.md`,
  - `CLAUDE.local.md`,
  - `GEMINI.md`,
  - `.github/copilot-instructions.md`,
  - `.github/instructions/*.instructions.md`,
  - `.cursor/rules/*` and `.cursorrules`,
  - `.continue/rules/*`,
  - `Modelfile` for local open-weight model packaging where applicable,
  - model-specific memory or instruction files where they materially affect runtime quality,
  - import/export and sync policies rather than hardcoding one provider's convention as universal.
- [x] Build a canonical instruction/context artifact registry in Rust so OpenRustClaw understands these files as normalized artifact classes rather than ad hoc vendor-specific strings:
  - universal project guidance,
  - model/provider-specific guidance,
  - local-only/private overrides,
  - path-scoped rules,
  - agent-profile artifacts,
  - open-weight model packaging artifacts,
  - orchestration/task manifests where relevant.
- [x] Define artifact precedence and merge policy:
  - global vs workspace vs nested directory scope,
  - shared vs model-specific instructions,
  - local/private overrides vs versioned project rules,
  - explicit conflict reporting,
  - deterministic merge order visible to operators.
- [x] Add artifact sync policies for model changes:
  - update the active model's preferred artifact set,
  - synchronize shared content across equivalent files when configured,
  - preserve per-model overrides,
  - show operator-visible diffs before destructive rewrites.
- [x] Add artifact adapters/import-export paths for major ecosystems without making them all first-class sources of truth:
  - Anthropic/Claude Code,
  - Gemini CLI,
  - OpenAI Codex/AGENTS.md conventions,
  - GitHub Copilot instructions,
  - Cursor rules,
  - Continue rules,
  - Ollama `Modelfile`,
  - selected autonomous-agent tool conventions where operator demand justifies it.
- [x] Add privacy and repo-safety semantics for local-only artifacts:
  - `.local`-style overrides,
  - auto-gitignore recommendations,
  - secret scanning/redaction,
  - operator-visible distinction between shared and personal artifacts.
- [x] Add targeted memory lookup parity beyond broad search:
  - memory get,
  - namespace reads,
  - recent memory timeline,
  - archive inspection.
- [x] Add persona/identity artifact support, either as first-class Markdown artifacts or a Rust-native equivalent:
  - `SOUL` / persona values,
  - user profile/context,
  - autonomy/operating rules,
  - editable identity metadata.
- [x] Add memory write policies for:
  - user facts,
  - project facts,
  - agent facts,
  - session summaries.
- [x] Add memory lifecycle automation:
  - turn-level memory extraction/formation,
  - similarity-based consolidation/merge,
  - strengthening/abstraction of repeated patterns,
  - forgetting/pruning with archive recovery.
- [x] Add model-swap memory rehydration policies so context survives provider/model changes cleanly:
  - reload core memory after model swap,
  - recompute model-budgeted context windows,
  - translate or compact persona/memory artifacts for the new model's preferred format,
  - preserve session continuity without requiring restart or manual memory repair.
- [x] Add operator-visible learnings/error ledgers that can feed the optimization framework without becoming uncontrolled self-modification.
- [x] Add context compaction parity for long threads and high-volume group chats.
- [x] Add stronger retrieval quality:
  - chunkers for code/docs/media transcripts,
  - embeddings or hybrid rankers where justified,
  - benchmark datasets and regression scoring.
- [x] Add import/export and migration tools from OpenClaw-style memory/session data where feasible, including:
  - `MEMORY.md` style artifacts,
  - persona-style memory vaults where translation is practical.

Exit criteria:

- [x] Operators can manage sessions and memory with the same practical power as OpenClaw for the current shipped CLI and MCP surfaces.
- [x] Context assembly is deterministic, inspectable, and Rust-owned.

## Phase 5: Channels and Routing Parity

Goal: reach practical parity with OpenClaw's documented channel and routing surface.

Completed:

- [x] WebChat baseline exists.
- [x] Telegram outbound, polling ingress, and webhook ingress exist.
- [x] Slack outbound, HTTP Events ingress, and Socket Mode websocket ingress exist.
- [x] Discord outbound, interactions ingress, and gateway message ingress exist.
- [x] Matrix access-token/password auth, `/sync` polling ingress, outbound room sends, reactions, join/leave, and local file upload support exist.
- [x] File-backed channel account and binding registry exists under `.claw/channels/`.
- [x] Shipped runtime now applies channel/account/workspace binding precedence plus pairing approval for tier-1 channels.
- [x] Group mention activation rules now exist on the shipped runtime path.
- [x] CLI channel operator controls now cover init/list/approve/block/activation/bind.
- [x] Shared outbound send policy now supports preview-then-blocks, block chunking, coalescing, and pacing.
- [x] Shipped reply/edit/reaction semantics now exist where the current tier-1 channel APIs support them.

Still remaining for deeper parity:

- [x] Extend the shipped registry/operator surfaces so a future Control UI can reuse the same typed runtime/account/binding state without inventing a separate control model.
- [x] Implement higher-fidelity channel UX parity where it materially affects operator experience:
  - [x] Telegram forum topic lifecycle/admin actions,
  - [x] Discord forwarded-attachment downloads beyond metadata/file-reference capture,
  - [x] Slack draft-stream replies.
- [x] Implement media in/out parity per channel:
  - attachment downloads and platform-native uploads where current support is still reference-based outside the now-complete Discord path,
  - file references.

Channel completion remaining:

- [x] WhatsApp parity:
  - multi-account login,
  - pairing/QR flow,
  - direct/group routing,
  - mentions,
  - media,
  - replies,
  - reconnect behavior.
- [x] iMessage parity:
  - local bridge integration,
  - send/receive,
  - attachment handling,
  - contact/group mapping.
  - current shipped path covers BlueBubbles/macOS direct send, verified BlueBubbles ping on connect, webhook-password validation for BlueBubbles ingress, direct-handle or chat-guid targeting, structured group/chat mapping from webhook payloads, structured attachment metadata plus outbound local-file sends in BlueBubbles mode, tapbacks including normal channel-send reactions, first-class CLI operator flows for ping/server/chats/contacts/send/send-file/tapback, and local agent/session routing.
- [x] Mattermost parity via Rust-native plugin/channel implementation.
  - shipped runtime now supports Rust-native Mattermost outbound sends, slash-command/outgoing-webhook ingress, shared-token validation, user/channel allowlists, trigger-word stripping, bot-mention detection, threaded replies via `root_id`, local file uploads through the Mattermost REST API, and normal local agent/session routing.
- [x] Google Chat parity.
  - current shipped path covers direct webhook ingress plus Pub/Sub push-envelope decoding, token or service-account outbound auth, response-mode gating, normalized space/user IDs, slash-command plus mention metadata capture, message ID plus body/thread/argument/slash/mention presence metadata, identity and event-time presence flags, derived display-name/action/attachment count and length metadata, card-click and space lifecycle routing, file-reference cards, structured attachment/file-reference metadata plus attachment-name/type/data-ref presence capture and attachment/file-reference counts, optional local attachment downloads with normalized `local_path` enrichment, first-class CLI operator flows for `status|connect|send|send-card|send-link-card`, and local agent/session routing.
- [x] Google Meet parity.
  - current shipped Rust path covers space creation/inspection, active-conference termination, conference-record/participant/recording/transcript inspection, direct-event plus Google Workspace Events / Pub/Sub payload decoding with transcript hydration, live webhook ingress on `openrustclaw start`, and durable runtime-event publication for Meet workflow automation.
- [x] Gmail inbound automation parity for mail-triggered workflows.
  - current shipped path covers Gmail watch setup and stop, Pub/Sub or direct-notification webhook ingress, history fetch, message hydration, structured attachment/file-reference metadata, normalized recipient/label counts plus recipient domains, message/thread/history/subject/body/received-at presence flags, recipient/domain/label/header presence flags, file-reference counts, attachment total-size metadata, address/domain length and attachment-name/MIME count metadata, unread and received-at metadata, parsed header/thread metadata, history/body-length metadata, subject/body/file-reference presence metrics, mail-triggered local agent/session routing, direct sends, threaded direct sends, replies including local-file attachments, metadata-driven label/archive/delete/forward actions, forwarded local-file attachments, and first-class CLI operator flows for `status|connect|process-notification|send|reply|label|archive|delete|forward`.
- [x] Matrix parity.
  - current shipped path covers auth, send, reply/thread relations with explicit presence flags, format/formatted-body presence metadata, media-presence metadata, polling ingress, inbound reaction, redaction, and membership events with richer lifecycle metadata plus target/reason/avatar/display-name presence flags, reaction/redaction/membership/media length metadata, room/event/sender/reply/content-URI derived lengths, room actions, helper-driven plus normal-send local file uploads, inbound media downloads into the Matrix data directory with structured file-reference metadata plus media MIME/size fields and download/content-uri/filename/size/file-reference presence flags, and first-class CLI operator flows for join/leave/list-rooms/send-formatted/react/send-file/typing/redact.
- [x] Microsoft Teams parity.
  - current shipped path covers Bot Framework webhook ingress, JWT verification, connector-`/v3` outbound operations, reply aliases, native local-file uploads via Connector attachments, metadata-driven typing/update/delete actions, conversation/reaction/update/delete lifecycle routing with normalized member/reaction id/count metadata and presence flags, normalized team/channel/tenant IDs, richer message body/reply/update/delete flags, mention presence/count metadata, richer attachment metadata plus structured file references, attachment name/type/url presence flags, attachment URL counts, file-reference counts, local download paths, adaptive-card file links, and local agent/session routing.
- [x] Signal parity if kept in scope.
  - current shipped path covers `signal-cli` daemon-backed direct/group send-receive, allowlist filtering, first-class CLI operator flows for registration/verify/link/list-groups, normalized routing metadata, source/recipient/group presence flags, text-length and attachment-size/file-reference metrics, source-number/group-name/quote-author length fields, outbound local-attachment sends, structured inbound attachment/file-reference metadata with daemon-provided local attachment path enrichment where available, attachment-only inbound routing, normalized attachment ids/names/mime types plus caption counts and attachment-id/name/MIME/caption presence flags, flat quote plus reply-target metadata with quote presence/text-length flags, sync-message lifecycle visibility, normalized group-member and mention metadata with presence flags, receipt lifecycle events, and local agent/session routing.
- [x] Defer optional Feishu/Lark parity out of the Phase 5 completion gate unless the product scope is expanded to include it:
  - docs/tables actions,
  - rich-text embedded media extraction.
- [x] Defer broader plugin-channel parity out of the Phase 5 completion gate and track it with the extension/plugin roadmap instead:
  - OpenClaw-documented plugin-channel support remains a real parity topic,
  - but it now belongs to the Rust-native extension/plugin model rather than blocking the shipped channel-runtime surface.
- [x] Normalize shared workspace/channel-scope/group/mention routing metadata across newly promoted partial-runtime channels so binding and isolation policy works consistently outside the original tier-1 set.

Exit criteria:

- [x] OpenRustClaw supports the same practical operator channel set targeted by OpenClaw for the declared shipped channel surface, with Rust-owned runtime paths.

## Phase 6: Tools, MCP, CLI, and Control Surfaces

Goal: match OpenClaw's operator UX and tooling surface while keeping Rust-native interfaces.

Status: complete for shipped CLI/MCP control-plane surfaces, shared typed control/config/diagnostics APIs, autonomy-tier and decision-lesson control surfaces, shared service-status, scheduler, channel-readiness, runtime-health, runtime-beacon, runtime-reload-plan, and runtime-log inspection APIs, shared session/memory/job inspection APIs, channel account CRUD parity, encrypted runtime secret-vault support, validated provider/model switching, runtime reload/cutover APIs with rollback plus partial-reload versus restart-required guidance, persisted applied-runtime snapshots, basic persisted provider/fallback health scans with degraded-mode warnings, full trust-first routed/orchestrated multi-model execution surfaces with receipt capture, checkpoints, trace logs, transcript persistence, parent/child delegation relationships, active supervision state, pause/resume/kill controls, estimated token/duration resource summaries, request-scoped model/autonomy override lanes, reflection candidates, reflection-to-lesson promotion, and lesson-aware steering context, stronger onboarding flows with existing-state handling plus post-onboarding doctor handoff, doctor validation, a shipped browser/tool-group slice with open-session/sessions/inspect/run-sequence/artifact inspection plus read-page/crawl-site/navigate/extract/screenshot/pdf surfaces across CLI, MCP, `/control/browser/...`, and `/control/ui`, including a Tier B `agent-browser` compatibility backend under the same Rust-owned control contract, typed extension-management APIs for `/control/skills...`, and an initial Web Control UI shell with config/vault editing plus runtime-health, runtime-beacon, runtime-reload-plan, autonomy/lesson, installed-extension inspection and registry discovery, active-run supervision, run-supervision/run-trace/run-transcript/run-resource inspection, enabled-channel readiness, persisted readiness snapshots, and recent/live runtime log visibility; deeper Web Control UI parity and deeper diagnostics still remain open, and channel auto-restart remains an operations hardening item rather than a Phase 6 Wave 3 blocker.

Additional later-surface work:

- [ ] Google Meet add-on UI embedding and live in-meeting collaboration surfaces.
- [ ] Matrix E2EE and advanced device-state management.

Completed:

- [x] MCP stdio server is real.
- [x] MCP exposes memory, scheduling, and RAG inspection.
- [x] `mcp2-cli` exists as an operator/debug tool, including remote legacy SSE support via `--mcp` and a workspace-owned saved-source registry for reusable MCP/OpenAPI entries.
- [x] Add a file-backed control-plane registry under `.claw/control/` for:
  - agent profiles,
  - model profiles,
  - Claw manifests,
  - runtime mode,
  - autonomy policy,
  - decision lessons,
  - task/category bindings.
- [x] Add CLI control-plane surfaces:
  - `openrustclaw control init`,
  - `openrustclaw control list`,
  - `openrustclaw control show`,
  - `openrustclaw control validate`,
  - `openrustclaw control describe`,
  - `openrustclaw control create-agent`,
  - `openrustclaw control create-model`,
  - `openrustclaw control create-claw`,
  - `openrustclaw control mode`,
  - `openrustclaw control assign-task`,
  - `openrustclaw control assign-category`.
- [x] Add MCP control-plane introspection surfaces:
  - `list_agent_profiles`,
  - `inspect_agent_profile`,
  - `list_model_profiles`,
  - `inspect_model_profile`,
  - `list_claws`,
  - `inspect_claw`,
  - `inspect_runtime_mode`,
  - `self_describe_runtime`.
- [x] Make Claw itself aware of the current runtime mode and delegation policy by syncing `.claw/control/CLAW_RUNTIME.md` into the workspace artifact registry.
- [x] Add onboarding scaffolding for the control plane:
  - initialize `.claw/control/`,
  - choose solo vs multi-claw mode,
  - scaffold orchestrator mode defaults when selected,
  - show model-role recommendations during setup.
- [x] Add `models scan` as the shipped recommendation/health surface for role-aware provider defaults.
- [x] Expand `doctor` so it can validate and, where safe, repair the shipped control-plane registry.
- [x] Build typed HTTP and WebSocket APIs that the Control UI and external clients share:
  - `GET /control/runtime`,
  - `GET/PUT /control/config`,
  - `POST /control/config/validate`,
  - `GET /control/diagnostics`,
  - `GET /control/diagnostics/ws`,
  - shared typed channel-registry HTTP routes under `/control/channels/...`.
- [x] Add a shared typed configuration protocol so CLI, future Control UI, and future external clients can read, validate, and write the same `AppConfig` model instead of diverging ad hoc flows.
- [x] Add channel account CRUD parity to the CLI and shared Control API:
  - `openrustclaw channels create-account|show-account|delete-account|show-binding|delete-binding`,
  - HTTP create/show/update/delete routes for channel accounts and bindings under `/control/channels/...`.
- [x] Add baseline operator diagnostics surfaces:
  - reusable typed `doctor` reports,
  - `openrustclaw doctor --non-interactive`,
  - `GET /control/diagnostics`,
  - `GET /control/diagnostics/ws`.
- [x] Add encrypted runtime secret-vault and secret-source loading:
  - workspace `.env` support for runtime/provider loading,
  - encrypted `.claw/control/runtime-vault.json` storage,
  - CLI secret management via `openrustclaw runtime vault ...`,
  - shared Control API secret management via `/control/runtime/vault/...`.
- [x] Add runtime reconfiguration surfaces for safe provider/model cutover:
  - `openrustclaw runtime status|reload|switch-provider|switch-model`,
  - `GET /control/runtime/status`,
  - `POST /control/runtime/reload`,
  - `POST /control/runtime/switch-provider`,
  - `POST /control/runtime/switch-model`,
  - validation before cutover,
  - timestamped config backups,
  - rollback when runtime reload fails after a config change.
- [x] Add bounded routed/orchestrated multi-model execution surfaces:
  - `openrustclaw orchestrate resolve|run`,
  - `POST /control/orchestration/resolve`,
  - `POST /control/orchestration/run`,
  - task/category/claw routing resolution against `.claw/control`,
  - model-profile fallback resolution when a configured profile is unsupported or unavailable,
  - persisted orchestration receipts under `.claw/control/orchestration-runs/`.
- [x] Add trust-first autonomy and decision-memory surfaces on top of the control plane:
  - autonomy tiers: `assisted`, `supervised`, `managed`, `autonomous`, `yolo`,
  - runtime guardrails for approvals, delegation caps, iteration caps, runtime budgets, and lesson-hint caps,
  - file-backed decision lessons under `.claw/control/lessons/`,
  - `openrustclaw control lessons|add-lesson|deactivate-lesson`,
  - `GET /control/autonomy`,
  - `GET/POST /control/autonomy/lessons`,
  - `POST /control/autonomy/lessons/{id}/deactivate`,
  - `/control/ui` autonomy-policy and decision-lesson panels.
- [x] Feed autonomy policy and scoped decision lessons into the shipped bounded orchestration surface:
  - lesson-aware steering notes on routing decisions,
  - runtime-capped delegation counts instead of hardcoded planner limits,
  - autonomy/lesson context in planner and worker prompts,
  - orchestration receipts that preserve the steering context used for a run.
- [x] Add supervision detail to the shipped bounded orchestration surface:
  - execution checkpoints for routing, planning, worker steps, and synthesis,
  - reflection candidates derived from fallback paths, failed workers, low-confidence outputs, and open questions,
  - `openrustclaw orchestrate list|inspect`,
  - `GET /control/orchestration/runs/{receipt_id}/checkpoints`,
  - `GET /control/orchestration/runs/{receipt_id}/supervision`,
  - `/control/ui` run-supervision inspection panel.
- [x] Close the operator feedback loop on bounded orchestration:
  - `openrustclaw orchestrate promote-candidate`,
  - `POST /control/orchestration/runs/{receipt_id}/reflection-candidates/{index}/promote`,
  - `/control/ui` reflection-candidate promotion controls,
  - promoted lessons inherit scoped task/category/claw/model/provider/autonomy defaults from the run receipt instead of relying on hidden automatic policy mutation.
- [x] Add orchestration trace visibility surfaces without forcing constant outer-loop intervention:
  - planner/worker/synthesis trace entries persisted in receipts,
  - parent/child delegation relationships persisted alongside the trace,
  - `openrustclaw orchestrate trace`,
  - `GET /control/orchestration/runs/{receipt_id}/trace`,
  - `/control/ui` run-trace inspection panel.
- [x] Add orchestration resource-visibility surfaces so operators can audit cost/latency shape without turning the agent into a micromanaged script:
  - per-trace estimated input/output token counts,
  - per-trace duration measurements,
  - `openrustclaw orchestrate resources`,
  - `GET /control/orchestration/runs/{receipt_id}/resources`,
  - `/control/ui` run-resource inspection panel.
- [x] Add request-scoped live override lanes on top of the shipped bounded orchestration surface:
  - override primary model profile per run,
  - override worker model profile per run,
  - override autonomy level, delegation/iteration/runtime caps, and approval policy per run,
  - keep the durable control registry unchanged while preserving overrides in the receipt,
  - expose the same override lane through `openrustclaw orchestrate resolve|run` and `/control/orchestration/resolve|run`.
- [x] Extend quarterback/orchestrator mode into a full trust-first orchestrated multi-claw runtime:
  - `openrustclaw orchestrate submit|active|watch|pause|resume|kill`,
  - `POST /control/orchestration/submit`,
  - `GET /control/orchestration/active`,
  - `GET /control/orchestration/active/{run_id}`,
  - `GET /control/orchestration/active/{run_id}/events`,
  - `POST /control/orchestration/active/{run_id}/pause|resume|kill`,
  - active run state under `.claw/control/orchestration-active/`,
  - named worker delegation with bounded follow-up turns,
  - primary-Claw selective critic/quality-review passes,
  - full parent/child transcript persistence via `openrustclaw orchestrate transcript` and `GET /control/orchestration/runs/{receipt_id}/transcript`.
- [x] Add a bounded native browser operator surface backed by the Rust automation crate:
  - `openrustclaw browser read-page|crawl-site|navigate|extract|screenshot|pdf`,
  - `POST /control/browser/read-page`,
  - `POST /control/browser/crawl-site`,
  - `POST /control/browser/navigate`,
  - `POST /control/browser/extract`,
  - `POST /control/browser/screenshot`,
  - `POST /control/browser/pdf`,
  - default artifact storage under `.claw/browser/`.
- [x] Add an initial Web Control UI shell that reuses the shipped typed control APIs:
  - `GET /control/ui`,
  - live diagnostics stream via `/control/diagnostics/ws`,
  - runtime/config/channel inspection,
  - orchestration receipt inspection via `/control/orchestration/runs...`,
  - browser artifact inspection and bounded browser action forms via `/control/browser/...`.
- [x] Add shipped service diagnostics and scheduler inspection surfaces:
  - `openrustclaw runtime services status|scheduler|events`,
  - `GET /control/services/status`,
  - `GET /control/services/scheduler`,
  - `GET /control/services/runtime-events`,
  - service-status, scheduler-health, and recent-runtime-events panels in `/control/ui`.
- [x] Add shipped enabled-channel readiness probes on top of the service baseline:
  - `openrustclaw runtime services channels`,
  - `GET /control/services/channels`,
  - enabled-channel readiness panel in `/control/ui`,
  - remote auth probes where the shipped channel path can do it cheaply and safely,
  - config/local-runtime readiness checks for the remaining shipped channels.
- [x] Add shipped runtime log visibility surfaces for operators:
  - file-backed runtime log capture under `.claw/control/runtime.log`,
  - `openrustclaw runtime services logs`,
  - `GET /control/logs/recent`,
  - `GET /control/logs/ws`,
  - `/control/ui` recent/live runtime-log inspection.
- [x] Add shipped session, memory, and scheduled-job inspection surfaces for shared HTTP/UI parity:
  - `GET /control/sessions`,
  - `GET /control/sessions/{id}`,
  - `GET /control/memory/namespaces`,
  - `GET /control/memory/timeline`,
  - `GET /control/memory/archive`,
  - `GET /control/jobs`,
  - `GET /control/jobs/{id}`,
  - `/control/ui` panels for sessions, memory namespaces/timeline/archive, and scheduled-job inspection.

Remaining:

- [x] Implement full session-tool parity exposed through MCP, CLI, and runtime APIs.
- [x] Add first-class agent profile configs for spawned agents:
  - model/thinking/timeout defaults,
  - tool allow/deny policies,
  - memory inheritance/scope,
  - output policies,
  - profile inheritance/versioning.
- [x] Add first-class model profile support:
  - provider/model capability descriptors,
  - per-model artifact preferences,
  - token/context budget policies,
  - reasoning/latency/cost hints,
  - safe fallback ordering when the primary model becomes unavailable or unauthorized.
- [x] Add a recommended model-role policy so onboarding can make sane defaults without hardcoding permanent provider assumptions:
  - recommend Groq for low-latency core runtime use,
  - recommend OpenRouter for broad fallback/control-plane coverage and free-model discovery,
  - recommend SiliconFlow for higher-capability secondary core routing,
  - recommend Ollama as the local/offline safety net,
  - keep all recommendations BYOK and scan-validated rather than assuming any model stays available forever.
- [x] Add an artifact-capability matrix to model profiles so OpenRustClaw can reason about which model/tool ecosystems prefer which file formats and how to translate them:
  - markdown instruction files,
  - path-scoped rule files,
  - local-only/private files,
  - YAML/TOML agent configs,
  - model-baked artifacts such as `Modelfile`.
- [ ] Extend multi-model execution beyond the shipped bounded routing surface:
  - model-per-tool or model-per-workflow selection,
  - deeper automatic routing by cost, latency, privacy, or capability,
  - richer live runtime overrides beyond the shipped CLI/control API receipts.
- [x] Add a first-class multi-claw execution model so users can choose how many Claws exist and how work is assigned:
  - `solo_claw` mode where one Claw handles all work,
  - `task_assigned` mode where individual tasks bind to specific Claws,
  - `category_assigned` mode where task categories map to specific Claws,
  - `orchestrated` mode where a quarterback/orchestrator Claw delegates to worker Claws,
  - explicit per-Claw identity, profile, model, tool, and memory scope.
- [x] Make multi-claw availability discoverable to both users and Claw itself:
  - user-visible mode selection and inspection from onboarding, CLI, and future Control UI,
  - runtime introspection so Claw knows whether it is running solo, assigned, or orchestrated,
  - operator-visible list of available Claws, what they own, and what delegation paths are allowed,
  - prompt/runtime context that tells each Claw when other Claws exist and when delegation is appropriate.
- [x] Add first-class task-to-Claw and category-to-Claw assignment policies:
  - direct task binding,
  - category routing tables,
  - default fallback Claw,
  - per-workspace overrides,
  - operator review and override surfaces.
- [x] Add a bounded quarterback/orchestrator model mode:
  - one model plans or routes work,
  - one or more worker models execute bounded subtasks,
  - structured worker completion envelopes,
  - explicit receipts showing which model decided versus which model executed.
- [x] Extend quarterback/orchestrator mode into a full orchestrated multi-claw runtime:
  - primary Claw can delegate to named worker Claws,
  - worker Claws return structured completion envelopes,
  - primary Claw can ask follow-up questions of worker Claws,
  - primary Claw decides when worker output is complete enough to expose to the user,
  - Rust persists the full parent/child Claw transcript and ownership chain.
- [x] Add sub-agent supervision surfaces:
  - active agent list/watch,
  - logs,
  - kill/pause/resume,
  - resource usage,
  - parent/child run relationships.
- [ ] Implement richer agent tool groups parity:
  - memory tools,
  - session tools,
  - filesystem tools,
  - browser/web tools,
  - media tools,
  - node tools.
- [ ] Build a Rust-native web access and browser automation stack that follows the execution-tier model instead of introducing a new permanent Python runtime dependency.
  - [ ] Define the capability split explicitly:
    - read-only web fetch/extract,
    - crawl and map for RAG ingestion,
    - interactive browser automation,
    - optional managed-browser compatibility,
    - optional stealth/anti-bot transport profiles.
  - [ ] Build a read-first HTTP acquisition lane in Rust for "help Claw read" tasks:
    - caching,
    - redirects,
    - cookies,
    - robots and rate-limit policy hooks,
    - readability/article extraction,
    - markdown/plain-text normalization,
    - metadata extraction,
    - PDF/document fetch handoff.
  - [ ] Build a crawl/map lane for doc and site ingestion:
    - sitemap discovery,
    - bounded site crawling,
    - canonical URL handling,
    - deduplication,
    - chunking handoff into Rust-owned RAG storage,
    - refresh/re-crawl policies,
    - source attribution and crawl receipts.
  - [ ] Build an interactive browser lane in Rust:
    - Chrome/Chromium DevTools Protocol control,
    - page/tab/session management,
    - navigation history,
    - DOM snapshotting,
    - accessibility-tree extraction,
    - forms,
    - uploads/downloads,
    - cookies/storage-state import and export,
    - screenshot/PDF capture.
  - [ ] Build a hybrid page-understanding layer:
    - DOM/accessibility-first element discovery,
    - screenshot capture for visual verification,
    - optional vision-model grounding for visually ambiguous pages,
    - stable action-target ids rather than raw coordinate-only execution.
  - [ ] Build a Rust-native action planner/executor contract for browser work:
    - navigate,
    - click,
    - type,
    - select,
    - scroll,
    - drag,
    - extract text/links/forms/tables,
    - wait-for conditions,
    - bounded retry semantics,
    - operator approval gates for sensitive actions.
  - [ ] Add browser/web tool surfaces to MCP, CLI, and runtime APIs:
    - [x] bounded CLI/runtime browser surfaces for navigate, extract, screenshot, and PDF capture
    - [x] read page
    - [x] crawl site
    - [x] open session
    - [x] inspect DOM/accessibility tree
    - [x] run bounded action sequences
    - [x] capture screenshots/artifacts
    - [x] export artifacts for later inspection
  - [x] Add a compatibility bridge for Playwright MCP or equivalent external browser runtimes only as Tier B:
    - optional operator-configured backend,
    - typed event translation,
    - no durable truth outside Rust,
    - explicit "compatibility" labeling in docs and UI.
  - [x] Add `agent-browser` as a first-class Tier B Rust-compatible browser backend rather than treating it as an ad hoc external tool.
    - Use it as a compatibility and operator-debug backend while the native CDP lane matures.
    - Keep OpenRustClaw as the durable source of truth for workflow state, retries, approvals, artifacts, and policy.
    - [x] Implement a shared `BrowserBackend` abstraction in Rust with at least:
      - `native_cdp`,
      - `agent_browser_cli`,
      - future managed/remote backends.
    - [x] Normalize `agent-browser` capabilities into the same internal contract used by the native lane:
      - open session,
      - navigate,
      - snapshot DOM/accessibility tree,
      - click/type/select/scroll,
      - extract structured text/links/forms,
      - capture screenshot/PDF,
      - import/export state,
      - run bounded batch actions.
    - [x] Leverage `agent-browser` features where they clearly accelerate parity:
      - machine-readable `--json` output,
      - `batch --json`,
      - ref-based targeting from snapshots,
      - `--session` / `--session-name` / `--profile` / `--state`,
      - domain allowlists,
      - action-policy files,
      - explicit confirmation categories,
      - local auth-vault and encrypted session support.
    - [x] Treat `agent-browser` as the preferred operator-debug and early-eval backend before full native parity:
      - easier reproducible browser sessions,
      - faster bounded multi-step runs,
      - better artifact capture during bring-up.
    - [x] Do not let `agent-browser` become the only browser path:
      - crawl/map and long-term RAG ingestion remain Rust-owned,
      - safety policy remains OpenRustClaw-owned,
      - runtime APIs must not depend directly on raw CLI output shapes.
    - [x] Add backend selection policy:
      - `native_cdp` default for production when supported,
      - `agent_browser_cli` optional for compatibility/debug/operator workflows,
      - explicit UI/CLI labeling of which backend executed a run.
    - [ ] Add backend-specific safety mapping:
      - map OpenRustClaw domain allowlists to `--allowed-domains`,
      - map approval-gated action classes to `--action-policy` and `--confirm-actions`,
      - map session isolation rules to `--session`, `--session-name`, and profile/state paths.
    - [x] Add backend-specific observability:
      - persist batch command receipts,
      - snapshots,
      - screenshots,
      - downloaded artifact references,
      - backend identity (`native_cdp` vs `agent_browser_cli`) on each run.
  - [ ] Add optional managed-browser infrastructure compatibility for scale-sensitive operators:
    - Browserbase-style remote browser backends,
    - proxy/session profile support,
    - session replay artifact import,
    - clear separation from the local default path.
  - [ ] Add optional stealth/anti-bot transport profiles as an operator-controlled capability rather than default behavior:
    - impersonation-aware HTTP client profiles,
    - configurable proxy backends,
    - init-script injection for browser sessions,
    - humanized input timing profiles,
    - explicit legal/compliance/operator-policy gating.
  - [ ] Add safety boundaries for browser automation:
    - domain allowlists and deny lists,
    - secret redaction in captured artifacts,
    - purchase/transfer/login approval gates,
    - form-submit confirmation policies,
    - download path sandboxing,
    - CSRF/session-isolation rules,
    - audit events for every sensitive browser action.
  - [ ] Add observability and replay for browser runs:
    - screenshots,
    - DOM/action traces,
    - network summaries,
    - failure artifacts,
    - LangSmith trace links,
    - operator replay for failed runs.
  - [ ] Add evaluation and benchmark coverage for web tasks:
    - read-only extraction accuracy,
    - docs/RAG crawl quality,
    - interactive task success,
    - latency/cost budgets,
    - regression suites for dynamic sites.
- [ ] Build the Web Control UI with parity for:
  - [x] initial dashboard shell for runtime/config/channels/orchestration/browser inspection
  - [x] configuration editing via shared `/control/config` validate/apply flows
  - [x] sessions inspection
  - [x] memory inspection
  - [x] scheduled jobs inspection
  - [x] channel account inspection
  - [x] service-status inspection
  - [x] logs
  - [x] extensions/plugins over the shipped skills runtime
  - live chat,
  - nodes,
  - [x] basic secret editing on top of the shared runtime vault API
  - richer vault workflows.
- [ ] Finish onboarding parity beyond the shipped scaffold:
  - [x] QuickStart vs Advanced path selection,
  - local gateway vs remote gateway/client mode,
  - [x] existing config detection with keep/modify/reset choices,
  - workspace/bootstrap file setup,
  - daemon/service install polish,
  - [x] post-onboarding health check and first dashboard/chat handoff,
  - richer remote access guidance.
- [ ] Make multi-claw mode and assignment state part of the shared typed configuration model:
  - current execution mode,
  - registered Claws,
  - per-Claw profile bindings,
  - task/category routing tables,
  - orchestrator/worker relationships,
  - safe downgrade back to solo mode.
- [ ] Add user configuration and settings parity:
  - interactive `configure`-style flows by section,
  - [x] typed config editing from CLI and Control UI,
  - settings validation before apply,
  - config migration and legacy-key detection,
  - live reload where safe,
  - backup/restore before destructive config repair.
- [x] Add onboarding-time provider/model scanning for the core and control-plane model lanes:
  - validate user-supplied keys,
  - list available models where provider APIs allow,
  - detect quota/auth/deprecation failures,
  - capture context-window and limit metadata where exposed,
  - recommend role assignments for `core_model`, `control_plane_model`, and ordered fallbacks.
- [x] Add an agent self-configuration harness so users can ask Claw to configure itself without the model operating blindly:
  - machine-readable self-description of enabled features, limits, tools, channels, and current config,
  - typed introspection APIs for provider/account/channel/model state,
  - safe config-edit proposals with validation before apply,
  - dry-run explanations and rollback points,
  - explicit approval gates for risky self-reconfiguration.
- [ ] Extend secret-vault parity beyond the shipped runtime lane:
  - migrate more plaintext legacy secrets where feasible,
  - [x] add Control UI secret editing on top of the shared runtime vault API,
  - cover more non-provider secret classes with the same encrypted-at-rest flow.
- [x] Extend config and personality hot-reload beyond the shipped runtime rebind:
  - editable persona/DNA/system prompt artifacts now reload cleanly because the runtime resolves workspace artifacts per request,
  - provider/runtime changes that are safe to rebind live are now surfaced through a persisted reload-plan lane,
  - operators now get explicit partial-reload versus full-restart guidance before applying runtime changes.
- [x] Add runtime provider and account switching without full restart where the active runtime can rebind safely.
- [x] Add model-switch hardening so changing providers/models does not crash the runtime:
  - transactional config swap,
  - validation before cutover,
  - session-safe rebind and rollback,
  - degraded-mode fallback to the prior config if the requested model cannot start.
- [ ] Extend operator-grade diagnostics beyond the shipped service baseline:
  - [x] enabled-channel readiness and auth probes where feasible,
  - deeper trace-link surfacing,
  - richer secrets/service-state inspection,
  - [x] real log streaming,
  - richer failure artifact drill-down.
- [x] Expand `doctor` into a high-signal repair and migration surface inspired by OpenClaw:
  - `openrustclaw doctor`,
  - `openrustclaw doctor --repair`,
  - `openrustclaw doctor --deep`,
  - `--non-interactive` headless-safe mode,
  - config backup before rewrite,
  - legacy key/shape normalization,
  - orphaned state/session cleanup,
  - task-manifest and legacy scheduler-store normalization,
  - model/memory readiness checks,
  - channel auth fix hints,
  - sandbox/dependency diagnostics with actionable remediation.
- [ ] Add optional external-agent interoperability surfaces:
  - a typed local API that can wrap CureClaw/Cursor-style CLI agents as subprocess-backed event streams,
  - NDJSON or `stream-json` translation into OpenRustClaw agent/run events,
  - session continuity mapping,
  - approval/trace integration,
  - strict marking as optional compatibility rather than parity-critical runtime.
- [ ] Add optional remote workstation execution support:
  - named SSH workstation registry,
  - per-workstation session isolation,
  - CLI/MCP routing to a workstation target,
  - health/connectivity tests.
- [ ] Add optional cloud-agent interoperability:
  - launch/status/stop/list/conversation/models,
  - webhook status callbacks,
  - steer/evaluator loop for delegated repo work,
  - clear isolation from Rust-owned core parity/runtime claims.
- [ ] Decide whether remote MCP transport belongs in the parity surface or remains an OpenRustClaw-specific deferred feature.

Recommended finish order from here:

- [ ] Wave 1: finish the operator-trust surfaces that make the current runtime easier to run safely day to day:
  - deeper Web Control UI parity,
  - richer operator-grade diagnostics,
  - onboarding and repair parity.
- [x] Wave 2: finish the trust-first orchestration runtime before broadening autonomy claims:
  - full orchestrated multi-claw runtime,
  - sub-agent supervision surfaces,
  - decision-quality and reflection/lesson steering that improves choices without over-harnessing strong models into brittle micromanagement.
- [x] Wave 3: finish the broader web and extension surface after the operator/runtime control layer is strong enough to supervise it:
  - [x] richer Rust-native browser/web tooling,
  - [x] MCP/browser/tool-group parity,
  - [x] extension/operator-management parity over the shipped skills runtime,
  - [x] broader hot-reload and provider/runtime resilience.
- [x] Wave 4: close the remaining operator experience gaps after the core runtime and browser/extension layers are solid:
  - [x] provider-backed inbound voice-note transcription and transcript injection for supported attachment refs,
  - [x] broader voice/media polish, including operator-grade voice status/voices/transcription/synthesis surfaces beyond the initial runtime injection lane,
  - [x] file-backed mobile node registry plus CLI/API/UI preview/operator surfaces,
  - [x] bounded mobile nodes and device-command parity for the shipped operator surface,
  - [x] final parity audit, docs alignment, and explicit intentional-difference review.

Exit criteria:

- [ ] An operator can configure, inspect, and drive the system from CLI, MCP, or Control UI without dropping into internal-only tools.
- [ ] OpenRustClaw has a Rust-native web access stack that can read, crawl, and interact with the web without requiring Browser Use or another Python browser agent runtime.

## Phase 7: Skills, Plugins, Media, Voice, and Nodes

Goal: reach OpenClaw's extension and device-command surface with Rust-native implementation choices.

Completed:

- [x] Skills install/update/remove/verify lifecycle exists.
- [x] Marketplace sync exists.
- [x] A real WASM executor with capability-aware enforcement exists.
- [x] A compiled skill artifact pipeline now exists:
  - file-backed compiled manifests, help indexes, scan reports, and generated MCP/CLI schemas under `.claw/skills/compiled/`,
  - compile-on-install/update/verify behavior for local skill paths,
  - token-efficient skill summaries for model/tool routing and operator inspection.
- [x] Compiled skills now expose a real MCP retrieval surface at startup:
  - `list_compiled_skills`,
  - `inspect_compiled_skill`,
  - dynamic `skill.<name>.summary|details|reference...` tools for approved/scanned compiled skills,
  - blocked compiled skills remain inspection-only with redacted content.
- [x] The generated compiled-skill CLI bridge is now real:
  - `openrustclaw skills invoke <name>` reuses the compiled cache instead of reparsing raw `SKILL.md`,
  - `/control/skills/{name}/invoke` and `/control/ui` expose the same bounded summary/detail/reference invocation lane,
  - blocked compiled skills remain redacted rather than pretending executable runtime parity already exists.
- [x] The long-term Rust-native extension model is now explicitly defined and emitted from the compile pipeline:
  - compiled skills now also emit `extension_manifest.json` beside the cached help/schema artifacts,
  - the manifest defines the Rust-native contract for WASI components, manifest-driven capability declarations, background services, command hooks, and tool injection without overstating current execution parity,
  - `openrustclaw skills list-extensions|inspect-extension`, `/control/skills/extensions...`, and `/control/ui` all expose the same manifest shape for operator inspection.
- [x] JavaScript/TypeScript plugin-host expectations are now superseded by a real Rust/WASM execution lane for shipped skills:
  - `openrustclaw skills execute <name> [--component ...] [--input ...]` now runs bounded `.wasm`/`.wat` components through the Rust `WasmSandbox`,
  - `/control/skills/{name}/execute` and `/control/ui` expose the same bounded execution lane,
  - compile-time capability declarations and verification/trust policy are applied before execution instead of trusting a JS-side plugin host.

Remaining:

- [ ] Add plugin parity for:
  - [x] agent tools through bounded compiled-skill MCP execution,
  - [x] background workflows through bounded compiled-skill scheduler execution,
  - [x] auth plugins through a file-backed compiled-skill auth registry, bounded OIDC authorize/code-exchange surfaces, callback handling, and encrypted runtime-vault token persistence,
  - voice-call plugins,
  - [x] channel extensions through file-backed channel binding metadata and bounded compiled-skill background hooks.
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
  - [x] provider-backed inbound attachment STT and transcript injection for supported runtime message flows,
  - [ ] broader STT parity,
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
- [x] Add typed node protocols in Rust for the shipped node command/control lane rather than ad hoc compatibility layers.
- [x] Add secure device capability gating and operator approval for node commands.

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
- [ ] Add control-plane model resilience so operator actions still work when the primary paid model is unavailable:
  - [x] operator-visible degraded-mode warnings now surface when the configured primary provider is unhealthy but a healthy fallback lane exists,
  - separate low-cost or local fallback model for onboarding, updates, config edits, and model-switch operations,
  - explicit distinction between primary task model and control-plane/safety model,
  - startup-time fallback validation,
  - operator warnings when the system is running in degraded control-plane mode.
- [ ] Add recurring provider/model health scans after onboarding:
  - [x] basic persisted runtime-health scans now refresh from the running gateway and are visible through CLI, API, and Control UI,
  - detect removed or disabled models,
  - detect auth or billing regressions,
  - detect changed limits where providers expose them,
  - precompute failover recommendations before the next model swap,
  - surface operator warnings instead of letting model changes fail blind at runtime.
- [ ] Add explicit governance for optional external execution backends:
  - allowed backend registry,
  - credential and token isolation,
  - audit trail for local CLI wrappers and cloud-agent calls,
  - operator policy for when external agent execution is permitted.
- [ ] Add channel/service runtime resilience features:
  - channel health monitor with configurable auto-restart,
  - [x] presence and liveness beacons for operator surfaces,
  - [x] readiness probes that reflect real channel connectivity.
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
- [x] Media send/receive and voice-note transcription parity is complete.
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

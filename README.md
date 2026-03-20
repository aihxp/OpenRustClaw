# OpenRustClaw

[![CI](https://github.com/aihxp/OpenRustClaw/actions/workflows/ci.yml/badge.svg)](https://github.com/aihxp/OpenRustClaw/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-2024_edition-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A high-performance AI agent platform written in Rust. 43 crates, 20 LLM providers, 15 CLI-startable messaging channels, and an optional Python sidecar kept only for bounded workflow compatibility.

Current execution planning lives in:

- [docs/feature-matrix.md](docs/feature-matrix.md)
- [docs/roadmap.md](docs/roadmap.md)
- [docs/parity-matrix.md](docs/parity-matrix.md)
- [docs/parity-positioning.md](docs/parity-positioning.md)

The roadmap target is explicit: achieve practical OpenClaw feature parity with a Rust-first runtime, while keeping OpenRustClaw-native improvements where they are stronger.
The execution model is also explicit: Rust-native for production-critical paths, sidecar compatibility for bounded migration, and LangGraph as the experimentation/authoring lane rather than the sole durability boundary.
`openrustclaw start` now defaults to the Rust runtime path without requiring Python; the sidecar is only used when a compatibility workflow is explicitly needed and configured.
LangGraph is therefore retained as an authoring/prototyping format and an optional compatibility bridge, but removed from the production-critical runtime path.
The shipped Rust runtime now owns durable scheduling, event-triggered workflows, session lifecycle hooks, hook execution policies, and reminder delivery with quiet-hours/retry-aware channel fallback policies.
The shipped runtime now also includes a file-backed task registry layer so operators can manage visible `.claw/tasks/`-style task manifests, priorities, sync/export, and task inspection workflows without making OS cron or loose files the durable source of truth.
The roadmap now includes a shipped Rust-native autonomous optimization framework inspired by `autoresearch`, generalized for skills, RAG, prompts, policies, bounded workflows, bounded code, and research-program targets.
The roadmap also now includes a Rust-native web access stack for read-only page extraction, crawl-for-RAG ingestion, interactive browser automation, MCP/browser tool exposure, and optional compatibility with external browser runtimes without making them the durable source of truth. That compatibility lane now explicitly includes `agent-browser` as a Rust-friendly Tier B backend for operator/debug use and early browser parity work while the native CDP path matures.
The shipped control surface now includes a broader browser/tool-group slice: `openrustclaw browser open-session|sessions|inspect|run-sequence|artifacts|read-page|crawl-site|navigate|extract|screenshot|pdf`, matching `/control/browser/...` APIs, MCP browser tools, durable session/artifact state under `.claw/browser/`, and an optional `agent-browser` Tier B compatibility lane under the same Rust-owned control contract.
The roadmap further includes model-aware artifact sync, memory rehydration on model swaps, multi-model routing with optional quarterback/orchestrator models, discoverable multi-claw execution modes, a self-configuration harness, and a control-plane fallback model so provider/model changes do not take the platform down.
The model-awareness track now also includes a canonical instruction/context artifact registry so OpenRustClaw can understand and translate common project guidance files like `AGENTS.md`, `AI.md`, `CONTEXT.md`, `ARCHITECTURE.md`, `CLAUDE.md`, `GEMINI.md`, Copilot instruction files, Cursor/Continue rules, and local open-weight `Modelfile` packaging.
The roadmap also now calls for onboarding-time provider/model scans and role-aware recommendations: Groq for low-latency core runtime use, OpenRouter for broad fallback/control-plane coverage, SiliconFlow for higher-capability secondary routing, and Ollama as the local/offline safety net, all validated against user-supplied keys rather than hardcoded assumptions.
The operator roadmap also now explicitly includes an OpenClaw-inspired onboarding journey, typed user configuration/settings flows, solo-versus-multi-claw setup choices, and a stronger `doctor` repair/migration surface rather than leaving these as ad hoc setup utilities.
The shipped control-plane surface now includes a file-backed `.claw/control/` registry for agent profiles, model profiles, Claw manifests, runtime-mode/task/category bindings, trust-first autonomy policy, operator-curated decision lessons, runtime self-description, shared typed control/config/diagnostics APIs, runtime service-status and scheduler inspection surfaces, persisted runtime-health scans with degraded-mode warnings, runtime reload-plan and liveness-beacon inspection, persisted channel-readiness snapshots, shared session/memory/job inspection APIs, typed skills/extension lifecycle APIs, an initial `/control/ui` dashboard with config validate/apply, vault editing, runtime-health/beacon/reload-plan inspection, autonomy/lesson inspection, installed-extension inspection and registry discovery, active orchestration supervision plus trace/transcript/resource detail, and recent/live runtime log visibility, encrypted runtime secret-vault support, validated provider/model switching, full trust-first multi-model orchestration surfaces with active-run state, checkpoints, trace logs, transcript persistence, estimated token/duration summaries, request-scoped model/autonomy override lanes, reflection candidates, parent/child delegation relationships, and MCP/CLI inspection tools; the deeper Web Control UI parity path still reuses this registry instead of inventing a separate state model.
The remaining closeout order is deliberate: operator-trust surfaces, the trust-first orchestration runtime, the browser/tool-group slice, extension operator management, and the broader runtime reload/resilience lane are now the shipped baseline. Wave 4 is now partially underway with provider-backed inbound voice-note transcription and transcript injection for supported attachment refs plus file-backed mobile node registry and preview/operator surfaces, but broader voice/media polish, real device-command parity, and the final parity audit still remain. The later long-term Rust-native plugin/channel model still remains a distinct roadmap track.

## Quick Start

```bash
git clone https://github.com/aihxp/OpenRustClaw.git
cd OpenRustClaw
cargo build --release

# Interactive setup -- configures providers, channels, and security
./target/release/openrustclaw onboard

# Scaffold and inspect the control plane
openrustclaw control init
openrustclaw control describe --json

# Start the agent with your configured channels
openrustclaw start --channels=telegram,discord,slack
```

## Architecture

```
                         ┌─────────────────────────┐
                         │          CLI             │
                         └────────────┬─────────────┘
        ┌─────────────┬──────────────┼──────────────┬─────────────┐
        │  Channels   │    Voice     │   Canvas     │   Cursor    │
        │ (15 current)│  Wake/STT/TTS│   A2UI       │  IDE (ACP)  │
        └──────┬──────┴──────┬───────┴──────┬───────┴──────┬──────┘
               └─────────────┴──────────────┴──────────────┘
                         ┌──────────┴──────────┐
                         │   Gateway (Axum WS) │
                         └──────────┬──────────┘
        ┌──────────┬────────────┬───┴───┬────────────┬────────────┐
        │  Agent   │  Memory    │ Skills│ Scheduler  │  Security  │
        │ Runtime  │  3-Tier    │ WASM  │ Durable    │ SSO/JWT    │
        └────┬─────┴─────┬──────┴───┬───┴─────┬──────┴─────┬──────┘
             │           │          │         │            │
        ┌────┴───┐  ┌────┴───┐  ┌──┴──┐  ┌───┴────┐  ┌───┴──────┐
        │Providers│  │   DB   │  │ MCP │  │Langbrdg│  │Observabil│
        │ 20 LLMs │  │SQLite  │  │JSON │  │ gRPC   │  │OTel/Prom │
        └─────────┘  └────────┘  │-RPC │  └───┬────┘  └──────────┘
                                 └─────┘      │
                                    ┌─────────┴─────────┐
                                    │ Python Sidecar    │
                                    │ Optional Compat   │
                                    └───────────────────┘
```

**Crate dependency order:**
`core` -> `db` -> `memory`, `providers`, `mcp`, `observability`, `security` -> `agent` -> `gateway`, `channels` -> `skills`, `scheduler` -> `langbridge` -> `cli`

## LLM Providers

20 providers with native SDK crates. All keys protected with `secrecy::SecretString`, all HTTP clients configured with connect/request timeouts.

| Provider | Crate | Streaming | Tool Use |
|----------|-------|-----------|----------|
| Anthropic (Claude) | `anthropic-rust` | Yes | Yes |
| OpenAI (GPT-4o) | `async-openai` | Yes | Yes |
| Google Gemini | `google-gemini` | Yes | Yes |
| OpenRouter | `openrouter-api` | Yes | Yes |
| AWS Bedrock | `aws-bedrock` | Yes | Yes |
| Azure OpenAI | `azure-openai` | Yes | Yes |
| Ollama (local) | `ollama-sdk` | Yes | Yes |
| Mistral | `mistral` | Yes | Yes |
| Cohere | `cohere` | Yes | Yes |
| Groq | `groq` | Yes | Yes |
| DeepSeek | `deepseek` | Yes | Yes |
| Together AI | `together-ai` | Yes | Yes |
| Fireworks AI | `fireworks-ai` | Yes | Yes |
| Replicate | `replicate` | Yes | Yes |
| Perplexity | `perplexity` | Yes | Yes |
| AI21 | `ai21` | Yes | Yes |
| Cloudflare AI | `cloudflare-ai` | Yes | Yes |
| vLLM | `vllm` | Yes | Yes |
| llama.cpp | `llama-cpp` | Yes | Yes |
| Ollama SDK | `ollama-sdk` | Yes | Yes |

Provider fallback chain with configurable cooldowns routes requests through available providers automatically.

## Messaging Channels

Current channel modules available in the repo:

Telegram, Discord, Slack, WhatsApp (Baileys bridge), Microsoft Teams, Mattermost, Google Chat, Google Meet, Gmail (Pub/Sub), Matrix, iMessage, Signal, LINE, Viber, WeChat, Messenger, Instagram DMs, WebChat

The current shipped startup path actively supports WebChat, Telegram, Discord, Slack, WhatsApp, Mattermost, Microsoft Teams, iMessage, Google Chat, Google Meet, Gmail Pub/Sub, Matrix, and Signal. Other channel modules remain in the repo but are gated or deferred from the shipped runtime surface.

Current tier-1 status:

- Telegram: auth probe, outbound send, Bot API polling plus webhook receive paths, native local-file uploads for media sends, reply threading, forum-topic metadata/topic targeting plus topic admin actions, poll handling, reactions, normalized inbound media/file-reference metadata, mention-aware group metadata, and local agent/session routing are implemented
- Discord: auth probe, native local-file uploads, outbound send, edit/reaction operations, verified Interactions HTTP ingress, Gateway `MESSAGE_CREATE`, `MESSAGE_UPDATE`, `MESSAGE_DELETE`, `TYPING_START`, and thread lifecycle receive paths, reconnect/session recovery, thread-aware session routing with parent-channel binding inheritance, thread-preferred replies, reply-reference propagation aliases, mention detection, attachment/embed metadata capture, forwarded-attachment local downloads plus file references, deferred acknowledgements, and local agent/session routing are implemented
- Slack: auth probe, outbound send, edit/reaction operations, built-in HTTP Events API ingress, Socket Mode websocket ingress with envelope ACKs, `events_api` plus slash-command and interactive payload routing, mention-aware routing metadata, thread ownership plus stream-mode metadata, draft-stream replies, native local-file uploads through Slack's external upload flow, attachment/file-reference metadata, download-action blocks, and local agent/session routing are implemented
- WhatsApp: Baileys bridge pairing/QR, inspectable QR/pairing state, first-class CLI operator flows for `status|connect|pair|send|send-media`, reconnect behavior, DM/group routing, mentions, replies, media send/receive, delivery acknowledgements, and local agent/session routing are implemented
- Mattermost: Rust-native outbound sends, slash-command and outgoing-webhook ingress, shared-token validation, user/channel allowlists, trigger-word stripping, bot-mention detection, threaded replies via `root_id`, REST-backed local file uploads from `file_references`, and local agent/session routing are implemented
- iMessage: BlueBubbles/macOS direct send, verified BlueBubbles ping on connect, webhook-password validation for BlueBubbles ingress, direct-handle or chat-guid targeting, structured group/chat mapping from webhook payloads, structured attachment metadata plus outbound local-file sends in BlueBubbles mode, tapbacks including normal channel-send reactions, first-class CLI operator flows for `ping|server|chats|contacts|send|send-file|tapback`, and local agent/session routing are implemented
- Google Chat: direct webhook ingress plus Pub/Sub push-envelope decoding, token-backed or service-account outbound auth, response-mode gating, normalized space/user IDs, slash-command metadata plus mention capture, message ID plus body/thread/argument/slash/mention presence metadata, identity and event-time presence flags, derived display-name/action/attachment count and length metadata, card-click and space lifecycle routing, card/file-reference responses, optional local attachment downloads with normalized `local_path` enrichment, first-class CLI operator flows for `status|connect|send|send-card|send-link-card`, and local agent/session routing are implemented for the shipped Google Chat workflow surface
- Google Meet: native Rust integration now covers space creation/inspection, active-conference termination, conference-record/participant/recording/transcript inspection, direct-event plus Google Workspace Events/Pub/Sub payload decoding with transcript hydration, live webhook ingress on `openrustclaw start`, and durable runtime-event publication for workflow automation; add-on UI embedding remains a later track
- Gmail Pub/Sub: Gmail watch setup and stop, Pub/Sub or direct-notification webhook ingress, history fetch, message hydration, structured attachment/file-reference metadata, normalized recipient/label counts plus recipient domains, message/thread/history/subject/body/received-at presence flags, recipient/domain/label/header presence flags, file-reference counts, attachment total-size metadata, address/domain length and attachment-name/MIME count metadata, unread and received-at metadata, parsed header/thread metadata, history/body-length metadata, subject/body/file-reference presence metrics, mail-triggered local agent/session routing, direct sends, threaded direct sends, replies including local-file attachments, metadata-driven label/archive/delete/forward actions, forwarded local-file attachments, and first-class CLI operator flows for `status|connect|process-notification|send|reply|label|archive|delete|forward` are implemented for the shipped Gmail workflow surface
- Matrix: access-token or password auth, `/sync` polling ingress, outbound room sends, reply/thread relations with explicit presence flags, format/formatted-body presence metadata, media-presence metadata, inbound reaction, redaction, and membership events with richer lifecycle metadata plus target/reason/avatar/display-name presence flags, reaction/redaction/membership/media length metadata, room/event/sender/reply/content-URI derived lengths, room join/leave, joined-room inspection, helper-driven plus normal-send local file uploads, inbound media downloads into the Matrix data directory with structured file-reference metadata plus media MIME/size fields and download/content-uri/filename/size/file-reference presence flags, first-class CLI operator flows for `join|leave|rooms|send-formatted|react|send-file|typing|redact`, and local agent/session routing are implemented; E2EE and advanced device-state management are tracked separately as a later surface
- Signal: `signal-cli` daemon-backed direct/group send-receive, allowlist handling, first-class CLI operator flows for register/verify/link/list-groups, normalized route metadata, source/recipient/group presence flags, text-length and attachment-size/file-reference metrics, source-number/group-name/quote-author length fields, outbound local-attachment sends, structured inbound attachment/file-reference metadata with daemon-provided local attachment path enrichment where available, attachment-only inbound routing, normalized attachment ids/names/mime types plus caption counts and attachment-id/name/MIME/caption presence flags, flat quote plus reply-target metadata with quote presence/text-length flags, sync-message lifecycle visibility, normalized group-member and mention metadata with presence flags, receipt lifecycle events, and local agent/session routing are implemented
- Teams: Bot Framework webhook ingress, JWT verification, connector-`/v3` outbound sends, reply aliases, native local-file uploads through Connector attachments, metadata-driven typing/update/delete actions, conversation/reaction/update/delete lifecycle routing with normalized member/reaction id/count metadata and presence flags, normalized team/channel/tenant IDs, richer message body/reply/update/delete flags, mention presence/count metadata, richer attachment metadata plus structured file references, attachment name/type/url presence flags, attachment URL counts, file-reference counts, local download paths, adaptive-card file links, and local agent/session routing are implemented

Channel routing/operator controls:

- `.claw/channels/` is the standard operator-visible registry for pending/approved accounts and channel bindings
- `openrustclaw channels init|list|show-account|create-account|delete-account|approve|block|activation|bind|show-binding|delete-binding` manages the shipped channel registry and binding policy
- `openrustclaw imessage ping|server|chats|contacts|send|send-file|tapback` exposes the shipped iMessage / BlueBubbles operator flows directly in the CLI
- `openrustclaw signal register|verify|link|list-groups` exposes the shipped Signal setup and operator flows directly in the CLI
- `GET/POST/PUT/DELETE /control/channels...` exposes the same shipped channel registry over typed HTTP so the future Control UI can reuse the runtime state model
- shipped channel routing now applies workspace/account/channel binding precedence, pairing approval gates, group mention activation, and shared reply chunking/coalescing/pacing policy
- shipped channel routing now normalizes workspace/scope/group/mention metadata across Teams, Google Chat, Matrix, and Signal as well, so the same binding and isolation rules apply across the newer shipped channels too
- `openrustclaw matrix join|leave|rooms|send-formatted|react|send-file|typing|redact` exposes the shipped Matrix operator flows directly in the CLI

## Control Plane

- `.claw/control/` is the standard operator-visible registry for:
  - agent profiles
  - model profiles
  - Claw manifests
  - solo/task/category/orchestrated runtime mode
  - autonomy tiers and execution guardrails
  - decision lessons for recurring routing/execution mistakes
  - task/category-to-Claw assignments
- `openrustclaw control init|list|show|validate|describe|create-agent|create-model|create-claw|mode|lessons|add-lesson|deactivate-lesson|assign-task|assign-category`
- shared typed control surfaces now exist at:
  - `GET /control/ui`
  - `GET /control/runtime`
  - `GET /control/autonomy`
  - `GET/POST /control/autonomy/lessons`
  - `POST /control/autonomy/lessons/{id}/deactivate`
  - `GET/PUT /control/config`
  - `POST /control/config/validate`
  - `GET /control/diagnostics`
  - `GET /control/diagnostics/ws`
  - `GET /control/logs/recent`
  - `GET /control/logs/ws`
- shared service diagnostics surfaces now exist at:
  - `GET /control/services/status`
  - `GET /control/services/scheduler`
  - `GET /control/services/runtime-events`
  - `GET /control/services/channels`
- shared operator inspection surfaces now exist at:
  - `GET /control/sessions`
  - `GET /control/sessions/{id}`
  - `GET /control/memory/namespaces`
  - `GET /control/memory/timeline`
  - `GET /control/memory/archive`
  - `GET /control/jobs`
  - `GET /control/jobs/{id}`
- shared runtime reconfiguration surfaces now exist at:
  - `GET /control/runtime/status`
  - `POST /control/runtime/reload`
  - `POST /control/runtime/switch-provider`
  - `POST /control/runtime/switch-model`
  - `GET /control/runtime/vault`
  - `PUT /control/runtime/vault/{key}`
  - `DELETE /control/runtime/vault/{key}`
- `openrustclaw runtime status|reload|switch-provider|switch-model`
- `openrustclaw runtime services status|scheduler|events|channels`
- `openrustclaw runtime services logs`
- `openrustclaw runtime vault status|list|set|delete`
- `openrustclaw orchestrate resolve|run|submit`
- `openrustclaw orchestrate list|inspect|trace|transcript|resources|promote-candidate`
- `openrustclaw orchestrate active|watch|pause|resume|kill`
- runtime secret sources now load from workspace `.env` and an encrypted `.claw/control/runtime-vault.json` when `OPENRUSTCLAW_VAULT_PASSPHRASE` is set
- provider/model cutovers are validated before config writes, runtime API cutovers roll back on failed reload, and config writes create timestamped backup files
- bounded orchestration surfaces now exist at:
  - `POST /control/orchestration/resolve`
  - `POST /control/orchestration/run`
  - `POST /control/orchestration/submit`
  - `GET /control/orchestration/active`
  - `GET /control/orchestration/active/{run_id}`
  - `GET /control/orchestration/active/{run_id}/events`
  - `POST /control/orchestration/active/{run_id}/pause`
  - `POST /control/orchestration/active/{run_id}/resume`
  - `POST /control/orchestration/active/{run_id}/kill`
  - `GET /control/orchestration/runs`
  - `GET /control/orchestration/runs/{receipt_id}`
  - `GET /control/orchestration/runs/{receipt_id}/checkpoints`
  - `GET /control/orchestration/runs/{receipt_id}/supervision`
  - `GET /control/orchestration/runs/{receipt_id}/trace`
  - `GET /control/orchestration/runs/{receipt_id}/transcript`
  - `GET /control/orchestration/runs/{receipt_id}/resources`
- `POST /control/orchestration/runs/{receipt_id}/reflection-candidates/{index}/promote`
- orchestration now carries runtime autonomy policy, lesson-aware steering notes, runtime-capped delegation limits, selective quality-review loops, and bounded worker follow-up turns into execution without turning the outer loop into a constant micromanager
- the same bounded resolve/run surfaces now accept request-scoped overrides for selected model profile, worker model profile, autonomy level, delegation/iteration/runtime caps, and approval policy, so one run can shift lanes without mutating the durable control registry
- orchestration receipts now also preserve planner/worker/synthesis trace entries, full parent/child transcript entries, parent/child delegation relationships, and estimated token/duration summaries so operators can inspect execution flow and resource shape without over-steering the models themselves
- reflection candidates can now be promoted into scoped decision lessons through CLI, API, or `/control/ui`, so decision learning stays explicit and operator-auditable
- routed/orchestrated runs now persist receipts under `.claw/control/orchestration-runs/` and active supervision state under `.claw/control/orchestration-active/` so operators can inspect which Claw planned, which Claws executed, which model-profile fallback path was used, which checkpoints were hit, which trace edges connected the run, what the live worker state is, and which autonomy/reflection context shaped the run
- the shipped `/control/ui` dashboard now reuses the same typed runtime/config/diagnostics/services/session/memory/job/channel/orchestration/browser APIs for an initial browser-based operator shell, including live diagnostics, live runtime logs, enabled-channel readiness inspection, service/scheduler/runtime-event inspection, session/memory/job inspection, config validate/apply, vault key set/delete, autonomy-policy/decision-lesson inspection, active orchestration watch/pause/resume/kill controls, orchestration run supervision/trace/transcript/resource detail, and bounded browser actions
- onboarding now detects existing workspace state, offers keep/modify/reset-with-backup choices, supports QuickStart vs Advanced paths, and finishes with a `doctor`-backed health handoff instead of stopping at raw config writes
- the browser operator lane now includes read-first HTTP fetch and bounded same-domain crawl actions, so operators can gather page/site context without invoking a full browser session for every task
- `openrustclaw doctor --repair --deep --non-interactive` now validates and, where safe, scaffolds the shipped control-plane registry while exposing the same typed diagnostic model used by `/control/diagnostics`
- the runtime syncs `.claw/control/CLAW_RUNTIME.md` so Claw itself can see whether it is running solo or alongside other Claws and what delegation/isolation policies exist

## Memory System

Three-tier architecture -- no full memory files injected into prompts:

- **Core Memory** (~500 tokens, always loaded) -- persistent user/system facts
- **Recall Memory** (on-demand search) -- hybrid BM25 + vector similarity + temporal decay
- **RAG Context** (budgeted assembly) -- deterministic retrieved context with stable source ids for citations, Rust-backed durable chunk storage, configurable retrieval controls for `top_k`, preferred/required/excluded sources and source types, minimum-overlap/minimum-score filters, plus score-aware and metadata-aware context shaping
- **Archive Memory** (consolidated) -- long-term storage with Rust-backed maintenance that persists summaries and removes archived recall entries
- **Durable Sessions** -- persisted sessions and conversation history with operator list/show/spawn/send/archive/close controls over CLI and MCP
- **File-backed Views** -- `.claw/memory/views/` exports for core, recall, archive, persona, and runtime-ledger inspection/edit flows
- **Model-aware Artifacts** -- Rust-native registry for `AGENTS.md`, `AI.md`, `CONTEXT.md`, `ARCHITECTURE.md`, `CLAUDE.md`, `GEMINI.md`, Copilot/Cursor/Continue rules, persona files, memory files, and `Modelfile`, with precedence-aware prompt resolution and preferred-target sync

```toml
[memory]
core_max_tokens = 500
recall_search_limit = 20
archive_after_days = 30

[sidecar]
role = "compatibility" # compatibility, experimental, disabled
auto_start = false
```

## MCP (Model Context Protocol)

Both client and server support over JSON-RPC stdio transport:

```bash
# Expose tools to MCP clients
openrustclaw mcp-server
```

Command allowlist enforced on subprocess spawning (`npx`, `uvx`, `node`, `python3`, `docker`, `deno`, `bun`, `cargo`, `go`).

The MCP server also exposes optimization tools for:

- listing and registering optimization targets
- submitting and inspecting optimization candidates
- running bounded optimization candidates in isolated temp workspaces
- recording approve/reject/promote decisions

## Autonomous Optimization

Phase 2.5 is implemented in `crates/optimization`.

```bash
# List targets
openrustclaw optimize list-targets

# Register a target
openrustclaw optimize register-target \
  --name prompt.optimize \
  --kind prompt_policy \
  --tier rust_native \
  --allowed-path prompt.txt \
  --eval "verify=bash -lc 'grep -q optimized prompt.txt'"

# Submit a candidate from a JSON change set
openrustclaw optimize submit-candidate \
  --target prompt.optimize \
  --hypothesis "shorter prompt improves quality" \
  --change-set changes.json

# Run the bounded experiment
openrustclaw optimize run-candidate <candidate-id>
```

The optimization subsystem persists targets, candidates, evaluations, and promotion history in SQLite, enforces allowlists and diff budgets, runs evals in temporary workspaces, and records promotion decisions for Tier C, Tier B, or human-reviewed Tier A merge queues.

## Security

Defense-in-depth across every layer:

| Layer | Mechanism |
|-------|-----------|
| **Authentication** | JWT with session tracking, Enterprise SSO (OIDC/SAML) |
| **API Keys** | `secrecy::SecretString` -- zeroized on drop, redacted in logs |
| **Transport** | Origin validation on all WebSocket connections; token auth enabled by default |
| **Webhooks** | HMAC-SHA256 with constant-time comparison, Stripe replay protection |
| **Sessions** | Filesystem isolation with path traversal prevention |
| **Skills** | Workspace and marketplace skill lifecycle is wired through the CLI with signature-state tracking, validated/sensitive capability metadata, unsigned-sensitive-skill rejection, verification-policy summaries, and a no-import WASM sandbox executor with timeout, memory limits, capability-gated execution helpers, sensitive-capability helpers, and verification-aware declared-capability policy checks |
| **Input** | Prompt injection detection (34+ patterns), canary tokens |
| **Network** | SSRF prevention on OIDC/SAML endpoints (private IP rejection) |
| **Subprocess** | MCP command allowlist, shell metacharacter rejection |
| **Audit** | Structured audit events with severity levels |

See [SECURITY.md](SECURITY.md) for the full security policy.

## Voice

```bash
openrustclaw talk --wake-word "Hey Assistant"
```

Wake word detection (Porcupine), speech-to-text (Whisper), text-to-speech (OpenAI, ElevenLabs), continuous talk mode. Audio dependencies are feature-gated behind `audio`.
Inbound voice-note transcription can be enabled on the runtime path for supported channel attachments that already expose a local file path or a directly fetchable media URL, and operators now also have explicit `openrustclaw voice status|transcribe` plus `/control/voice/status|transcribe` surfaces for inspection and ad hoc transcription.

## Configuration

```toml
[llm]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[channels]
telegram = { enabled = true, token = "${TG_TOKEN}" }
discord  = { enabled = true, token = "${DISCORD_TOKEN}" }

[security]
require_auth = true
allowed_origins = ["https://app.example.com"]

[voice]
enabled = true

[voice.stt]
provider = "openai"
model = "whisper-1"
language = "auto"
transcribe_inbound_notes = true
download_dir = "./data/voice/inbound"

[voice.tts]
provider = "openai"
model = "tts-1"
voice = "alloy"
```

## Building and Testing

```bash
# Build
cargo build --workspace

# Test (769 tests)
cargo test --workspace

# Lint
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --all -- --check

# Build without optional subsystems
cargo build -p openrustclaw-cli --no-default-features
```

### Feature Flags

Heavy dependencies are opt-in:

| Crate | Feature | Dependencies |
|-------|---------|-------------|
| `distributed` | `raft-consensus`, `etcd`, `consul`, `redis`, `mdns` | Raft, etcd-client, Consul, Redis, mDNS |
| `voice` | `audio` | cpal, rodio, rubato, hound |
| `cli` | `voice`, `cursor` (default on) | Voice subsystem, Cursor IDE integration |
| `mobile` | `ios`, `android` | Platform-specific FFI |
| `automation` | `chrome` | headless_chrome |

## Project Structure

```
crates/
  core/          # Types, traits, error hierarchy (thiserror)
  db/            # SQLite via sqlx, libSQL for vectors, rusqlite for CLI
  memory/        # 3-tier memory system with policies and search
  providers/     # LLM provider trait + Anthropic/OpenAI/Gemini/OpenRouter/Ollama
  mcp/           # MCP client/server over JSON-RPC stdio
  agent/         # Agent runtime with tool execution loop
  gateway/       # Axum WebSocket server with auth, sessions, and webhook integrations
  channels/      # 20 messaging channel integrations
  skills/        # Skill registry, loader, marketplace, and WASM sandbox
  scheduler/     # Durable job scheduling, workflow runtime, and eventing
  optimization/  # Rust-native autonomous optimization framework
  security/      # Auth, SSO, isolation, input sanitization, skill verification
  langbridge/    # Compatibility bridge to optional Python sidecar
  observability/ # OpenTelemetry, Prometheus metrics, tracing
  cli/           # Terminal UI (ratatui), all CLI commands
  + 16 native LLM SDK crates
  + canvas, cursor, voice, automation, mobile, distributed
sidecar/         # Optional compatibility workflows for migration/experiments
```

## Contributing

```bash
git clone https://github.com/aihxp/OpenRustClaw.git
cd OpenRustClaw
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

All async code uses tokio. Errors use `thiserror` for libraries, `anyhow` for the CLI. Logging via `tracing` macros (`info!`, `warn!`, `error!`) -- never `println!` outside the CLI crate. See [CLAUDE.md](CLAUDE.md) for the full conventions guide.

## License

MIT -- see [LICENSE](LICENSE).

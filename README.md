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
The roadmap further includes model-aware artifact sync, memory rehydration on model swaps, multi-model routing with optional quarterback/orchestrator models, discoverable multi-claw execution modes, a self-configuration harness, and a control-plane fallback model so provider/model changes do not take the platform down.
The model-awareness track now also includes a canonical instruction/context artifact registry so OpenRustClaw can understand and translate common project guidance files like `AGENTS.md`, `AI.md`, `CONTEXT.md`, `ARCHITECTURE.md`, `CLAUDE.md`, `GEMINI.md`, Copilot instruction files, Cursor/Continue rules, and local open-weight `Modelfile` packaging.
The roadmap also now calls for onboarding-time provider/model scans and role-aware recommendations: Groq for low-latency core runtime use, OpenRouter for broad fallback/control-plane coverage, SiliconFlow for higher-capability secondary routing, and Ollama as the local/offline safety net, all validated against user-supplied keys rather than hardcoded assumptions.
The operator roadmap also now explicitly includes an OpenClaw-inspired onboarding journey, typed user configuration/settings flows, solo-versus-multi-claw setup choices, and a stronger `doctor` repair/migration surface rather than leaving these as ad hoc setup utilities.
The shipped control-plane surface now includes a file-backed `.claw/control/` registry for agent profiles, model profiles, Claw manifests, runtime-mode/task/category bindings, runtime self-description, and MCP/CLI inspection tools; the future Web Control UI is expected to reuse this registry instead of inventing a separate state model.

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

Telegram, Discord, Slack, WhatsApp (Baileys bridge), Microsoft Teams, Google Chat, Gmail (Pub/Sub), Matrix, iMessage, LINE, Viber, WeChat, Messenger, Instagram DMs, WebChat

The current shipped startup path actively supports WebChat, Telegram, Discord, Slack, WhatsApp, iMessage, Google Chat, Gmail Pub/Sub, Matrix, and Signal, and it also allows Microsoft Teams on a partial shipped path. Other channel modules remain in the repo but are gated or deferred from the shipped runtime surface.

Current tier-1 status:

- Telegram: auth probe, outbound send, Bot API polling receive, reply threading, forum-topic metadata/topic targeting plus topic admin actions, poll handling, reactions, mention-aware group metadata, and local agent/session routing are implemented
- Discord: auth probe, outbound send, edit/reaction operations, verified Interactions HTTP ingress, Gateway `MESSAGE_CREATE` receive, reconnect/session recovery, thread-aware session routing with parent-channel binding inheritance, thread-preferred replies, reply-reference propagation aliases, mention detection, attachment/embed metadata capture, forwarded-attachment local downloads plus file references, and local agent/session routing are implemented; deeper gateway polish still remains
- Slack: auth probe, outbound send, edit/reaction operations, built-in HTTP Events API ingress, mention-aware routing metadata, thread ownership plus stream-mode metadata, draft-stream replies, attachment/file-reference metadata, download-action blocks, and local agent/session routing are implemented for HTTP mode; Socket Mode remains incomplete
- WhatsApp: Baileys bridge pairing/QR, reconnect behavior, DM/group routing, mentions, replies, media send/receive, and local agent/session routing are implemented
- iMessage: BlueBubbles/macOS direct send, BlueBubbles inbound webhook routing, tapbacks including normal channel-send reactions, group/participant metadata normalization, structured attachment metadata, and local agent/session routing are implemented; richer group/contact mapping still remains
- Google Chat: direct webhook ingress plus Pub/Sub push-envelope decoding, token-backed or service-account outbound auth, response-mode gating, normalized space/user IDs, slash-command metadata plus mention capture, message body/thread/argument presence metadata, card-click and space lifecycle routing, card action parameter plus form-input metadata, event-time/user-email metadata, card/file-reference responses, structured attachment/file-reference metadata plus attachment-name/type/data-ref capture, and local agent/session routing are implemented; richer operator/media parity still remains
- Google Meet: native Rust operator integration now covers space creation/inspection, active-conference termination, conference-record/participant/recording/transcript inspection, and Google Workspace Events/Pub/Sub payload decoding with transcript hydration; add-on UI/runtime embedding remains a later track
- Gmail Pub/Sub: Gmail watch setup, Pub/Sub webhook ingress, history fetch, message hydration, structured attachment/file-reference metadata, normalized recipient/label counts plus recipient domains, recipient/label/header presence flags, file-reference counts, attachment total-size metadata, unread and received-at metadata, parsed header/thread metadata, history/body-length metadata, attachment-name/MIME presence metadata, mail-triggered local agent/session routing, direct sends, replies including local-file attachments, label/archive/delete actions, and forwarding are implemented; richer operator surfaces still remain
- Matrix: access-token or password auth, `/sync` polling ingress, outbound room sends, reply/thread relations with explicit presence flags, formatted-body and media-presence metadata, inbound reaction, redaction, and membership events with richer lifecycle metadata, room join/leave, joined-room inspection, helper-driven plus normal-send local file uploads, inbound media downloads into the Matrix data directory with structured file-reference metadata plus media MIME/size fields, and local agent/session routing are implemented; deeper E2EE and richer operator parity still remain
- Signal: `signal-cli` daemon-backed direct/group send-receive, allowlist handling, registration/verify/link helpers, normalized route metadata, outbound local-attachment sends, structured inbound attachment/file-reference metadata, attachment-only inbound routing, normalized attachment ids/names/mime types plus caption counts, flat quote plus reply-target metadata with quote presence/text-length flags, sync-message lifecycle visibility, normalized group-member and mention metadata with presence flags, receipt lifecycle events, and local agent/session routing are implemented; richer operator UX and inbound media download parity still remain
- Teams: Bot Framework webhook ingress, JWT verification, outbound sends, reply aliases, conversation/reaction/update/delete lifecycle routing with normalized member id/count metadata, normalized reaction counts/types, normalized team/channel/tenant IDs, richer message body/reply/update/delete flags, richer attachment metadata plus structured file references, attachment URLs, file-reference counts, and local download paths, adaptive-card file links, and local agent/session routing are on the shipped runtime path, but it is still partial relative to the tier-1 surfaces

Channel routing/operator controls:

- `.claw/channels/` is the standard operator-visible registry for pending/approved accounts and channel bindings
- `openrustclaw channels init|list|approve|block|activation|bind` manages tier-1 channel pairing state and binding policy
- `GET/POST /control/channels...` exposes the same shipped channel registry over typed HTTP so the future Control UI can reuse the runtime state model
- shipped channel routing now applies workspace/account/channel binding precedence, pairing approval gates, group mention activation, and shared reply chunking/coalescing/pacing policy
- shipped channel routing now normalizes workspace/scope/group/mention metadata across Teams, Google Chat, Matrix, and Signal as well, so the same binding and isolation rules apply to the newer partial-runtime channels

## Control Plane

- `.claw/control/` is the standard operator-visible registry for:
  - agent profiles
  - model profiles
  - Claw manifests
  - solo/task/category/orchestrated runtime mode
  - task/category-to-Claw assignments
- `openrustclaw control init|list|show|validate|describe|create-agent|create-model|create-claw|mode|assign-task|assign-category`
- `openrustclaw doctor --repair --deep` now validates and, where safe, scaffolds the shipped control-plane registry
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
wake_word = "Hey Assistant"
stt_provider = "whisper"
tts_provider = "openai"
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

# OpenRustClaw vs OpenClaw Parity Matrix

This is the source-backed parity inventory for the current OpenRustClaw program.

It maps documented OpenClaw user-facing features to the current OpenRustClaw state, the owning code, representative tests, local docs, and the severity of the remaining gap.

Reviewed against official OpenClaw sources on 2026-03-18:

- [OpenClaw docs home](https://docs.openclaw.ai/)
- [OpenClaw features](https://docs.openclaw.ai/concepts/features)
- [OpenClaw CLI reference](https://docs.openclaw.ai/cli)
- [OpenClaw multi-agent routing](https://docs.openclaw.ai/concepts/multi-agent)
- [OpenClaw streaming and chunking](https://docs.openclaw.ai/concepts/streaming)
- [OpenClaw plugin agent tools](https://docs.openclaw.ai/plugins/agent-tools)
- [OpenClaw Deepgram provider](https://docs.openclaw.ai/providers/deepgram)
- [OpenClaw site](https://openclaw.ai/)
- [OpenClaw repository](https://github.com/openclaw/openclaw)

## Status Keys

- `matched`: practical parity exists today.
- `partial`: some of the documented OpenClaw behavior exists, but operators would notice gaps.
- `gated`: code exists or a crate exists, but it is intentionally not part of the shipped surface.
- `missing`: no meaningful parity path exists yet.
- `stronger-divergence`: OpenRustClaw intentionally differs and is stronger in a meaningful way.
- `out-of-scope`: intentionally not a parity target right now.

## Scope Keys

- `core`: part of the target parity surface.
- `plugin`: extension/plugin parity target.
- `divergence`: intentional product difference to preserve.
- `out-of-scope`: explicitly not in the near-term parity target.

## Matrix

| OpenClaw feature family | OpenClaw documented surface | Scope | ORC status | Owner | Representative tests | Local docs | Gap severity | Notes / next slice |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Gateway process | Single gateway process as source of truth for sessions, routing, and channel connections | core | matched | `crates/gateway`, `crates/cli`, `crates/agent` | `tests/e2e/tests/vertical/test_gateway_layer.rs`, `tests/integration/src/gateway_test.rs` | `README.md`, `docs/src/architecture/overview.md` | low | Core runtime exists and is usable end to end |
| Multi-agent routing | Isolated sessions per agent, sender, workspace, and bindings | core | partial | `crates/cli/src/commands/start.rs`, `crates/core`, `crates/agent` | `tests/e2e/src/test_chat_workflow.rs`, `tests/e2e/src/test_security_workflow.rs` | `docs/roadmap.md`, `docs/feature-matrix.md` | high | Session reuse and thread-aware routing exist, but full account/workspace binding parity is still open |
| Session tools | Inspect, create, route, and manage sessions from operator surfaces and tools | core | partial | `crates/core`, `crates/agent`, `crates/cli`, `crates/mcp` | `tests/e2e/tests/vertical/test_gateway_layer.rs`, `tests/e2e/src/test_chat_workflow.rs` | `docs/roadmap.md` | high | Session runtime exists; session-tool parity is still missing |
| Direct vs group session policy | Direct chats can collapse into `main`; groups isolate; thread scope can override channel scope | core | partial | `crates/cli/src/commands/start.rs`, `crates/channels/src/discord.rs` | `crates/cli/src/commands/start.rs` unit tests, `tests/e2e/src/test_chat_workflow.rs` | `docs/roadmap.md`, `docs/feature-matrix.md` | medium | Thread-aware scope exists for Discord, but policy parity is not complete across channels |
| Streaming and chunking | Preview streaming, block streaming, coalescing, pacing | core | partial | `crates/agent`, `crates/gateway`, `crates/channels` | `tests/e2e/src/test_chat_workflow.rs`, `tests/e2e/tests/horizontal/test_chat_journey.rs` | `docs/src/operations/observability.md`, `docs/roadmap.md` | high | Streaming exists, but OpenClaw-style per-channel chunking modes are not fully implemented |
| WhatsApp | WhatsApp Web / pairing / group routing / media | core | gated | `crates/channels` | crate-local scaffolding only | `README.md`, `docs/feature-matrix.md`, `docs/roadmap.md` | critical | Present in repo, not shipped; major parity gap |
| Telegram | Bot auth, send/receive, routing | core | matched | `crates/channels/src/telegram.rs`, `crates/cli/src/commands/start.rs` | `crates/channels/src/telegram.rs`, `cargo test -p openrustclaw-channels telegram` | `README.md`, `docs/feature-matrix.md` | low | Current tier-1 parity path is real |
| Discord | Interactions, gateway message ingress, replies, threads, routing | core | partial | `crates/channels/src/discord.rs`, `crates/cli/src/commands/start.rs` | `crates/channels/src/discord.rs`, `cargo test -p openrustclaw-channels discord` | `README.md`, `docs/feature-matrix.md` | medium | Core runtime exists; richer gateway/runtime polish still remains |
| iMessage | Local bridge, send/receive, attachments | core | gated | `crates/channels` | crate-local scaffolding only | `README.md`, `docs/feature-matrix.md`, `docs/roadmap.md` | critical | Repo surface exists, but no shipped runtime path |
| Mattermost plugin channel | Plugin-based channel extension | plugin | missing | none shipped | none | `docs/roadmap.md`, `docs/parity-positioning.md` | high | OpenClaw documents plugin-channel support; OpenRustClaw needs a Rust-native extension/channel model |
| Slack / extra operator channels | Slack and additional channel accounts via CLI | core | partial | `crates/channels/src/slack.rs`, `crates/cli` | `crates/channels/src/slack.rs`, `cargo test -p openrustclaw-channels slack` | `README.md`, `docs/feature-matrix.md` | medium | Slack HTTP mode is real; broader account CRUD parity is still open |
| Media support | Images, audio, and documents in and out | core | partial | `crates/channels`, `crates/gateway`, `crates/automation` | limited channel and gateway coverage | `README.md`, `docs/roadmap.md` | high | Attachment metadata exists for Discord, but full media parity is not done |
| Voice-note transcription | Provider-backed audio transcription in message flows | core | partial | `crates/voice`, media/provider surfaces | feature-gated voice tests only | `README.md`, `docs/roadmap.md` | high | Voice subsystem exists, but OpenClaw-style inbound audio transcription parity is not complete |
| Web Control UI | Browser dashboard for chat, config, sessions, and nodes | core | missing | no shipped UI runtime | none | `docs/roadmap.md`, `docs/parity-positioning.md` | critical | This is one of the largest parity gaps |
| Mobile nodes | iOS/Android nodes, pairing, Canvas, device commands | core | gated | `crates/mobile`, adjacent feature crates | none representative for parity | `README.md`, `docs/feature-matrix.md`, `docs/roadmap.md` | critical | Repo surface exists, but operator parity does not |
| Onboarding wizard | Guided setup and service install | core | partial | `crates/cli/src/commands/onboard.rs`, `crates/cli` | CLI parsing tests in `crates/cli/src/main.rs` | `README.md`, `docs/roadmap.md` | medium | `onboard` exists, but OpenClaw-style end-to-end channel account setup parity is incomplete |
| Channel account management | `channels add`, pairing approve, account-scoped routing | core | partial | `crates/cli`, `crates/channels` | current CLI parse coverage only | `docs/roadmap.md`, `docs/feature-matrix.md` | high | Needs full CRUD + pairing + routing parity |
| Memory system | Core/recall/archive memory with search and maintenance | core | matched | `crates/db`, `crates/memory`, `crates/gateway` | `tests/e2e/src/test_memory_workflow.rs`, `tests/e2e/tests/regression/test_memory_recall.rs` | `README.md`, `docs/src/guides/memory.md`, `docs/feature-matrix.md` | low | Strong practical parity, with Rust-owned durability |
| RAG and context | Retrieved context, budgeting, stable source handling | core | partial | `crates/db`, `crates/gateway`, `sidecar/src/workflows/rag_pipeline.py` | `sidecar/test_sidecar.py`, gateway/RAG tests | `README.md`, `docs/feature-matrix.md`, `docs/roadmap.md` | medium | Durable lexical path exists; richer indexing and retrieval quality still open |
| Plugin agent tools | Plugin-provided tool surface exposed to agent runs | plugin | partial | `crates/skills`, `crates/mcp`, `crates/agent` | `crates/skills/src/loader.rs`, `crates/skills/src/sandbox.rs` | `docs/src/guides/skills.md`, `docs/roadmap.md` | high | Skills/tool injection exist, but plugin parity is not complete |
| Skills / extension runtime | Signed extension lifecycle, capability gating, safe execution | divergence | stronger-divergence | `crates/skills`, `crates/security` | `crates/skills/src/loader.rs`, `crates/skills/src/sandbox.rs`, `crates/security/src/skill_verifier.rs` | `README.md`, `docs/src/guides/skills.md`, `docs/parity-positioning.md` | low | Intentional stronger Rust-native capability enforcement |
| Provider auth extensions | OpenClaw docs mention provider auth integrations such as OAuth-backed providers | core | partial | `crates/providers`, `crates/security/src/sso` | provider tests and security tests | `README.md`, `docs/src/guides/providers.md` | medium | Multi-provider support is stronger overall, but some OpenClaw-specific auth UX parity is not complete |
| Security and pairing controls | Tokens, allowlists, origin checks, pairing approval, safety controls | core | partial | `crates/security`, `crates/gateway`, `crates/channels`, `crates/cli` | `tests/e2e/src/test_security_workflow.rs`, `tests/integration/src/security_test.rs` | `README.md`, `SECURITY.md`, `docs/src/guides/security.md` | medium | Security posture is strong; pairing-approval parity across channels is still open |
| MCP operator surface | MCP server and MCP-friendly tool access | divergence | stronger-divergence | `crates/mcp`, `crates/mcp2cli`, `crates/cli` | `tests/integration/src/mcp_test.rs`, `cargo test -p openrustclaw-cli mcp_server` | `README.md`, `docs/src/guides/mcp-servers.md`, `docs/parity-positioning.md` | low | OpenRustClaw is intentionally stronger here than current OpenClaw docs imply |
| Observability and evals | Traces, diagnostics, runtime visibility | core | partial | `crates/observability`, `crates/gateway`, `crates/cli`, `sidecar` | `sidecar/test_sidecar.py`, observability crate tests | `docs/src/operations/observability.md`, `docs/feature-matrix.md` | medium | Major paths are traced; full operator-significant coverage still open |

## Scope Split

### Core parity targets

- Gateway runtime
- Multi-agent routing
- Sessions and session policy
- Streaming and chunking
- WhatsApp
- Telegram
- Discord
- iMessage
- Slack and channel account management
- Media support
- Voice-note transcription
- Web Control UI
- Mobile nodes
- Onboarding and pairing flows
- Memory and RAG
- Security and operator controls
- Observability for operator-significant paths

### Plugin parity targets

- Plugin channels such as Mattermost-style extension support
- Plugin agent tools
- Background workflow and tool-extension model parity

### Intentional divergences to preserve

- Rust-native skills/capability enforcement instead of a weaker plugin/runtime model
- MCP and `mcp2-cli` as first-class operator tooling
- Broader multi-provider model support than the current Pi-centric OpenClaw path

### Out of scope for current parity claims

- Full source-language parity with the Node/TypeScript OpenClaw implementation
- Keeping Python as a required production dependency forever
- Any undocumented OpenClaw internal behavior that is not exposed as user-facing documented surface

## Severity Guide

- `critical`: major documented OpenClaw capability absent from the shipped ORC surface
- `high`: capability exists in ORC but would materially disappoint an operator expecting documented OpenClaw behavior
- `medium`: usable but incomplete
- `low`: parity mostly achieved or ORC intentionally exceeds the requirement

## Phase 1 Completion Note

This file, together with [feature-matrix.md](feature-matrix.md), [parity-positioning.md](parity-positioning.md), and [roadmap.md](roadmap.md), is the contract for Phase 1.

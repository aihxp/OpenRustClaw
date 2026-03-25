# Parity Positioning

This page explains how to read OpenRustClaw's OpenClaw-parity work.

The goal is not to imitate OpenClaw's internals. The goal is to deliver the same operator-visible feature set, with Rust-first ownership where OpenRustClaw can do better.

## What We Already Match Well

- Rust-owned gateway runtime
- Core memory, recall memory, archive maintenance
- Durable scheduler semantics
- MCP stdio server and MCP operator tooling
- Shipped channel/runtime surface across Telegram, Discord, Slack, WhatsApp, Mattermost, iMessage, Google Chat, Google Meet, Gmail Pub/Sub, Matrix, Signal, and Teams
- Channel-scoped routing, pairing approval, session routing, and channel registry workflows
- Media/file-reference send and receive flows for the declared shipped channel set
- A bounded media operator lane over Anthropic, Gemini, Ollama, and OpenAI-compatible describe/extract surfaces
- A bounded voice-call lane with health, metrics, artifacts, events, reconnect, and stale-call handling over compiled-skill bindings
- A bounded Talk Mode lane with status, metrics, event-timeline inspection, live microphone capture, and speaker playback over the feature-gated runner
- A bounded mobile command lane with receipt metrics and event-timeline inspection over the shared typed Rust mobile command protocol
- A bounded mobile app-session lane with metrics and event-timeline inspection over the same heartbeat/runtime receipts
- A bounded mobile runtime lane with pairing, capability inventory, execution receipts, push/sync/notification/inbox/outbox, activity, and media-artifact receipts over the shared typed Rust mobile node model
- Skills install/update/verify lifecycle with real sandbox execution

## Where OpenRustClaw Is Intentionally Stronger

- Rust-native durability and process ownership rather than a JS-first gateway runtime
- Stronger skill capability enforcement and verification-aware execution controls
- Compiled skill help/schema artifacts plus explicit extension manifests so models and operators can route through cached summaries instead of re-reading raw skill files every turn
- Broader multi-provider model support
- First-class MCP and `mcp2-cli` workflows
- Stricter runtime/docs truthfulness and CI gates around shipped features
- A shipped Rust-native autonomous optimization framework instead of relying on a Python self-improvement loop as the product-level answer

These are not parity failures. They are product choices to preserve.

## How LangGraph Fits

LangGraph is not being rejected. It is being demoted from "possible permanent runtime brain" to a controlled role in the architecture:

- Rust-native execution is the long-term production target.
- Sidecar/LangGraph compatibility is acceptable for shipped flows still being migrated.
- LangGraph remains useful as the rapid experimentation lane and authoring/prototyping format for new workflow ideas.
- LangGraph is not part of the production-critical runtime path.

The key constraint is that Rust owns the durable outer loop:

- scheduler,
- event bus,
- retries,
- leases,
- checkpoints,
- operator inspection.

LangGraph may execute a bounded workflow run. It should not be the only durability boundary for the product.

## Remaining Intentional Differences

- Interactive live chat remains CLI/MCP-first instead of becoming a separate browser-chat parity requirement inside `/control/ui`
- OpenRustClaw ships Mattermost as a first-class Rust channel and uses compiled-skill channel extensions/background services as the extension substrate instead of copying OpenClaw's plugin-channel runtime shape
- The shipped provider-auth surface is the bounded OIDC auth-plugin lane; broader provider-native auth UX remains optional product expansion rather than a parity blocker
- Optional deferred channel scope such as Feishu/Lark remains out of the current declared parity target

These are intentional claim boundaries, not missing parity for the declared shipped surface.

## Recommended Finish Order

The remaining parity work is not being treated as "add every autonomous feature as fast as possible." The trust-first operator and orchestration waves are now the shipped baseline:

- operator-trust surfaces:
  - runtime logs,
  - diagnostics,
  - channel readiness,
  - onboarding/repair handoff;
- trust-first orchestration runtime:
  - full orchestrated multi-Claw execution,
  - active supervision with pause/resume/kill,
  - decision-quality loops that can learn from prior mistakes without over-steering strong models into brittle scripts.

From here, the preferred remaining order is:

- keep the richer Phase 7 mobile/runtime capability work explicit:
  - full mobile node runtime and broader device capability flows beyond the newly shipped bounded command lane,
  - deeper platform capability coverage beyond the newly shipped typed node protocol for command/control surfaces;
- keep the later channel auto-restart and broader operations hardening work explicit instead of blurring it into the already-shipped Wave 3 runtime reload/resilience surfaces.

This order is intentional. OpenRustClaw should get more autonomous only when operators can still see what it is doing, understand why it made a decision, and stop or steer it safely.

## What Counts As "Parity"

A feature only counts as parity-complete when:

- startup/config exists,
- runtime behavior works end to end,
- persistence semantics match operator expectations,
- representative tests exist,
- user-facing docs are accurate.

Repo scaffolding, partial crates, or hidden experimental code do not count.

## What Does Not Count As A Parity Failure

- OpenRustClaw using Rust where OpenClaw uses Node/TypeScript
- OpenRustClaw reducing or removing Python from the runtime path
- OpenRustClaw exposing stronger capability/sandbox limits
- OpenRustClaw adding operator tooling that OpenClaw does not currently document

## Current Claim Boundary

Today, OpenRustClaw can fairly claim:

- practical OpenClaw feature parity for the declared shipped surface,
- a parity matrix that is green for that declared target surface,
- Rust-owned production runtime ownership end to end,
- and stronger Rust-native foundations for durability, observability, and extension security.

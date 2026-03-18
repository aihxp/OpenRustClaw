# Parity Positioning

This page explains how to read OpenRustClaw's OpenClaw-parity work.

The goal is not to imitate OpenClaw's internals. The goal is to deliver the same operator-visible feature set, with Rust-first ownership where OpenRustClaw can do better.

## What We Already Match Well

- Rust-owned gateway runtime
- Core memory, recall memory, archive maintenance
- Durable scheduler semantics
- MCP stdio server and MCP operator tooling
- Telegram tier-1 runtime path
- Slack HTTP ingress runtime path
- Discord message ingress, interactions ingress, and thread-aware routing baseline
- Skills install/update/verify lifecycle with real sandbox execution

## Where OpenRustClaw Is Intentionally Stronger

- Rust-native durability and process ownership rather than a JS-first gateway runtime
- Stronger skill capability enforcement and verification-aware execution controls
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

## Where We Still Trail OpenClaw

- WhatsApp parity
- iMessage parity
- Web Control UI
- Mobile nodes and device-command flows
- Channel account management and pairing flows
- Session-tool parity
- Streaming/chunking parity across all channels
- Media in/out parity and voice-note transcription parity
- Plugin-channel parity such as Mattermost-style extensions

These are parity gaps and are tracked as open work in [parity-matrix.md](parity-matrix.md) and [roadmap.md](roadmap.md).

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

- a mostly honest shipped surface,
- meaningful parity progress across gateway, memory, scheduling, MCP, and tier-1 channels,
- stronger Rust-native foundations for durability and extension security.

It cannot yet claim:

- full OpenClaw feature parity,
- Control UI parity,
- mobile node parity,
- or all-Rust production ownership across every currently shipped orchestration path.

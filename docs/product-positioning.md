# Product Positioning

This page explains how to read OpenRustClaw's current product contract.

The goal is not to frame OpenRustClaw as a shadow of another runtime. The goal is to describe the
shipped operator surface clearly, explain the intentional product boundaries, and keep the runtime
truthful about what is and is not part of the supported path.

## What OpenRustClaw Ships Well

- Rust-owned gateway runtime and durable scheduling
- Core, recall, and archive memory with operator inspection
- Shared CLI, MCP, HTTP, and Control UI operator surfaces
- Multi-channel routing, pairing, and account/binding controls
- Media inspection, extraction, and description workflows
- Voice-note transcription, talk runtime inspection, and bounded voice-call hooks
- Mobile node pairing, receipts, summaries, and command execution flows
- Skills install/update/verify lifecycle with bounded Rust/WASM execution

## Where OpenRustClaw Is Intentionally Stronger

- Rust-owned durability and process ownership instead of a weaker compatibility-first runtime model
- Stronger skill capability enforcement and verification-aware execution controls
- Compiled skill help/schema artifacts and explicit extension manifests for model/operator routing
- Broader multi-provider model support
- First-class MCP and `mcp2-cli` workflows
- Tighter runtime/docs/test truthfulness and CI gates around the shipped surface

These are product choices to preserve, not liabilities to apologize for.

## Current Claim Boundaries

OpenRustClaw is intentionally opinionated about a few boundaries:

- interactive live chat remains CLI/MCP-first instead of becoming a separate browser-chat surface
- Mattermost is shipped as a first-class Rust channel rather than as a second plugin-channel runtime
- provider-auth support is the bounded OIDC auth-plugin lane over the shared runtime vault
- optional channel/workstation/cloud-agent expansion remains separate from the current shipped surface

These are intentional boundaries for the current product surface, not documentation gaps.

## How LangGraph Fits

LangGraph is retained as a controlled compatibility and experimentation lane:

- Rust-native execution is the long-term production target
- sidecar compatibility is acceptable for bounded flows still being migrated
- LangGraph remains useful for experimentation and authoring
- LangGraph is not the production-critical durability boundary

Rust owns:

- scheduler and event loop
- retries, leases, and checkpoints
- persistence
- operator inspection

## What Counts As Shipped

A feature only counts as part of the supported OpenRustClaw surface when:

- startup/config exists
- runtime behavior works end to end
- persistence semantics match operator expectations
- representative tests exist
- user-facing docs are accurate

Repo scaffolding, partial crates, or hidden experimental code do not count.

## What Does Not Count As A Failure

- using Rust where another ecosystem might use Node/TypeScript
- reducing or removing Python from the production path
- exposing stronger capability/sandbox limits
- adding operator tooling beyond the minimum documented surface

## Current Statement

Today, OpenRustClaw can fairly claim:

- a coherent shipped operator surface
- Rust-owned production runtime paths for the supported product contract
- stronger Rust-native foundations for durability, observability, and extension security
- explicit, documented claim boundaries where the product stays intentionally bounded

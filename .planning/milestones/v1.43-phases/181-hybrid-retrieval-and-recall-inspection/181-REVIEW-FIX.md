---
status: none_fixed
findings_in_scope: 1
fixed: 0
skipped: 1
iteration: 1
---

# Phase 181 Code Review Fix

Attempted to apply automatic fixes for Phase 181 review findings.

## Outcome

- `WR-01` was not auto-fixed.

## Skip Reasons

### WR-01: MCP `search_memory` still bypasses the hybrid embedding lane

This finding requires more than threading an existing handle through the MCP handler:

- `crates/cli/src/commands/start.rs` currently builds `GatewayState` with `embedding_service: None`
- there is no production `EmbeddingProvider` implementation wired into the runtime startup path
- the existing embedding-aware retrieval flows in the gateway and agent depend on a service that is not constructed for the MCP server today

That makes this a broader runtime integration task, not a safe blind auto-fix for a retroactive sweep. It should be handled in a dedicated follow-up change that introduces a real startup-time embedding provider and validates the MCP, gateway, and agent retrieval surfaces together.

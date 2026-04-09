---
status: findings
depth: standard
files_reviewed: 14
files_reviewed_list:
  - crates/agent/src/memory_tools.rs
  - crates/agent/src/tool_factory.rs
  - crates/agent/src/tools.rs
  - crates/app/src/memory_views.rs
  - crates/cli/src/commands/inspect.rs
  - crates/cli/src/commands/memory.rs
  - crates/cli/src/commands/start.rs
  - crates/core/src/types.rs
  - crates/db/src/memory_store.rs
  - crates/gateway/src/server.rs
  - crates/memory/src/context.rs
  - crates/memory/src/embeddings.rs
  - crates/memory/src/recall.rs
  - crates/scheduler/src/eventing.rs
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 181 Code Review

Standard review of the Phase 181 hybrid retrieval and bounded recall surfaces in:

- `crates/agent/src/memory_tools.rs`
- `crates/agent/src/tool_factory.rs`
- `crates/agent/src/tools.rs`
- `crates/app/src/memory_views.rs`
- `crates/cli/src/commands/inspect.rs`
- `crates/cli/src/commands/memory.rs`
- `crates/cli/src/commands/start.rs`
- `crates/core/src/types.rs`
- `crates/db/src/memory_store.rs`
- `crates/gateway/src/server.rs`
- `crates/memory/src/context.rs`
- `crates/memory/src/embeddings.rs`
- `crates/memory/src/recall.rs`
- `crates/scheduler/src/eventing.rs`

### WR-01: MCP `search_memory` still bypasses the hybrid embedding lane

**File:** `crates/cli/src/commands/start.rs:12477-12508`

**Issue:** The MCP `search_memory` handler constructs a `MemoryQuery` and then always calls `memory_store.search(&query)`, never `search_with_embedding`. Phase 181 explicitly scoped the shipped operator-facing search surfaces through the live vector-aware retrieval path, but this handler is still lexical-only while the agent tool and gateway internal API already use `embed_query()` plus `search_with_embedding()`. As a result, one of the exposed retrieval surfaces quietly reports degraded or unavailable vector state even when embeddings are configured, and operator/MCP behavior drifts from the hybrid path the phase promised.

**Fix:** Thread the embedding service into the MCP `search_memory` handler and use the same query-embedding fallback flow as `MemorySearchTool` and `internal_memory_search_handler`, so the MCP search surface exercises the hybrid lane instead of permanently staying lexical-only.

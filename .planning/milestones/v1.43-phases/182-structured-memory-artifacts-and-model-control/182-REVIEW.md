---
status: findings
depth: standard
files_reviewed: 12
files_reviewed_list:
  - crates/app/src/memory_views.rs
  - crates/cli/src/commands/inspect.rs
  - crates/cli/src/commands/memory.rs
  - crates/cli/src/commands/start.rs
  - crates/cli/src/main.rs
  - crates/core/src/types.rs
  - crates/db/src/memory_store.rs
  - crates/db/src/migrate.rs
  - crates/db/src/models.rs
  - crates/memory/src/lib.rs
  - crates/memory/src/model_artifacts.rs
  - crates/memory/src/policies.rs
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 182 Code Review

Standard review of the Phase 182 structured model-artifact and projection surfaces.

### WR-01: Projection sync swallows real core-memory delete failures

**File:** `crates/memory/src/model_artifacts.rs:128-136`

**Issue:** `ModelArtifactService::sync_projection()` removed every reserved core-memory slot with `let _ = ...remove(...)`, which discarded all errors, not just "key missing" cases. A real storage failure during projection refresh would therefore be reported as a successful promote/update while stale projected core-memory entries remained in place.

**Fix:** Ignore only `CoreMemory` not-found errors during reserved-key cleanup and propagate all other delete failures so artifact promotion/update fails loudly instead of leaving inconsistent projections behind.

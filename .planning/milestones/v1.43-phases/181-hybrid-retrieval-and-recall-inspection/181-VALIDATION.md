---
phase: 181
slug: hybrid-retrieval-and-recall-inspection
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-08
---

# Phase 181 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test |
| **Config file** | `Cargo.toml` workspace targets |
| **Quick run command** | `cargo test -p openrustclaw-db memory_store -- --nocapture` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~300 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p openrustclaw-db memory_store -- --nocapture` or the smallest targeted crate command for the touched seam
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 300 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 181-01-00 | 01 | 0 | RETR-01, RETR-02, RETR-03 | T-181-01-01 / T-181-01-03 | Wave 0 resolves the live vector path, explanation contract, and retrieval artifact taxonomy before hybrid changes lock in | unit | `cargo test -p openrustclaw-core --lib` | ✅ | ⬜ pending |
| 181-01-01 | 01 | 1 | RETR-01 | T-181-01-01 | Hybrid scoring preserves explicit lexical, vector, recency, confidence, and importance factors | unit + integration | `cargo test -p openrustclaw-db memory_store -- --nocapture` | ❌ W0 | ⬜ pending |
| 181-01-02 | 01 | 1 | RETR-01, RETR-03 | T-181-01-03 | Live caller path uses embeddings or reports degraded hybrid state truthfully | integration | `cargo test -p openrustclaw-agent memory_tools -- --nocapture` | ❌ W0 | ⬜ pending |
| 181-01-03 | 01 | 1 | RETR-02, RETR-04 | T-181-01-02 | Recall assembly stays bounded and does not inject raw recall/archive payloads into prompts | unit + integration | `cargo test -p openrustclaw-memory context -- --nocapture` | ❌ W0 | ⬜ pending |
| 181-01-04 | 01 | 1 | RETR-04 | T-181-01-02 | Prompt contract still describes and enforces the strict recall boundary | unit | `cargo test -p openrustclaw-agent prompt_describes_strict_memory_store_boundary -- --nocapture` | ✅ partial | ⬜ pending |
| 181-02-01 | 02 | 2 | RETR-02 | T-181-02-01 / T-181-02-03 | Recall view models carry bounded explanation metadata without raw blob exposure | unit | `cargo test -p openrustclaw-app memory_views -- --nocapture` | ❌ W0 | ⬜ pending |
| 181-02-02 | 02 | 2 | RETR-03 | T-181-02-02 | Inspect/control/MCP and runtime-events expose durable retrieval explanations and factor metadata | integration | `cargo test -p openrustclaw-cli inspect -- --nocapture` | ❌ W0 | ⬜ pending |
| 181-02-03 | 02 | 2 | RETR-03 | T-181-02-02 | Runtime-event telemetry persists factor breakdowns, source lineage, and degraded-state metadata across sessions | integration | `cargo test -p openrustclaw-cli runtime_events -- --nocapture` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/db/src/memory_store.rs` targeted tests for preserved FTS rank, explicit score fusion, and batched access updates
- [ ] `crates/app/src/memory_views.rs` tests for bounded recall-pack shaping, dedupe, provenance, freshness, and artifact metadata
- [ ] `crates/cli/src/commands/inspect.rs` or `crates/cli/src/commands/start.rs` tests for retrieval explanation payloads and enriched `MemorySearched` event data
- [ ] `crates/agent/src/prompt.rs` regression test proving no raw recall or archive payloads are injected after Phase 181 changes

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Retrieval explanation output is understandable and bounded in one real operator loop | RETR-02, RETR-03 | Human readability and usefulness are hard to judge from pure assertions | Run one recall search through the existing CLI or control surface, inspect the explanation payload, and confirm it answers “why did this memory appear?” without exposing raw blobs |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 300s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

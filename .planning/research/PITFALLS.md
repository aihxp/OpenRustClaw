# Domain Pitfalls

**Domain:** trust-first assistant runtime work for self-learning, deeper memory, and a God Mode lane
**Researched:** 2026-04-07

The dominant failure mode for this milestone is not "the model makes an occasional mistake." It is persistent state corruption: untrusted content becomes durable memory or lessons, retrieval quality drops while looking richer on paper, and a high-power autonomy lane quietly weakens the default runtime's trust boundary.

For OpenRustClaw specifically, the risk is amplified because the repo already has live learning and autonomy seams instead of starting from zero. `crates/memory/src/policies.rs` currently blocks speculative assistant writes by default, `crates/db/src/memory_store.rs` already has ranking quality debt, `sidecar/src/memory_bridge.py` and `sidecar/src/workflows/rag_pipeline.py` already have fragile integration paths, and `crates/cli/src/commands/enterprise_autonomy.rs` already mutates runtime autonomy policy. v1.43 should harden those seams before broadening them.

## Critical Pitfalls

### Pitfall 1: Untrusted content gets promoted into durable memory or lessons
**Category:** Safety, Product, Implementation
**What goes wrong:** Reflection candidates, tool outputs, browser content, emails, RAG documents, prior model outputs, or autonomy receipts get promoted into durable memory, decision lessons, or future skill drafts and are later treated as truth or permission.
**Why it happens:** The system blurs user-authored intent with third-party content. A "successful run" is mistaken for proof that the underlying reasoning or extracted lesson is safe to persist. This is especially risky if v1.43 weakens the existing write-policy posture in `crates/memory/src/policies.rs` or lets `crates/app/src/autonomy_lessons_control.rs` accept low-provenance lessons.
**Consequences:** Persistent prompt injection, memory poisoning, brittle personalization, self-amplified bad behavior, and unsafe future actions, especially once a high-power lane exists.
**Prevention:** Put every learned artifact through a quarantine pipeline with typed provenance fields such as source kind, source ids, trust level, confidence, and review state. Treat all tool outputs and third-party text as untrusted input. Extract only validated structured fields before promotion. Never let the model write directly from free-form output into shared memory, procedural memory, or skill artifacts. Require operator review for procedural lessons, skill suggestions, and any artifact that could affect future tool use. Keep shared policy artifacts read-only.
**Detection:** Any learned artifact without provenance, confidence, and source references is a defect. Watch for recalls or lessons that cite tool output but cannot show origin, or for repeated bad advice that traces back to one contaminated source.

### Pitfall 2: God Mode bleeds into the default trust-first runtime
**Category:** Safety, Product, Operational
**What goes wrong:** God Mode becomes a broad policy override instead of a sharply isolated lane. Normal runtime flows inherit its permissions, approval bypasses, or tool reachability by accident.
**Why it happens:** The repo already has a full-autonomy control path in `crates/cli/src/commands/enterprise_autonomy.rs`. The easiest implementation is to widen that same policy surface, but that creates configuration bleed, shared-token risk, and mode confusion across CLI, Control UI, gateway, and MCP surfaces.
**Consequences:** The default runtime stops being truthfully operator-gated. Prompt injection and memory poisoning gain a far larger blast radius. Operators lose trust because "off by default" is no longer actually off.
**Prevention:** Treat God Mode as a separate runtime lane with explicit enable, operator identity, TTL or session scope, prominent warnings, dedicated audit fields, and a kill switch. Enforce dangerous-action policy at the downstream tool boundary and MCP layer, not just in prompts. Require a clear mode banner and mode-specific receipts on every surface. Never auto-enter God Mode from task type, lesson confidence, or prior success.
**Detection:** Any destructive or approval-free action in a non-God trace is a release blocker. Any audit record missing mode, operator id, approval state, or kill-switch visibility is incomplete.

### Pitfall 3: Deeper memory increases recall volume while lowering precision and privacy
**Category:** Product, Safety, Implementation
**What goes wrong:** Retrieval appears richer because more memories are found, but answers become noisier, more contradictory, more stale, or more privacy-invasive.
**Why it happens:** Teams optimize for "more remembered things" instead of useful recall. OpenRustClaw already has ranking fragility in `crates/db/src/memory_store.rs`: searches fetch `limit * 3`, flatten BM25 with `bm25_score = 1.0`, and do per-result access-count writes. If v1.43 layers more heuristics on top of that without a replay harness, recall quality will drift.
**Consequences:** Prompt bloat, incorrect personalization, cross-session leakage, harder debugging, and a false sense that memory depth improved the product.
**Prevention:** Preserve the three-tier contract: core stays small and operator-inspectable, recall stays on-demand and scoped, archive stays consolidated. Split user facts, operator model, episodic traces, procedural lessons, and archives into typed namespaces instead of one blended store. Retrieve structured snippets with provenance, not raw files or raw memory blobs. Ship explainable ranking and replayed recall evals before widening default retrieval depth.
**Detection:** Watch recall precision on a frozen trace set, contradictory memories in the same answer, token growth in prompts, and recall results missing explicit scope metadata such as user, operator, namespace, and artifact type.

### Pitfall 4: The learning pipeline degrades silently but still looks "enabled"
**Category:** Operational, Implementation
**What goes wrong:** Missing embeddings, broken bridge configuration, fallback behavior, or process-local stores leave the learning lane partially or fully non-functional while the product still reports that memory or learning is on.
**Why it happens:** This repo already has prototype-grade degradation paths called out in `.planning/codebase/CONCERNS.md`, including `sidecar/src/workflows/rag_pipeline.py` using `RAG_STORE` and `sidecar/test_sidecar.py` broadly skipping on missing dependencies. If similar fallback behavior survives into v1.43, operators will get false confidence.
**Consequences:** Operators cannot trust inspection output. Bugs become anecdotal and hard to reproduce. The milestone can claim "learning" while actually running lexical or non-durable behavior.
**Prevention:** Fail closed when self-learning or deeper-memory features are enabled without real dependencies and durable backing services. Expose health for embeddings, reranking, bridge auth, consolidation, and archive writes. Disable learning activation if prerequisites are missing. Keep mock embeddings, mock answers, and in-process stores strictly test-only.
**Detection:** Any production run using fallback counters, missing embedding model ids, process-local-only storage, or skipped integration checks is unhealthy. A restart that loses learned state is a hard failure.

### Pitfall 5: Self-learning ships without an eval and rollback discipline
**Category:** Product, Safety, Operational
**What goes wrong:** The system accumulates lessons, memory heuristics, or skill suggestions that "felt useful" in anecdotes but reduce net performance or safety.
**Why it happens:** Learning loops create constant pressure to activate artifacts quickly. Without frozen datasets, trace grading, and rollbackable artifact versions, every change becomes vibe-based.
**Consequences:** Regression churn, unsafe tool behavior, inability to explain whether v1.43 improved anything, and no principled way to disable bad learning except a broad rollback.
**Prevention:** Define evals before activation. Build a replay set from shipped memory and autonomy traces, then grade recall precision, lesson precision, unsafe action rate, approval compliance, and mode confusion. Activate learned artifacts in shadow or canary mode before promotion. Version every lesson, profile patch, and ranker change so each can be disabled independently.
**Detection:** If a learned artifact cannot show before or after metrics and cannot be individually deactivated, it is not ready for production.

## Moderate Pitfalls

### Pitfall 1: The system starts inferring sensitive or unstable user or operator traits
**Category:** Product, Safety
**What goes wrong:** Cross-session modeling stores guesses, moods, temporary states, or sensitive traits as if they were stable preferences or trusted facts.
**Prevention:** Keep modeled facts narrow, durable, operator-visible, and editable. Default to explicit user statements and stable project or operator facts. Do not infer sensitive categories by default. Separate user profile, operator profile, and system policy artifacts so each has different write rules.

### Pitfall 2: Audit records are too thin to explain learned behavior or God Mode actions
**Category:** Operational, Implementation
**What goes wrong:** Tool execution history exists, but it cannot answer why an action happened, which memory or lesson influenced it, or what approval state existed at the moment.
**Prevention:** Extend the audit model around `crates/app/src/tool_execution_audit.rs` to capture mode, approval snapshot, lesson ids, memory artifact ids, policy version, and source provenance. Audit should support explanation and rollback, not just event counting.

### Pitfall 3: Consolidation and profile updates race or overwrite each other
**Category:** Operational, Implementation
**What goes wrong:** Concurrent threads or delayed background work overwrite newer profile state, double-process the same conversations, or summarize away important facts.
**Prevention:** Use versioned writes, per-artifact topic granularity, and idempotent consolidation watermarks. Because OpenRustClaw forbids cron as a product mechanism, schedule consolidation only through explicit LangGraph workflow state and track the exact lookback window or watermark so no window is skipped or replayed unintentionally.

### Pitfall 4: Learned skills become hidden self-modification
**Category:** Safety, Product
**What goes wrong:** The runtime starts editing prompts, skills, or tool policies directly rather than producing reviewable suggestions.
**Prevention:** Treat learned skills as drafts, not live code. Require explicit operator review, signing or versioning, and activation through normal artifact-management flows. No silent writes to skill directories or shared prompts.

### Pitfall 5: Surface labeling and warnings drift across CLI, Control UI, and gateway
**Category:** Product, Operational
**What goes wrong:** One surface says a run is supervised while another is effectively running with God Mode overrides or learned artifacts enabled.
**Prevention:** Use one canonical mode-description source and one inspection vocabulary across `crates/cli/src/commands/inspect.rs`, runtime receipts, and control-plane responses. "God Mode", "learned lesson", and "memory-backed personalization" must mean the same thing everywhere.

## Minor Pitfalls

### Pitfall 1: Vocabulary sprawl makes operator trust worse
**Category:** Product
**What goes wrong:** Candidates, memories, lessons, profiles, archives, and skills overlap conceptually, so operators cannot tell what is advisory versus active.
**Prevention:** Keep a strict artifact taxonomy and expose it consistently in inspect, audit, and docs.

### Pitfall 2: Retention grows faster than retrieval quality
**Category:** Operational
**What goes wrong:** v1.43 stores more artifacts than it can rank, inspect, or expire cleanly.
**Prevention:** Apply quotas, TTLs, archive thresholds, and compaction rules from day one. Do not defer storage hygiene until after recall quality work.

### Pitfall 3: Test coverage stays unit-heavy while system behavior changes cross-session
**Category:** Implementation
**What goes wrong:** Inline tests pass, but the real failures appear only across sessions, restarts, approval boundaries, or mixed Rust/Python flows.
**Prevention:** Add integration and E2E coverage for cross-session recall, lesson quarantine and activation, God Mode receipts, kill switch behavior, and fallback rejection. Reuse the existing real-SQLite and real-router test patterns from `.planning/codebase/TESTING.md`.

## Phase-Specific Warnings

Roadmap order should be defensive. Do not activate self-learning or broaden God Mode before candidate provenance, replay evals, and rollback paths exist.

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Candidate capture and lesson schema | Treating successful outcomes as sufficient evidence to learn | Start with quarantine-only artifacts. Require provenance, confidence, source ids, and inactive-by-default status before any activation path exists. |
| Retrieval and ranking hardening | Deeper memory worsens precision, privacy, and prompt size | Build replay evals first, preserve FTS and scope signals in `crates/db/src/memory_store.rs`, and ship inspectable ranking reasons before changing defaults. |
| Consolidation and user/operator modeling | One giant mutable profile drifts, conflicts, or stores unstable inferences | Use typed documents per concern, explicit namespaces, durable-fact rules, and editable operator-visible summaries instead of raw prompt injection. |
| Learned lessons and skill suggestions | Hidden self-modification or prompt drift | Keep lessons advisory until reviewed. Generate skill proposals as review artifacts, not automatic writes. Require rollback and per-artifact activation state. |
| God Mode runtime lane | Privilege bleed, approval bypass, and unsafe tool reach | Isolate mode config, require explicit enable plus TTL, keep point-of-risk confirmations, and enforce downstream authorization and sandboxing for shell, browser, and MCP actions. |
| Audit, inspect, and recovery | Inability to explain why something was learned or done | Record mode, approvals, source artifacts, and policy version on every learned activation and high-risk action. Add kill switch and artifact-level disable or deactivate flows. |
| Verification and release exit | Shipping on anecdotal wins | Block release on replay-eval improvement, no silent fallbacks, God Mode isolation tests, and documented recovery drills. |

## Sources

- HIGH: Repo context in `.planning/PROJECT.md`, `.planning/codebase/CONCERNS.md`, `.planning/codebase/TESTING.md`, and `.planning/codebase/CONVENTIONS.md`
- HIGH: Repo seams in `crates/memory/src/policies.rs`, `crates/db/src/memory_store.rs`, `sidecar/src/memory_bridge.py`, `sidecar/src/workflows/rag_pipeline.py`, `crates/app/src/autonomy_lessons_control.rs`, `crates/app/src/tool_execution_audit.rs`, and `crates/cli/src/commands/enterprise_autonomy.rs`
- HIGH: OpenAI, "Safety in building agents" https://developers.openai.com/api/docs/guides/agent-builder-safety
- HIGH: OpenAI, "Computer use" https://developers.openai.com/api/docs/guides/tools-computer-use
- HIGH: OpenAI, "Local shell" https://developers.openai.com/api/docs/guides/tools-local-shell
- HIGH: OpenAI, "Evaluation best practices" https://developers.openai.com/api/docs/guides/evaluation-best-practices
- HIGH: OWASP, "Top 10 for Large Language Model Applications" https://owasp.org/www-project-top-10-for-large-language-model-applications/
- HIGH: OWASP GenAI, "LLM08: Excessive Agency" https://genai.owasp.org/llmrisk2023-24/llm08-excessive-agency/
- MEDIUM: LangChain docs, "Memory overview" https://docs.langchain.com/oss/javascript/concepts/memory
- MEDIUM: LangChain docs, "Deep Agents Memory" https://docs.langchain.com/oss/javascript/deepagents/memory
- MEDIUM: Anthropic Engineering, "Writing effective tools for AI agents" https://www.anthropic.com/engineering/writing-tools-for-agents

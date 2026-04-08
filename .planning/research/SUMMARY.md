# Project Research Summary

**Project:** OpenRustClaw
**Domain:** Trust-first Rust assistant runtime with balanced memory, reviewable self-learning, and a high-power autonomy lane
**Researched:** 2026-04-07
**Confidence:** HIGH

## Executive Summary

OpenRustClaw v1.43 should be built as a hardening milestone, not an expansion-by-default milestone. The research is consistent: balanced memory, self-learning, and God Mode should all extend the existing Rust-first runtime instead of creating parallel stacks. Durable state should stay in `crates/db` and `crates/memory`, the Python sidecar should remain an async workflow lane only, and the current autonomy and lesson seams should be deepened with better provenance, review, and audit rather than replaced.

For balanced memory, the decision is to improve retrieval quality before increasing retrieval breadth. The recommended approach is a three-layer memory substrate inside SQLite plus libSQL: small core memory, typed on-demand recall, and archived summaries with provenance. Retrieval should become hybrid and inspectable, with typed artifact classes, explicit score components, and bounded prompt projection instead of raw memory injection. This gives the milestone better recall quality, better operator trust, and a cleaner base for user and operator modeling.

For self-learning and God Mode, the safe pattern is candidate before promotion. Successful runs, reflections, and tool audit can generate learning candidates, but nothing should become a lesson, model artifact, or skill update until it passes review, evaluation, and rollback discipline. God Mode should be a named overlay on the existing enterprise autonomy lane with stronger audit, TTL or session scope, and artifact labeling, not a second runtime or a silent permissions widening. The main risks are persistent state corruption, retrieval precision collapse, and privilege bleed; the mitigation is phase order: retrieval and inspection first, promotion controls second, stronger execution power last.

## Key Findings

### Recommended Stack

The stack recommendation is conservative and repo-aligned. Keep SQLite, `sqlx`, `libsql`, and the existing Rust memory abstractions as the center of gravity. Upgrade the current retrieval path so libSQL-native vector search and FTS5 candidate generation feed a fused, inspectable ranking pipeline in Rust, instead of doing shallow BM25 plus in-process vector scans. Keep LangGraph scheduling in the sidecar, but fail closed when memory or learning prerequisites are missing and route all durable writes back through Rust-owned APIs.

For retrieval quality, add a retrieval-specialized provider lane rather than a new platform. The strongest recommendation is Cohere embeddings plus Cohere rerank through `crates/providers`, with OpenAI embeddings as fallback when Cohere is not configured. For learning and skill reuse, keep file-backed approved lessons and compiled skills, but move candidate queues, promotions, model artifacts, and evaluation evidence into SQLite so they can be inspected, filtered, expired, and rolled back cleanly.

**Core technologies:**
- `sqlx` + SQLite: canonical relational store for memory, learning candidates, promotions, and auditable metadata.
- `libsql`: native vector query execution path for scalable, repo-aligned hybrid retrieval without adding a second database.
- `crates/core` traits + `crates/db`/`crates/memory`: existing abstraction seam for memory retrieval, persistence, and policy without forking the runtime.
- `cohere` in `crates/providers`: recommended retrieval profile for document/query embeddings and hosted reranking.
- `async-openai`: fallback embeddings provider when Cohere is unavailable.
- LangGraph sidecar: bounded async summarization and evaluation orchestration only, not source-of-truth storage.
- LangSmith + existing observability stack: experiment and trace sink for recall, learning, and God Mode evals instead of adding a new eval platform.

### Expected Features

The must-have outcome is not “more memory”; it is balanced memory with inspection, provenance, and control. v1.43 needs better hybrid recall, assembled and deduplicated results, structured user/operator/project artifacts, and a review queue that turns successful runs and reflection evidence into promotable lessons or skill proposals. God Mode must remain operator-explicit, separately labeled, and fully auditable.

The milestone’s differentiators are explainable retrieval, typed model artifacts, skill-improvement proposals that remain human-readable and diffable, and a God Mode manifest that clearly labels higher-power actions and artifacts. The consistent defer recommendation is to postpone any always-on autonomous self-modification, global identity stitching, and dashboard-heavy memory UX until the review and audit substrate is trustworthy.

**Must have (table stakes):**
- Inspectable hybrid recall with real lexical, vector, recency, confidence, and importance shaping.
- Bounded assembled recall output with provenance, freshness, and artifact-type metadata.
- Structured user, operator, and project memory artifacts consumed through recall, not raw prompt injection.
- Reviewable learning candidates for lessons, model updates, archive projections, and skill proposals.
- Explicit God Mode enable, disable, kill-switch, audit, and recovery flows.

**Should have (competitive):**
- “Why this memory appeared” explanations and retrieval-event logging.
- Distinct artifact classes such as stable preference, trusted project fact, operator override, lesson, and skill hint.
- Lightweight evaluation before promotion, including replay against recent traces or fixtures.
- Skill-improvement proposals that can update existing skills, not only create new ones.
- Artifact labeling and quarantine for anything learned or produced under God Mode.

**Defer (v2+):**
- Full graph-memory exploration UI.
- Fully automatic prompt or skill self-editing.
- Global cross-workspace identity stitching.
- Autonomous background God Mode campaigns.
- Broad semantic search over every repo artifact by default.

### Architecture Approach

The architecture recommendation is to keep one runtime and add missing services around it. Rust should own a new memory retrieval assembly service, a learning ingest and review layer, structured user/operator model stores, and promotion plus rollback paths. The sidecar should only summarize, score, or evaluate candidates asynchronously through internal APIs. Approved lessons should remain file-backed because the current routing layer already consumes them there; event-heavy candidate state belongs in SQLite. God Mode should stay attached to the existing enterprise autonomy manifest and event model, with stronger grants and stronger receipts rather than a parallel control plane.

**Major components:**
1. `crates/app/src/memory_retrieval.rs` — assemble bounded memory packs from core, recall, archive, and structured model artifacts.
2. `crates/app/src/learning_loop_control.rs` + `crates/app/src/learning_review.rs` — turn run receipts and tool audit into candidates, then promote or reject them with rollback metadata.
3. `crates/db/src/learning_store.rs` + `crates/db/src/model_store.rs` — persist candidate queues, promotions, retrieval events, and structured user/operator models.
4. `crates/app/src/skill_improvement_control.rs` + `.claw/skills/proposals/` — generate reviewable skill proposals and pass approved ones into the existing compile path.
5. `crates/cli/src/commands/enterprise_autonomy.rs` and related control surfaces — remain the owner of God Mode enablement, warnings, TTL, kill switch, and audit receipts.

### Critical Pitfalls

The risks are clear and mostly self-inflicted if the milestone is sequenced badly. The system can easily look “smarter” while becoming less trustworthy if it promotes untrusted content, widens recall without precision controls, or lets God Mode permissions bleed into default execution. The research strongly recommends defensive ordering and explicit artifact taxonomy to prevent that.

1. **Untrusted content becomes durable truth** — quarantine every learned artifact with provenance, trust level, confidence, and review state before promotion.
2. **God Mode bleeds into default runtime behavior** — keep it as a separately enabled, TTL-scoped, enterprise-authenticated overlay with dedicated audit fields and kill switch coverage.
3. **Deeper memory lowers precision and privacy** — ship replayed recall evals, typed namespaces, and inspectable score reasons before broadening default retrieval depth.
4. **Learning silently degrades behind fallbacks** — fail closed when embeddings, reranking, bridge auth, or durable stores are unavailable; never ship mock fallbacks as live behavior.
5. **Self-learning ships without rollback discipline** — version every lesson, profile patch, ranker change, and skill proposal so each can be disabled independently after eval.

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: Retrieval And Inspection Foundation
**Rationale:** Balanced memory starts with better retrieval quality and better operator visibility; otherwise every later learning feature amplifies ranking debt.
**Delivers:** Hybrid retrieval hardening in Rust, retrieval-event logging, typed artifact metadata, explainable assembled recall, and recall replay evals.
**Addresses:** Inspectable hybrid recall, bounded assembled recall, provenance-linked memory inspection.
**Avoids:** Precision collapse, privacy leakage, and anecdotal “more memory” wins.

### Phase 2: Balanced Memory Artifacts And Model Stores
**Rationale:** Once retrieval is trustworthy, add durable user, operator, project, and archive artifacts with selective projection into core memory.
**Delivers:** Structured `user_models` and `operator_models`, typed memory artifacts, archive lineage, consolidation policy gates, and bounded prompt projection.
**Uses:** SQLite, `sqlx`, `libsql`, existing `crates/memory` policy and archive seams.
**Implements:** Rust-owned durability with sidecar-assisted summarization only.

### Phase 3: Learning Candidate Queue And Promotion Controls
**Rationale:** Self-learning should start as reviewable candidate capture, not live behavior.
**Delivers:** `learning_candidates`, `learning_promotions`, provenance schema, operator review surfaces, promotion or rejection flow, and rollback pointers.
**Addresses:** Reviewable lessons, model updates, archive projections, and trust-first learning activation.
**Avoids:** Memory poisoning, silent activation, and unbounded lesson accumulation.

### Phase 4: Skill Proposal Verification Lane
**Rationale:** Reusable behavior comes after lessons because skill changes carry a larger blast radius.
**Delivers:** Skill proposal candidates, proposal storage under `.claw/skills/proposals/`, verification gates, acceptance rubric, and approved compile path integration.
**Uses:** Existing skill registry mutation and compiled skill pipeline.
**Implements:** Candidate-before-promotion for reusable skills without hidden self-modification.

### Phase 5: God Mode Overlay, Audit, And Recovery
**Rationale:** God Mode should be last because it magnifies every weakness in retrieval, learning, and audit.
**Delivers:** Named God Mode overlay on enterprise autonomy, session or TTL scope, append-only audit receipts, artifact labeling and quarantine, recovery and kill-switch drills.
**Addresses:** Distinct high-power lane, stronger warnings, stronger action journal, quarantine of artifacts produced under stronger grants.
**Avoids:** Privilege bleed, approval bypass, and mode confusion across surfaces.

### Phase Ordering Rationale

- Retrieval comes first because the current ranking path already has known debt, and every later memory or learning feature depends on trustworthy recall.
- Structured memory artifacts follow retrieval because projection and consolidation are only safe once scoring, provenance, and inspection are stable.
- Learning candidates come before lessons or skills so the milestone can collect signal without silently changing future behavior.
- Skill improvement stays behind review and verification because executable artifacts are higher risk than lessons or profile summaries.
- God Mode is last because stronger grants without strong audit, rollback, and artifact labeling would weaken the repo’s trust-first posture.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 1:** libSQL vector-query schema, current crate support details, and final score-fusion design need implementation-level validation.
- **Phase 3:** evaluation dataset shape and promotion criteria for lessons versus model updates need precise planning to avoid weak review gates.
- **Phase 5:** exact authorization boundaries across CLI, control UI, gateway, and MCP need a dedicated security review before rollout.

Phases with standard patterns (skip research-phase):
- **Phase 2:** structured model stores and bounded projection follow directly from the repo’s existing memory policies and architecture guidance.
- **Phase 4:** reviewable skill proposal flow is mostly an extension of the existing skill mutation and compile path rather than a novel subsystem.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Strong repo fit plus official vendor docs support SQLite + libSQL, Cohere retrieval, and LangSmith tracing. |
| Features | MEDIUM | Product expectations are well supported, but differentiator priority still depends on milestone scope and operator tolerance. |
| Architecture | HIGH | The current codebase already exposes the right seams; the main decision is where to deepen them, not whether to replace them. |
| Pitfalls | HIGH | Risks are grounded in existing repo seams and well-supported safety guidance around memory, agency, eval, and audit. |

**Overall confidence:** HIGH

### Gaps to Address

- `libsql` implementation details: validate the exact vector-native schema and query API supported by the versions already pinned in the workspace before phase breakdown.
- Retrieval provider default: confirm whether Cohere should be default-on for milestone planning or optional behind configuration due to cost and operator setup friction.
- Promotion thresholds: define concrete acceptance criteria for lesson, model, and skill candidate promotion before work begins.
- God Mode surface contract: decide whether “God Mode” is a pure naming overlay on full autonomy or requires additional manifest fields and UI vocabulary changes.
- Replay datasets: freeze representative recall, learning, and autonomy traces early so success criteria are measurable during execution.

## Sources

### Primary (HIGH confidence)
- Local repo research: `.planning/research/STACK.md`, `.planning/research/FEATURES.md`, `.planning/research/ARCHITECTURE.md`, `.planning/research/PITFALLS.md`
- Repo seams cited across research: `crates/db/src/memory_store.rs`, `crates/memory/src/policies.rs`, `crates/memory/src/archive.rs`, `crates/cli/src/commands/orchestrate.rs`, `crates/app/src/autonomy_lessons_control.rs`, `crates/cli/src/commands/control.rs`, `crates/cli/src/commands/enterprise_autonomy.rs`, `crates/app/src/tool_execution_audit.rs`, `sidecar/src/workflows/memory_maintenance.py`
- Turso AI and Embeddings docs: https://docs.turso.tech/features/ai-and-embeddings
- Cohere Embed API docs: https://docs.cohere.com/reference/embed
- Cohere Rerank API docs: https://docs.cohere.com/reference/rerank
- LangSmith OpenTelemetry tracing docs: https://docs.langchain.com/langsmith/trace-with-opentelemetry
- OpenAI safety and evaluation guidance: https://developers.openai.com/api/docs/guides/agent-builder-safety, https://developers.openai.com/api/docs/guides/evaluation-best-practices, https://developers.openai.com/api/docs/guides/tools-computer-use, https://developers.openai.com/api/docs/guides/tools-local-shell
- OWASP GenAI guidance: https://owasp.org/www-project-top-10-for-large-language-model-applications/ and https://genai.owasp.org/llmrisk2023-24/llm08-excessive-agency/

### Secondary (MEDIUM confidence)
- OpenClaw Honcho Memory docs: https://docs.openclaw.ai/concepts/memory-honcho
- OpenClaw plugin and skill docs: https://docs.openclaw.ai/tools/plugin and https://docs.openclaw.ai/tools/creating-skills
- Honcho representation API docs: https://docs.honcho.dev/v3/api-reference/endpoint/peers/get-representation
- LangGraph memory and persistence docs: https://docs.langchain.com/oss/javascript/langgraph/add-memory
- LangChain memory overview and deep agents memory docs: https://docs.langchain.com/oss/javascript/concepts/memory and https://docs.langchain.com/oss/javascript/deepagents/memory
- Brennan Jones et al., "Users' Expectations and Practices with Agent Memory" (CHI EA 2025): https://brennanjones.com/media/documents/publications/chiea25-666.pdf
- Anthropic Engineering, "Writing effective tools for AI agents": https://www.anthropic.com/engineering/writing-tools-for-agents

---
*Research completed: 2026-04-07*
*Ready for roadmap: yes*

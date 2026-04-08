# Feature Research: v1.43 Learning Loop, Memory Depth, and God Mode

**Scope:** deeper recall quality, durable memory consolidation, cross-session user and operator modeling, reviewable self-learning, reusable skill improvement, and an explicit God Mode lane
**Researched:** 2026-04-07

## Category 1: Recall Quality and Memory Inspection

### Table Stakes

- Improve recall ranking so search uses a real hybrid signal instead of brittle lexical or vector-only matches. For this repo, that means better score shaping across `crates/db/src/memory_store.rs`, not just a larger result set.
- Return assembled recall results that are concise, deduplicated, and provenance-linked so the operator can tell why a memory was surfaced.
- Preserve the current trust boundary that only core memory is always injected. `crates/memory/src/context.rs` and `crates/agent/src/prompt.rs` already establish the right baseline: recall stays tool-driven, not prompt-stuffed.
- Add first-class inspection for recall quality: surfaced memories should show origin, freshness, confidence, and whether they came from raw recall or a consolidated summary.
- Make memory search useful across sessions, compaction, and channel changes, since that is now table stakes in systems like OpenClaw plus Honcho.

### Differentiators

- Add "why this memory appeared" metadata and a ranked explanation trace, not just a blob of recalled text.
- Distinguish user facts, operator preferences, project facts, past decisions, and execution lessons at retrieval time so the runtime can prefer the right class for the task.
- Support a bounded "recall preview" lane in inspect or control surfaces so operators can debug bad retrieval without reading raw SQLite rows or raw memory files.

### Deferred Ideas

- Full graph-memory exploration UI.
- Background re-embedding or continuous retrieval tuning daemons.
- Broad semantic search over every repo artifact by default.

### Anti-Features

- Raising recall by dumping full `MEMORY.md` or other raw memory artifacts into every prompt.
- Pretending more retrieved items automatically means better memory.
- Hiding ranking decisions behind a single opaque "relevance" score with no inspectable factors.

## Category 2: Consolidated Memory and User or Operator Models

### Table Stakes

- Consolidate lower-level recall into durable summaries and profile artifacts so the system remembers stable patterns without replaying whole transcripts. `crates/memory/src/archive.rs` already points in this direction and should become product-grade.
- Maintain separate artifacts for user model, operator model, and project or workspace memory. The milestone should not collapse those into one undifferentiated profile.
- Keep consolidation reviewable and policy-gated through `crates/memory/src/policies.rs`; learned summaries should be durable only when they clear explicit policy.
- Ensure the runtime consumes model artifacts through structured recall, not raw file injection. This is required to stay aligned with the existing "recall-only memory" contract.
- Support inspection and correction of learned profiles so operators can remove stale or wrong inferences.

### Differentiators

- Add explicit memory classes such as "stable preference", "working style", "trusted project fact", and "operator override" so model artifacts are more precise than generic summaries.
- Let the system produce compact "current understanding" artifacts that can be cited back to the operator before they are promoted into stronger profile state.
- Preserve source lineage from a profile claim back to its contributing memories, reflections, or successful runs.

### Deferred Ideas

- Fully automatic persona evolution with no operator checkpoints.
- Rich visual profile editors or dashboard-heavy profile UX.
- Global cross-workspace identity stitching.

### Anti-Features

- Writing speculative personality claims into durable memory with no review path.
- Merging user and operator preferences into one model when those scopes have different trust semantics.
- Making profile updates silent and irreversible.

## Category 3: Learning Candidates, Lessons, and Reuse

### Table Stakes

- Turn successful runs and reflection candidates into reviewable learning candidates. The current reflection seam in `crates/app/src/orchestration_reporting.rs` and the decision-lesson control surface in `crates/app/src/autonomy_lessons_control.rs` are the right starting points.
- Promote only bounded lessons: concrete signals, recommendations, scope, confidence, and provenance. The existing decision-lesson shape is already close to what a trust-first learning loop needs.
- Separate candidate generation from promotion. The runtime may suggest a lesson automatically, but durable activation should stay explicit.
- Record where a lesson came from: success trace, failure trace, human correction, or repeated pattern.
- Let operators inspect active lessons and deactivate them cleanly when they become stale.

### Differentiators

- Support lesson types beyond autonomy hints: recall tuning hints, routing hints, tool-choice hints, and skill-gap hints.
- Add lightweight evaluation before promotion: replay against a recent trace, a bounded fixture, or an operator confirmation step.
- Track lesson usefulness over time so stale or low-yield lessons decay instead of accumulating forever.

### Deferred Ideas

- Automatic RL-style policy optimization.
- Self-editing prompts with no human-readable lesson artifact.
- Cross-workspace lesson federation.

### Anti-Features

- Treating every successful run as something that should be remembered.
- Promoting lessons directly from chain-of-thought-like artifacts or transient reasoning traces.
- Building an unbounded lesson store with no expiry, merge, or deactivation path.

## Category 4: Reusable Skill Improvement

### Table Stakes

- Convert repeated successful workflows into reviewable skill proposals, not hidden self-modification. The repo already has stable skill seams in `crates/app/src/skill_registry_mutation.rs`, `crates/app/src/compiled_skill_overview.rs`, and `crates/skills/src/*`.
- Keep proposals human-readable and diffable: trigger, intent, required tools, guardrails, and expected outputs.
- Require verification before install or activation. A proposed skill should pass at least one bounded replay, compile, or fixture step before it becomes reusable.
- Separate "suggest a skill" from "install or update a skill". This preserves the existing trust posture around skill mutation.
- Preserve provenance from skill proposal back to the lesson or traces that motivated it.

### Differentiators

- Generate skill proposals that explicitly reference missing reusable behavior, not just dump a transcript into `SKILL.md`.
- Support "improve existing skill" proposals alongside "create new skill" proposals so the system can sharpen current capabilities without uncontrolled sprawl.
- Add a small acceptance rubric for proposals: does it reduce repetition, is the tool surface bounded, is the trigger observable, and is the failure mode inspectable.

### Deferred Ideas

- Marketplace publishing flows for learned skills.
- Fully automatic skill rollout to all workspaces.
- Skill synthesis from long multi-run histories without intermediate review.

### Anti-Features

- Allowing the assistant to rewrite installed skills silently after a good run.
- Treating raw transcripts as reusable skills with no abstraction step.
- Auto-installing networked or high-capability skills from learned behavior alone.

## Category 5: God Mode as a Separate High-Power Lane

### Table Stakes

- Keep God Mode as a distinct operator-explicit lane rather than a silent expansion of default autonomy. `crates/cli/src/commands/enterprise_autonomy.rs` already establishes the correct shape for a separate override path.
- Make the lane obviously different in naming, warnings, audit output, and recovery behavior. It should not feel like "managed mode, but slightly stronger."
- Preserve explicit enable, disable, kill-switch, and baseline-restore flows.
- Record a stronger action journal for God Mode runs so operators can inspect what happened, what was bypassed, and what learned artifacts were created under the stronger lane.
- Prevent God Mode from bypassing memory or skill provenance rules. High power should expand execution scope, not erase trust evidence.

### Differentiators

- Add a session- or run-scoped God Mode manifest that lists why it was enabled, what bounds were changed, and what artifacts it produced.
- Clearly mark which lessons, memories, or skill proposals were produced under God Mode so later review can weigh them differently.
- Provide a fast recovery lane that can disable God Mode and quarantine artifacts produced during the high-power run.

### Deferred Ideas

- Broad UX redesign around enterprise governance.
- Multi-operator approval workflow expansion beyond the current scoped baseline.
- Autonomous background God Mode campaigns.

### Anti-Features

- Making God Mode the easiest path for fixing weak memory or weak skills.
- Letting God Mode silently persist low-confidence memories or self-modified skills.
- Using God Mode as a blanket excuse to skip operator review, artifact labeling, or rollback support.

## Feature Dependencies

`better recall ranking` -> `inspectable assembled recall` -> `consolidated summaries and profiles`

`reflection candidates and successful runs` -> `reviewable learning candidates` -> `promoted lessons`

`promoted lessons` -> `skill proposals` -> `verification and operator approval` -> `installed or updated reusable skills`

`explicit God Mode enablement` -> `high-power action journal` -> `artifact labeling and quarantine` -> `disable or rollback`

## MVP Recommendation

Prioritize:

1. Inspectable hybrid recall with ranking fixes and assembled result traces.
2. Consolidated user, operator, and project model artifacts that stay recall-driven rather than prompt-injected.
3. A review queue that turns reflection candidates and successful runs into promotable lessons.
4. Skill proposal generation plus verification gates, paired with a separately labeled God Mode artifact trail.

Defer:

- Full self-editing skills: it closes the gap fast on paper but directly weakens the trust-first posture.
- Global always-on user modeling across all workspaces: too much identity coupling for this milestone.
- Broad autonomous optimization loops: they create hard-to-explain behavior before the learning artifacts are mature.

## Sources

- OpenClaw Honcho Memory docs: https://docs.openclaw.ai/concepts/memory-honcho
- OpenClaw plugin and memory slot docs: https://docs.openclaw.ai/tools/plugin
- OpenClaw creating skills docs: https://docs.openclaw.ai/tools/creating-skills
- Honcho API docs, representation endpoint: https://docs.honcho.dev/v3/api-reference/endpoint/peers/get-representation
- Jones et al., "Users' Expectations and Practices with Agent Memory" (CHI EA 2025): https://brennanjones.com/media/documents/publications/chiea25-666.pdf

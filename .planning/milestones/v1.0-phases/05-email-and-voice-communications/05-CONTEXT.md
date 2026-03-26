# Phase 5: Email and Voice Communications - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Make the shipped email and voice communication lanes reliable enough for MVP use. This phase hardens the existing Gmail Pub/Sub channel, the existing voice session and talk runtime, and the existing compiled-skill voice-call control surfaces so operators can trust communication outcomes and diagnose failures without inventing a new telephony stack.

</domain>

<decisions>
## Implementation Decisions

### MVP communications boundary
- **D-01:** Phase 5 should harden the communication lanes already present in the Rust runtime instead of broadening scope to every channel or telephony integration in the repo.
- **D-02:** The email lane for MVP is the shipped Gmail Pub/Sub ingress plus outbound action path (`reply`, `label`, `archive`, `delete`, `forward`, workflow trigger), not a generic mailbox abstraction project.
- **D-03:** The phone or voice lane for MVP is the shipped voice-session runtime, talk-mode receipts, and bounded compiled-skill voice-call controls, not full live carrier-grade telephony orchestration.
- **D-04:** Success means those lanes feel production-credible for real operator workflows: configured cleanly, persisted durably, and diagnosable from shipped control or artifact surfaces.

### Reliability and persistence
- **D-05:** Email workflows must persist enough structured evidence to answer: what arrived, what action was taken, and whether the action failed or succeeded.
- **D-06:** Voice workflows must persist enough structured evidence to answer: what session or call occurred, what state it reached, what artifacts were produced, and why it ended or stalled.
- **D-07:** Existing workspace-local `.claw` artifacts and control-plane endpoints are the preferred persistence path; Phase 5 should extend shipped patterns rather than introduce an unrelated storage model.
- **D-08:** Service readiness and runtime health should fail clearly when Gmail or voice prerequisites are missing instead of appearing "available" with partial configuration.

### Operator trust path
- **D-09:** Operators should be able to review communication health and recent outcomes through shipped control surfaces before opening raw workspace files.
- **D-10:** Phase 5 should keep the operator story coherent from readiness checks, to runtime activity, to persisted artifacts, to remediation.
- **D-11:** Email and voice surfaces do not need identical schemas, but they should converge on the same trust pattern: explicit state, durable evidence, and clear failure labels.

### the agent's Discretion
- The exact split between email hardening and voice hardening can move between plans as long as the phase ends with one coherent communications trust story.
- The agent may prioritize Gmail and voice-session surfaces before the deeper voice-call plugin lane if that sequencing yields clearer MVP value and lower implementation risk.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Email ingress and readiness
- `crates/channels/src/gmail_pubsub.rs` — Gmail Pub/Sub ingress, message parsing, attachment metadata, and outbound email actions
- `crates/cli/src/commands/start.rs` — Gmail webhook ingress and control-plane runtime wiring
- `crates/cli/src/commands/services.rs` — readiness probes that decide whether communication lanes appear configured

### Voice runtime and operator control
- `crates/cli/src/commands/voice_runtime.rs` — persisted voice sessions, transcript, artifact, metrics, health, reconnect, and reap flows
- `crates/cli/src/commands/start.rs` — `/control/voice/...` and `/control/talk/...` runtime endpoints
- `crates/cli/src/commands/control_ui.html` — current browser operator surface for voice, talk, and skill voice-call controls
- `crates/cli/src/commands/skills.rs` — bounded voice-call plugin inspection and lifecycle helpers

### Existing documentation and verification anchors
- `README.md` — current claims about shipped voice and communication surfaces
- `docs/src/getting-started/quickstart.md` — first-run operator story that will need a communications addendum
- `tests/integration/src` — current integration coverage for onboarding, assistant continuity, memory, tools, and other trust slices

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `gmail_pubsub.rs` already parses inbound messages richly, captures attachment metadata, and exposes outbound actions, so Phase 5 can harden observability and operator trust without inventing the core email path.
- `services.rs` already includes Gmail readiness probes, which provides an existing operator gate for credential and path validation.
- `voice_runtime.rs` already persists voice-session receipts, transcripts, artifacts, metrics, stale-session health, and reconnect or reap behavior under the workspace.
- `start.rs` and `control_ui.html` already expose substantial voice and talk control APIs, so Phase 5 can focus on coherence, diagnostics, and end-to-end trust rather than greenfield surface creation.
- The Phase 4 execution ledger now provides a general-purpose operator evidence pattern that communications work can reuse when a channel action needs a durable receipt.

### Established Patterns
- Earlier phases improved MVP trust by surfacing typed summaries and operator-facing evidence through the control plane instead of relying on log spelunking.
- Workspace-local files under `.claw` are already the durable artifact root for tool, coding, voice, and other runtime receipts.
- Control UI tends to surface summary and status first, then raw JSON detail, which is the right pattern for communication-lane debugging too.

### Gaps to Close
- Gmail currently has parsing and action capability, but the operator trust story for recent email outcomes is thinner than the newer tool, memory, and coding surfaces.
- Voice runtime is feature-rich, but the MVP communication story still needs a cleaner end-to-end framing around readiness, session outcomes, and failure diagnosis.
- README and quickstart still do not present a concise production-ready communications path that ties email and voice surfaces back to the operator control plane.

</code_context>

<specifics>
## Specific Ideas

- A strong first slice is durable, operator-visible email activity reporting that mirrors the recent-execution trust improvements from Phase 4.
- Voice hardening should probably emphasize end-state clarity: operators need to know whether a session is active, stale, ended, or failed and where the transcript or artifact evidence lives.
- The communications docs should describe one concrete review loop: confirm readiness, trigger or inspect traffic, review persisted outcomes, then debug failures from the retained artifacts.

</specifics>

<deferred>
## Deferred Ideas

- Full multi-provider call-center automation, IVR trees, and enterprise telephony routing remain outside MVP scope.
- New communication channels beyond the already shipped Gmail and voice lanes are out of scope for this phase.
- Deeper business-process automation over email or phone can build later on top of the hardened communication substrate from this phase.

</deferred>

---
*Phase: 05-email-and-voice-communications*
*Context gathered: 2026-03-26*

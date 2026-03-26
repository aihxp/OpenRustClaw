# Phase 1: Onboarding and First-Run Trust - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Make the existing OpenRustClaw install, onboarding, doctor, and first assistant launch path feel production-ready for solo developers and small operator teams. This phase hardens the shipped first-run path rather than inventing a second setup system.

</domain>

<decisions>
## Implementation Decisions

### Onboarding ownership
- **D-01:** Keep onboarding Rust-first and CLI-owned. The default first-run path should be `openrustclaw onboard` plus `openrustclaw doctor`, not a Python-sidecar bootstrap.
- **D-02:** Treat the existing onboarding wizard, doctor report, and getting-started docs as the canonical surfaces to harden instead of adding a parallel setup flow.
- **D-03:** Optimize the default path for a solo operator on a fresh workspace, while preserving an advanced path for extra channels, skills, and service installation.
- **D-04:** Existing workspace state should be handled through explicit keep, modify, or reset-with-backup choices, never silent mutation.

### First-run validation
- **D-05:** `doctor` is the gate for first-start trust. It should clearly separate blocking failures from warnings and cover config, providers, control state, channels, and onboarding-managed workspace state.
- **D-06:** The first-run path should validate user-supplied providers, secrets, and channel prerequisites instead of relying on assumed defaults.
- **D-07:** Safe repair and scaffolding are acceptable when bounded and explicit, but destructive or surprising mutations are not.
- **D-08:** The same diagnostic model should support both interactive operator output and structured automation output.

### First assistant handoff
- **D-09:** A healthy onboarding run should hand off into the persisted assistant session directly, instead of forcing the operator to reconstruct the next step manually.
- **D-10:** The quickstart, installation docs, onboarding summary, and doctor output should agree on the canonical next steps: `start`, `assistant`, `doctor`, and Control UI access.
- **D-11:** Default first-run assistant behavior should stay narrow and trustworthy, with limited tool exposure and persisted session continuity visible from the start.
- **D-12:** A failed health check should block or strongly discourage automatic launch into the assistant path.

### Operator trust and polish
- **D-13:** First-run UX should be explicit about what state is written, where it lives, and how to inspect or recover it.
- **D-14:** Operator messaging should prefer concrete diagnostics and next actions over aspirational product language.
- **D-15:** Production-ready means the path is testable, restartable, and documented, not merely interactive.
- **D-16:** This phase should close gaps across code, docs, and verification together; documentation-only polish is insufficient.

### the agent's Discretion
- The exact CLI wording, doctor check ordering, and summary formatting are at the agent's discretion as long as the flow remains concise and trustworthy.
- The precise split between wizard behavior, doctor behavior, and docs updates is flexible if the resulting first-run path is more coherent.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Onboarding and diagnostics
- `crates/cli/src/commands/onboard.rs` — current onboarding wizard, workspace-state handling, post-onboarding health check, and first assistant launch handoff
- `crates/cli/src/commands/doctor.rs` — current diagnostic model, check inventory, health semantics, and interactive vs structured reporting

### First-run documentation
- `docs/src/getting-started/quickstart.md` — current guided first-run story, persisted assistant session expectations, and memory/tool defaults
- `docs/src/getting-started/installation.md` — install prerequisites, doctor guidance, and onboarding entrypoint
- `README.md` — top-level product positioning plus current claims about onboarding, doctor, and operator setup

### Runtime and operator surfaces
- `config/default.toml` — default runtime assumptions that onboarding and doctor validate
- `crates/cli/src/commands/start.rs` — control and runtime startup surface referenced by the first-run path

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/cli/src/commands/onboard.rs`: existing onboarding wizard with QuickStart vs Advanced profiles, workspace-state detection, post-check health handoff, and optional persisted assistant launch
- `crates/cli/src/commands/doctor.rs`: typed diagnostics report already supports health aggregation and structured output
- `docs/src/getting-started/quickstart.md`: existing operator-facing narrative already frames `doctor`, `onboard`, `start`, and `assistant` as the first-run path
- `README.md`: current product claims already commit the repo to guided onboarding and doctor-backed setup

### Established Patterns
- Rust CLI command modules under `crates/cli/src/commands/` are the canonical operator surface; new onboarding work should extend them instead of creating side channels
- Documentation is treated as a maintained product contract, so onboarding hardening must keep docs and shipped behavior aligned
- The codebase favors typed status and serializable runtime/operator state, which onboarding and diagnostics should preserve

### Integration Points
- Onboarding calls into `doctor::collect_report` for post-setup validation
- Onboarding hands off into the persisted assistant session through the existing chat or assistant command surface
- Runtime defaults and workspace state under `.claw/` plus `.env` are the core state touched during first-run setup

</code_context>

<specifics>
## Specific Ideas

- The first shipped impression should feel like "OpenClaw, but clean": one clear setup path, one health gate, one obvious assistant handoff.
- Phase 1 should serve solo developers and small operator teams first, not enterprise rollout workflows.

</specifics>

<deferred>
## Deferred Ideas

- Richer Control UI onboarding and continuity UX belongs after the basic CLI-led first-run path is trustworthy.
- Enterprise provisioning, RBAC, SSO, and large-team setup flows remain out of scope for this phase.
- Broader assistant memory policy work belongs to Phase 3, even if onboarding docs reference it lightly.

</deferred>

---
*Phase: 01-onboarding-and-first-run-trust*
*Context gathered: 2026-03-26*

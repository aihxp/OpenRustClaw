---
gsd_state_version: 1.0
milestone: v1.44
milestone_name: Agent Discovery, Journey Cohesion, and Provider Access
current_phase: none
current_phase_name: none
current_plan: none
status: milestone complete
stopped_at: Phase 190 completed after repairing the remaining provider-versus-agent journey seams across README, onboarding, inspect, and Control UI.
last_updated: "2026-04-08T23:59:00Z"
last_activity: 2026-04-08
progress:
  total_phases: 5
  completed_phases: 5
  total_plans: 10
  completed_plans: 10
  percent: 100
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-04-08)

**Core value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.
**Current focus:** `v1.44` is complete. The next planning action is to start a new milestone from this stronger agent-lane baseline.

## Current Position

Current Phase: none
Current Phase Name: none
Total Phases: 5
Current Plan: none
Total Plans in Phase: 0
Status: milestone complete
Last activity: 2026-04-08

Phase: 5 of 5
Plan: 2 of 2
Progress: [##########] 100%

## Performance Metrics

**Velocity:**

- Total plans completed: historical total retained across shipped milestones
- Average duration: historical average retained across shipped milestones
- Total execution time: historical multiple-milestone execution retained across v1.0-v1.43

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- v1.42 made provider access mode and explicit primary-model selection truthful during onboarding, but it still left the product catalog split across onboarding, static model lists, and runtime policy surfaces.
- v1.43 strengthened memory, learning, and God Mode, but it did not yet close the broader agent UX gap around local vendor agent discovery, delegated execution, or end-to-end journey cohesion.
- Official vendor guidance now constrains this milestone: Claude Code explicitly supports subscription browser login for Claude Code itself but tells third-party products to use Anthropic APIs or cloud-provider integrations instead of rehosting `claude.ai` login, Gemini CLI documents Google sign-in versus API-key versus Vertex paths, and Codex CLI exposes direct login or API-key paths rather than a token-export contract.
- The milestone will therefore treat installed local agent tools as delegated execution backends or documented provider lanes, not as token sources to be scraped into OpenRustClaw.
- Existing repo seams already support this direction: onboarding has a `subscription_managed` concept, config has `external_backends`, browser policy already governs local CLI wrappers, and control or inspect surfaces already expose rich operator evidence patterns.
- Phase 186 is now complete: the shared app-layer local-agent discovery catalog is reused in `openrustclaw models`, onboarding, and setup handoff or inspect, and Gemini now has runtime parity through those flows.
- Phase 187 plan 01 now formalizes delegated local vendor-agent backends as typed contracts with vendor-managed model labels and policy-ready execution evaluation.
- Phase 187 is now complete after surfacing delegated backend contracts through setup handoff and enterprise policy while keeping the allowlist boundary unified.
- Phase 188 is complete: onboarding and `openrustclaw models` share one provider-or-agent lane catalog, and durable setup state remembers selected lane identity through inspect and repair flows.
- Phase 189 is complete: delegated local-agent backends now resolve through the runtime provider factory as bounded audited runtime lanes, and control init can seed vendor-managed model-profile templates for eligible backends.
- Phase 190 is complete: README, onboarding, inspect, and Control UI now use the same truthful provider-lane versus delegated-agent language and expose delegated runtime receipts more clearly.

### Pending Todos

- None.

### Blockers/Concerns

- Cursor documentation is partially shielded by a Vercel security checkpoint during automated fetches, so Cursor support remains detection-only until a documented programmable surface is confirmed during implementation.

## Session Continuity

Last session: 2026-04-08
Stopped at: Phase 190 completed after repairing the remaining provider-versus-agent journey seams across README, onboarding, inspect, and Control UI.
Resume file: None

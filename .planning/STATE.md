---
gsd_state_version: 1.0
milestone: v1.45
milestone_name: Agent Fabric, Routing Console, and Guided Delegation
current_phase: 191
current_phase_name: cursor surface verification and backend expansion
current_plan: none
status: roadmap defined
stopped_at: Milestone v1.45 was initialized with a five-phase roadmap covering Cursor truthfulness, multi-host delegation, routing UX, and first-task orchestration.
last_updated: "2026-04-08T23:59:00Z"
last_activity: 2026-04-08
progress:
  total_phases: 5
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-04-08)

**Core value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.
**Current focus:** `v1.45` is ready to begin at Phase 191, starting with Cursor truthfulness and delegated backend expansion before the wider fabric and routing UX work.

## Current Position

Current Phase: 191
Current Phase Name: cursor surface verification and backend expansion
Total Phases: 5
Current Plan: none
Total Plans in Phase: 0
Status: roadmap defined
Last activity: 2026-04-08

Phase: 1 of 5
Plan: 0 of 2
Progress: [----------] 0%

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
- `v1.44` closed the basic delegated-agent journey, but Cursor remains detection-only, multi-host delegation does not exist yet, the Control UI is still a broad dashboard instead of a focused routing console, and first-task orchestration can still feel generic after onboarding.
- `v1.45` will treat those as one connected execution-fabric problem rather than four unrelated cleanup items.
- Phase 191 is next: verify Cursor’s programmable surface honestly and either expand backend support or preserve a consistent detection-only boundary.

### Pending Todos

- None.

### Blockers/Concerns

- Cursor documentation is partially shielded by a Vercel security checkpoint during automated fetches, so Cursor support remains detection-only until a documented programmable surface is confirmed during implementation.

## Session Continuity

Last session: 2026-04-08
Stopped at: Milestone v1.45 was initialized with a five-phase roadmap covering Cursor truthfulness, multi-host delegation, routing UX, and first-task orchestration.
Resume file: None

---
gsd_state_version: 1.0
milestone: v1.44
milestone_name: Agent Discovery, Journey Cohesion, and Provider Access
current_phase: none
current_phase_name: requirements and roadmap definition
current_plan: none
status: defining requirements
stopped_at: Milestone v1.44 started to unify local agent discovery, compliance-safe delegated backend access, onboarding truthfulness, and end-to-end journey cohesion.
last_updated: "2026-04-08T07:25:14Z"
last_activity: 2026-04-08
progress:
  total_phases: 5
  completed_phases: 0
  total_plans: 10
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-04-08)

**Core value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.
**Current focus:** v1.44 is defining and then executing the agent-discovery, provider-access, onboarding, and journey-cohesion roadmap.

## Current Position

Current Phase: none
Current Phase Name: requirements and roadmap definition
Total Phases: 5
Current Plan: none
Total Plans in Phase: 2
Status: defining requirements
Last activity: 2026-04-08

Phase: 0 of 5
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

### Pending Todos

- None.

### Blockers/Concerns

- Cursor documentation is partially shielded by a Vercel security checkpoint during automated fetches, so Cursor support must be scoped conservatively unless a documented programmable surface is confirmed during implementation.

## Session Continuity

Last session: 2026-04-08
Stopped at: Milestone v1.44 started to unify local agent discovery, compliance-safe delegated backend access, onboarding truthfulness, and end-to-end journey cohesion.
Resume file: None

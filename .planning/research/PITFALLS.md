# v1.44 Pitfalls Research

**Date:** 2026-04-08
**Scope:** common failure modes when adding local agent discovery and delegated execution

## Key Pitfalls

### 1. Token Scraping Disguised as Convenience

The biggest risk is turning vendor CLIs into secret sources by reading cached credentials or browser-session state. This is both fragile and likely outside documented vendor paths.

**Prevention:**
- only use documented login, status, or execution flows
- never import vendor tokens into OpenRustClaw config
- preserve a separate backend type for delegated execution

### 2. Lying About Capability

Detecting a binary does not prove it can safely serve as a general-purpose backend.

**Prevention:**
- classify backends as ready, partial, blocked, or unsupported
- show reasons in onboarding and inspect
- keep model discovery optional and truthful

### 3. Fragmented Catalogs

The repo already has onboarding descriptors, static model lists, and backend policy in different places. Adding one more list would make the UX worse.

**Prevention:**
- move catalog truth into shared app services
- render that shared catalog everywhere else

### 4. Delegated Runs Bypassing Runtime Boundaries

External agent CLIs can become invisible power lanes if delegation is not audited and bounded.

**Prevention:**
- emit execution receipts with backend attribution
- preserve approval, autonomy, and memory boundaries
- attach delegated execution to existing policy and audit systems

### 5. Overclaiming Cursor Support

The installed `cursor` binary on this machine looks like a desktop launcher, and automated documentation access is partially blocked by Cursor's security checkpoint.

**Prevention:**
- detect Cursor honestly
- avoid claiming provider execution support without a documented programmable lane
- keep Cursor as integration surface first, backend second

## Phase Assignment

- Phase 186: capability truthfulness and discovery evidence
- Phase 187: compliance-safe backend contracts and policy
- Phase 188: catalog convergence and onboarding truthfulness
- Phase 189: delegated execution boundaries and receipts
- Phase 190: wording drift, dead ends, and cross-surface repair

## Sources

- [Claude Code SDK and third-party guidance](https://docs.anthropic.com/en/docs/claude-code/sdk)
- [Gemini CLI authentication](https://geminicli.com/docs/get-started/authentication/)
- [Gemini CLI terms](https://geminicli.com/terms/)
- [OpenAI introducing the Codex app](https://openai.com/index/introducing-the-codex-app/)

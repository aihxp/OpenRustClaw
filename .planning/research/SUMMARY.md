# v1.44 Research Summary

**Date:** 2026-04-08
**Milestone candidate:** Agent Discovery, Journey Cohesion, and Provider Access

## Synthesis

OpenRustClaw already has most of the architectural seams needed for this milestone:
- onboarding already models `subscription_managed`
- config already models `external_backends`
- policy and inspect surfaces already know how to expose bounded external execution

What it lacks is cohesion. The current provider story is split across onboarding, static model lists, runtime policy, and integration docs. That makes the product feel less capable than it is and less trustworthy than it should be.

Official vendor guidance narrows the safe implementation path:
- Anthropic explicitly supports first-party Claude Code account login for Claude Code, but tells third-party products to use APIs or cloud-provider integrations instead of rehosting `claude.ai` login.
- Gemini CLI supports Sign in with Google, Gemini API keys, and Vertex AI, and notes that auth method changes the applicable terms and privacy posture.
- Codex CLI is a first-party login-capable tool and also exposes explicit API-key login, which suggests OpenRustClaw should prefer direct CLI delegation or documented API use instead of token import.
- Cursor remains the least certain vendor surface for delegated execution on this machine, so support should begin with truthful detection and documented integration rather than aggressive backend claims.

## Recommendation

Ship `v1.44` as a five-phase milestone:
1. discover local agent backends truthfully
2. define compliance-safe delegated backend contracts
3. converge onboarding and model selection around one shared catalog
4. deliver runtime delegation with audit and bounded autonomy
5. audit and repair the full user and agent journeys

## Non-Negotiable Constraint

OpenRustClaw should not scrape or import vendor OAuth tokens, browser sessions, or local credential caches. If a vendor-authenticated local agent is usable, it should be usable through that vendor's documented CLI or API surface, with OpenRustClaw treating it as a delegated backend or direct provider lane accordingly.

## Sources

- [Claude Code SDK and third-party guidance](https://docs.anthropic.com/en/docs/claude-code/sdk)
- [Gemini CLI authentication](https://geminicli.com/docs/get-started/authentication/)
- [Gemini CLI terms](https://geminicli.com/terms/)
- [OpenAI introducing the Codex app](https://openai.com/index/introducing-the-codex-app/)

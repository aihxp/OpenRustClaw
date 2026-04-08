# v1.44 Feature Research

**Date:** 2026-04-08
**Scope:** what the milestone should and should not deliver

## Table Stakes

- Detect installed local agent tools and show them in product surfaces.
- Classify auth style, readiness, and limitations truthfully.
- Keep onboarding, model selection, inspect, and control aligned on the same backend catalog.
- Preserve audit logs and bounded autonomy when delegated execution is used.
- Explain blocked or unsupported backends in plain operator language.

## Differentiators Worth Shipping

- A single provider or agent catalog shared across CLI, onboarding, inspect, and control instead of one-off static menus.
- Durable discovery evidence with version, path, auth capability, model capability, and policy classification.
- A clean distinction between direct API providers and delegated agent backends.
- Truthful vendor-compliance messaging during onboarding instead of hidden assumptions.
- Journey-level repair work that treats the install-to-first-task path and the discovery-to-inspection path as first-class product surfaces.

## Anti-Features

- Pretending every detected tool is a general-purpose provider.
- Using vendor consumer logins as if they were reusable API credentials.
- Guessing model lists from brand names.
- Hiding compliance constraints from onboarding or control surfaces.
- Adding another disconnected catalog that differs from `openrustclaw models` or `inspect`.

## Prioritization

1. Discovery truthfulness
2. Compliance-safe backend contracts
3. Onboarding and model-selection convergence
4. Runtime delegation and inspection
5. Journey audit and product-text cleanup

## Sources

- [Claude Code SDK and third-party guidance](https://docs.anthropic.com/en/docs/claude-code/sdk)
- [Gemini CLI authentication](https://geminicli.com/docs/get-started/authentication/)
- [OpenAI introducing the Codex app](https://openai.com/index/introducing-the-codex-app/)

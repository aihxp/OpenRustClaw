# v1.44 Stack Research

**Date:** 2026-04-08
**Scope:** local agent discovery, delegated execution, onboarding cohesion, provider truthfulness

## Existing Repo Baseline

- `crates/cli/src/commands/onboard.rs` already models `api_key`, `local_runtime`, and `subscription_managed`, but only ships Anthropic, OpenAI, OpenRouter, and Ollama in the onboarding provider catalog.
- `crates/core/src/config.rs` already exposes `external_backends`, `allow_local_cli_wrappers`, and audit-log settings that can govern delegated local agent execution.
- `crates/cli/src/commands/models.rs` still uses a disconnected static catalog and does not share the richer onboarding or backend-policy context.
- `crates/app/src/browser_backend_control.rs` and adjacent enterprise-policy flows prove that the product already has a policy pattern for bounded local CLI wrappers.

## Local Machine Findings

- Installed locally: `cursor` at `/usr/local/bin/cursor`, `claude` at `/opt/homebrew/bin/claude`, `codex` at `/opt/homebrew/bin/codex`, `gemini` at `/opt/homebrew/bin/gemini`.
- No separate `cursor-cli`, `claude-code`, `codex-cli`, or `gemini-cli` binary names were found in `PATH`.
- `claude auth` exposes `login`, `logout`, and `status`.
- `codex login` exposes `status`, `--with-api-key`, and `--device-auth`.
- `gemini` exposes interactive and headless execution, model selection, MCP, extensions, skills, and `--yolo`.
- `cursor --help` on this machine looks like the desktop launcher and MCP/config surface, not a clearly documented general-purpose model backend.

## Vendor-Supported Paths

- **Claude Code:** official docs say developers should use the Anthropic API, Amazon Bedrock, or Vertex AI for third-party offerings, and should not offer `claude.ai` login or rate limits in third-party products.
- **Codex CLI:** official materials show a first-party CLI with direct login support; the CLI itself also supports explicit API-key login, which is a safer reusable integration seam than token import.
- **Gemini CLI:** official docs explicitly support Sign in with Google, Gemini API key, and Vertex AI as authentication choices, and note that the chosen auth method changes terms, privacy, and pricing.
- **Cursor:** automated fetches of Cursor's docs are partially blocked by a Vercel security checkpoint, so Cursor support should remain conservative until a documented programmable execution or auth surface is confirmed.

## Recommended Stack Direction

- Add a shared app-layer `AgentBackendDiscoveryService` that shells out only to safe vendor help or status commands and records typed discovery evidence.
- Add a typed `DelegatedAgentBackend` contract separate from direct API providers.
- Reuse the existing `external_backends` policy family for delegated local agent execution rather than inventing a parallel permission system.
- Keep direct provider SDK/API integrations as the canonical path for Anthropic, OpenAI, Google Gemini API, OpenRouter, and Ollama.
- Treat installed vendor CLIs as delegated execution surfaces, not credential sources.

## What Not To Add

- No credential-store scraping.
- No token import from vendor config or browser caches.
- No fake model enumeration when a vendor CLI does not expose it.
- No Cursor backend claims beyond what the documented surface and local binary actually support.

## Sources

- [Claude Code SDK and third-party guidance](https://docs.anthropic.com/en/docs/claude-code/sdk)
- [Gemini CLI authentication](https://geminicli.com/docs/get-started/authentication/)
- [Gemini CLI terms](https://geminicli.com/terms/)
- [OpenAI introducing the Codex app](https://openai.com/index/introducing-the-codex-app/)

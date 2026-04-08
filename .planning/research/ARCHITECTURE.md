# v1.44 Architecture Research

**Date:** 2026-04-08
**Scope:** shared contracts needed to close agent UX gaps without breaking current provider architecture

## Proposed Architecture

### 1. Shared Discovery Layer

Create an app-layer discovery service that:
- enumerates supported local agent binaries
- runs safe discovery commands like `--help`, `--version`, or documented auth-status commands
- records typed evidence such as path, version, readiness, auth method, model-discovery support, and policy classification

This should feed one canonical catalog for onboarding, `models`, inspect, control, and future MCP tools.

### 2. Separate Backend Types

Keep two explicit runtime lanes:
- **Direct providers**: Anthropic API, OpenAI API, OpenRouter, Gemini API or Vertex, Ollama
- **Delegated local agent backends**: Claude Code, Codex CLI, Gemini CLI, and any future supported local agent runtime

This avoids forcing delegated local agents through the existing `LlmProvider` contract where capability and auth assumptions do not match.

### 3. Policy and Audit Reuse

Reuse the existing `external_backends` policy model and audit-log patterns so delegated agent execution inherits:
- allow or deny lists
- local wrapper controls
- environment allowlists
- durable audit evidence

### 4. Onboarding Catalog Unification

Replace the static onboarding descriptor table plus the disconnected `models.rs` list with a shared app-level catalog adapter. The CLI can still render it, but the source of truth should live outside the command layer.

### 5. Runtime Routing Bridge

Add a bounded execution bridge that can:
- launch supported vendor CLIs
- capture backend attribution and failure evidence
- preserve autonomy and approval boundaries
- write inspectable receipts for operator review

## Integration Notes

- `subscription_managed` already exists conceptually in onboarding, so the architecture should complete that seam rather than invent a new access-mode vocabulary.
- `external_backends` already exists in config and policy, so delegated execution should attach there first.
- `control_registry` and model profiles will need typed references to delegated backends, but they should not collapse backend type differences.
- Cursor should remain a documented integration surface or future backend candidate until a supported programmable lane is confirmed.

## Sources

- [Claude Code SDK and third-party guidance](https://docs.anthropic.com/en/docs/claude-code/sdk)
- [Gemini CLI authentication](https://geminicli.com/docs/get-started/authentication/)
- [Gemini CLI terms](https://geminicli.com/terms/)

# External Integrations

**Analysis Date:** 2026-03-26

## LLM Providers

The workspace includes a generic provider layer in `crates/providers/` plus many provider-specific crates:

- `crates/anthropic_rust/`
- `crates/async_openai/`
- `crates/openrouter_api/`
- `crates/ollama_sdk/`
- `crates/gemini/`
- `crates/groq/`
- `crates/bedrock/`
- `crates/fireworks/`
- `crates/cohere/`
- `crates/perplexity/`
- `crates/azure_openai/`
- `crates/ai21/`
- `crates/vllm/`
- `crates/together/`
- `crates/deepseek/`
- `crates/replicate/`
- `crates/llama_cpp/`

The shipped runtime defaults are configured in `config/default.toml` under `[providers]`.

## Databases and Persistence

- SQLite primary runtime DB at `data/openrustclaw.db` via `config/default.toml`
- Persistence crates:
  - `crates/db/` for schema and storage wiring
  - `crates/memory/` for recall/core memory and artifacts
  - `crates/scheduler/` for durable jobs
- SQL/proto assets:
  - `crates/db/migrations/`
  - `proto/`
  - `crates/distributed/proto/distributed.proto`

## Messaging / Channel Integrations

Channel adapters live in `crates/channels/src/` and cover a broad set of operator-facing platforms, including:

- `telegram.rs`
- `discord.rs`
- `slack.rs`
- `teams.rs`
- `whatsapp.rs`
- `signal.rs`
- `gmail_pubsub.rs`
- `google_chat.rs`
- `matrix.rs`
- `mattermost.rs`
- `twilio.rs`
- `wechat.rs`
- `webchat.rs`

These are configured through `config/default.toml` and example configs under `config/`.

## Browser / Automation Integrations

- Browser and page tooling lives in `crates/automation/src/tools/`
- Browser control commands live in `crates/cli/src/commands/browser.rs`
- External browser backends are controlled by `[external_backends]` in `config/default.toml`

## Skills / Extensions / MCP

- `crates/skills/` handles skill loading, compilation, sandboxing, and marketplace logic
- `crates/mcp/` provides MCP client/server support
- `crates/mcp2cli/` provides MCP/OpenAPI to CLI bridging and saved-source workflows
- Workspace/operator metadata and compiled outputs are expected under `.claw/`

## Security / Identity / Auth

- `crates/security/` contains:
  - auth checks
  - audit logic
  - input sanitization
  - origin validation
  - skill verification
  - isolation/sandbox concerns
  - SSO/OIDC/SAML support in `crates/security/src/sso/`

Config-level auth controls live in:
- `config/default.toml` `[security]`
- gateway and control APIs via runtime/control command surfaces

## Sidecar / Interop

- Optional Python compatibility lane in `sidecar/`
- gRPC contract documentation in `sidecar/README.md`
- Rust/Python bridge concerns are also documented in `docs/src/architecture/overview.md`

## Observability / Operations

- Metrics/tracing stack lives in `crates/observability/`
- Prometheus and Grafana assets:
  - `prometheus/prometheus.yml`
  - `grafana/dashboard.json`
- CI integration via `.github/workflows/ci.yml`

## Deployment Targets

- Docker: `Dockerfile`, `Dockerfile.dev`, `docker-compose.yml`, `docker-compose.dev.yml`
- Helm: `deployments/helm/openrustclaw/`
- Terraform:
  - `deployments/terraform/aws/`
  - `deployments/terraform/azure/`
  - `deployments/terraform/gcp/`

## High-Signal Paths

- `config/default.toml`
- `crates/providers/src/lib.rs`
- `crates/channels/src/lib.rs`
- `crates/skills/src/lib.rs`
- `crates/mcp/src/lib.rs`
- `sidecar/README.md`
- `deployments/`

---
*Integration analysis: 2026-03-26*
*Update when providers, channels, deployment surfaces, or external contracts change*

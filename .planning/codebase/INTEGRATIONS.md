# External Integrations

**Analysis Date:** 2026-04-04

## APIs & External Services

**LLM Providers wired into the runtime:**
- Anthropic - default provider lane for chat completions
  - SDK/Client: `crates/providers/src/anthropic.rs` plus native SDK crate `crates/anthropic_rust/`
  - Auth: `ANTHROPIC_API_KEY`
- OpenAI - primary/fallback chat, embeddings, STT, and TTS surfaces
  - SDK/Client: `crates/providers/src/openai.rs`, native SDK crate `crates/async_openai/`, voice settings in `config/default.toml`
  - Auth: `OPENAI_API_KEY`
- OpenRouter - control-plane provider and routed fallback with quality/price/throughput/web search strategies
  - SDK/Client: `crates/providers/src/openrouter.rs` and `crates/openrouter_api/`
  - Auth: `OPENROUTER_API_KEY`
- Ollama - local model fallback over local REST
  - SDK/Client: `crates/providers/src/ollama.rs`
  - Auth: None
- Gemini - configurable Google Generative Language provider
  - SDK/Client: `crates/providers/src/gemini.rs`
  - Auth: `GEMINI_API_KEY`

**Additional native model SDK crates shipped in the workspace:**
- AWS Bedrock - standalone SDK crate in `crates/bedrock/`
  - SDK/Client: `crates/bedrock/src/client.rs`
  - Auth: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, optional `AWS_SESSION_TOKEN`
- Azure OpenAI - standalone enterprise SDK crate in `crates/azure_openai/`
  - SDK/Client: `crates/azure_openai/src/client.rs` and `crates/azure_openai/src/auth.rs`
  - Auth: `AZURE_OPENAI_API_KEY` or Azure AD token flow
- AI21, Cohere, Cloudflare AI, DeepSeek, Fireworks, Groq, Mistral, Perplexity, Replicate, Together, vLLM, llama.cpp - standalone SDK crates in `crates/ai21/`, `crates/cohere/`, `crates/cloudflare_ai/`, `crates/deepseek/`, `crates/fireworks/`, `crates/groq/`, `crates/mistral/`, `crates/perplexity/`, `crates/replicate/`, `crates/together/`, `crates/vllm/`, and `crates/llama_cpp/`
  - SDK/Client: each crate owns its API client in `src/client.rs`
  - Auth: provider-specific env vars in crate examples and READMEs

**Messaging and communications integrations:**
- Slack - bot API plus optional HTTP ingress or Socket Mode
  - SDK/Client: `crates/channels/src/slack.rs`
  - Auth: bot token plus optional app token and signing secret from `config/default.toml`
- Telegram - polling or webhook bot integration
  - SDK/Client: `crates/channels/src/telegram.rs`
  - Auth: bot token from `config/default.toml`
- Discord - bot REST + verified interactions ingress
  - SDK/Client: `crates/channels/src/discord.rs`
  - Auth: bot token, application ID, interaction public key
- Microsoft Teams - Bot Framework integration
  - SDK/Client: `crates/channels/src/teams.rs`
  - Auth: app ID, app password, optional tenant ID
- Mattermost - REST + slash/outgoing webhook integration
  - SDK/Client: `crates/channels/src/mattermost.rs`
  - Auth: bot token, optional webhook token
- Google Chat - bot API with service-account OAuth and optional Pub/Sub
  - SDK/Client: `crates/channels/src/google_chat.rs`
  - Auth: service account key path / token source
- Google Meet - Meet API plus Workspace Events / Pub/Sub push
  - SDK/Client: `crates/channels/src/google_meet.rs`
  - Auth: service account key path and delegated user
- Gmail Pub/Sub - Gmail watch/history sync
  - SDK/Client: `crates/channels/src/gmail_pubsub.rs`
  - Auth: service account key path, Google project/subscription config
- WhatsApp - local Node bridge using Baileys
  - SDK/Client: `crates/channels/src/whatsapp.rs` and `crates/channels/baileys-bridge/package.json`
  - Auth: local session files under configured `session_path`
- Signal - `signal-cli` or native libsignal lane
  - SDK/Client: `crates/channels/src/signal.rs`
  - Auth: local Signal identity in configured `data_dir`
- Matrix - homeserver API integration
  - SDK/Client: `crates/channels/src/matrix.rs`
  - Auth: access token or password in channel config
- X/Twitter - X API v2 integration
  - SDK/Client: `crates/channels/src/x_twitter.rs`
  - Auth: bearer token plus user-context keys/secrets
- Twilio - SMS/MMS API integration
  - SDK/Client: `crates/channels/src/twilio.rs`
  - Auth: account SID and auth token
- Meta Messenger / Instagram Direct - Graph API integration
  - SDK/Client: `crates/channels/src/meta.rs`
  - Auth: app ID, app secret, page access token, verify token
- LINE - Messaging API integration
  - SDK/Client: `crates/channels/src/line.rs`
  - Auth: channel access token and channel secret
- Viber - Bot API integration
  - SDK/Client: `crates/channels/src/viber.rs`
  - Auth: auth token
- WeChat Work / Official Accounts - messaging integration
  - SDK/Client: `crates/channels/src/wechat.rs`
  - Auth: corp/app credentials plus optional encryption keys
- iMessage - local macOS lane or BlueBubbles bridge
  - SDK/Client: `crates/channels/src/imessage.rs`
  - Auth: BlueBubbles server URL/password when bridge mode is `bluebubbles`

**Workflow and orchestration services:**
- Python LangGraph sidecar - bounded workflow execution lane for agent orchestration, reminders, scheduler, memory maintenance, and RAG
  - SDK/Client: Rust manager in `crates/langbridge/src/sidecar.rs`, gRPC server in `sidecar/src/server.py`
  - Auth: internal token pair `OPENRUSTCLAW_INTERNAL_API_URL` and `OPENRUSTCLAW_INTERNAL_API_TOKEN`

**Observability services:**
- LangSmith - trace export from both Rust and sidecar
  - SDK/Client: `crates/observability/src/langsmith.rs` and `sidecar/src/langsmith_bridge.py`
  - Auth: `LANGSMITH_API_KEY` or `LANGCHAIN_API_KEY`
- OTLP collectors - trace export via gRPC OTLP
  - SDK/Client: `crates/observability/src/tracing_config.rs`
  - Auth: endpoint-based; no custom app auth detected

## Data Storage

**Databases:**
- SQLite (primary operational store)
  - Connection: `[database].url` in `config/default.toml` or `OPENRUSTCLAW_DATABASE__URL` via `crates/core/src/config.rs`
  - Client: `sqlx` pool in `crates/db/src/pool.rs`
- SQLite-backed memory and RAG tables
  - Connection: same runtime DB URL as above
  - Client: `SqliteMemoryStore`, `SqliteCoreMemoryStore`, and `SqliteRagStore` in `crates/db/src/`
- libSQL / rusqlite
  - Connection: Not actively instantiated in the current visible runtime path
  - Client: documented contract only in `crates/db/src/lib.rs`

**File Storage:**
- Local filesystem only
- Runtime data lives under `./data` or `/app/data` from `Dockerfile` and `docker-compose.yml`
- Runtime secrets vault is file-backed at `.claw/control/runtime-vault.json` through `crates/cli/src/commands/runtime.rs`
- Channel-specific local assets include WhatsApp session files, Signal data, Matrix data, and attachment download directories from `config/default.toml`

**Caching:**
- No external cache service detected
- In-process caching utilities exist via workspace dependency `cached` in `Cargo.toml`

## Authentication & Identity

**Auth Provider:**
- Custom JWT auth for gateway/session tokens
  - Implementation: `crates/security/src/auth.rs`
- Control-plane bearer and trusted-proxy gates
  - Implementation: env-selected token names in `config/default.toml` and `crates/cli/src/commands/start/auth.rs`
- Enterprise SSO adapters
  - Implementation: OIDC in `crates/security/src/sso/oidc.rs` and SAML 2.0 in `crates/security/src/sso/saml.rs`

## Monitoring & Observability

**Error Tracking:**
- None detected as a separate SaaS error tracker
- Runtime tracing exports to LangSmith and OTLP from `crates/observability/src/langsmith.rs` and `crates/observability/src/tracing_config.rs`

**Logs:**
- Structured `tracing` logs across Rust crates from `Cargo.toml`
- Python sidecar uses standard `logging` in `sidecar/src/server.py` and `sidecar/src/langsmith_bridge.py`
- Prometheus metrics exposed from `crates/gateway/src/metrics_endpoint.rs`

## CI/CD & Deployment

**Hosting:**
- Self-hosted binary or Docker runtime from `Dockerfile`
- Optional reverse proxy/TLS stack via `nginx` and `certbot` profiles in `docker-compose.yml`

**CI Pipeline:**
- GitHub Actions CI in `.github/workflows/ci.yml`
- GitHub Actions E2E and scheduled regression suite in `.github/workflows/e2e-tests.yml`
- GitHub Actions tagged release artifact publishing in `.github/workflows/release-binaries.yml`

## Environment Configuration

**Required env vars:**
- Config overrides use the `OPENRUSTCLAW_...` prefix with `__` separators from `crates/core/src/config.rs`
- Runtime provider keys: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `OPENROUTER_API_KEY`, optional `GEMINI_API_KEY`
- Optional local-model override: `OLLAMA_BASE_URL` is consumed by CLI/provider flows in `crates/cli/src/commands/models.rs` and `crates/cli/src/commands/doctor.rs`
- LangSmith: `LANGSMITH_API_KEY`, `LANGSMITH_PROJECT`, optional `LANGSMITH_ENDPOINT`
- OTLP: `OPENRUSTCLAW_OTLP_ENDPOINT` or `OTEL_EXPORTER_OTLP_ENDPOINT`, optional `OPENRUSTCLAW_OTEL_SERVICE_NAME`, `OPENRUSTCLAW_OTEL_SERVICE_VERSION`
- Internal sidecar bridge: `OPENRUSTCLAW_INTERNAL_API_URL`, `OPENRUSTCLAW_INTERNAL_API_TOKEN`
- Runtime vault: `OPENRUSTCLAW_VAULT_PASSPHRASE`
- Control-plane gates: whatever env vars are named by `security.control_api_token_env` and `security.trusted_proxy_token_env` in `config/default.toml`

**Secrets location:**
- Environment variables and config overrides
- Encrypted runtime vault file at `.claw/control/runtime-vault.json` via `crates/cli/src/commands/runtime.rs`
- `.env.example`, `.env.docker`, and `.env` are present in the repo root; treat them as environment inputs, not code

## Webhooks & Callbacks

**Incoming:**
- `/webhooks/slack/events` in `crates/cli/src/commands/start.rs`
- `/webhooks/telegram/events` in `crates/cli/src/commands/start.rs`
- `/webhooks/discord/interactions` in `crates/cli/src/commands/start.rs`
- Teams webhook path from `config.channels.teams.webhook_path`, mounted by `teams_ingress_router` in `crates/cli/src/commands/start.rs`
- Mattermost webhook path from `config.channels.mattermost.webhook_path`, mounted in `crates/cli/src/commands/start.rs`
- `/webhooks/google-chat/events` in `crates/cli/src/commands/start.rs`
- Google Meet webhook path from `config.channels.google_meet.webhook_path`, mounted in `crates/cli/src/commands/start.rs`
- `/webhooks/gmail/pubsub` in `crates/cli/src/commands/start.rs`
- `/webhooks/imessage/bluebubbles` in `crates/cli/src/commands/start.rs`
- Generic webhook router under `/webhooks/{*path}` in `crates/gateway/src/webhooks.rs`
- Internal sidecar-only callbacks under `/internal/memory/*` and `/internal/rag/*` in `crates/gateway/src/server.rs`

**Outgoing:**
- LLM HTTP calls to Anthropic, OpenAI, OpenRouter, Ollama, and Gemini from `crates/providers/src/*.rs`
- Sidecar gRPC calls between Rust and Python from `crates/langbridge/src/sidecar.rs` and `sidecar/src/server.py`
- LangSmith REST calls from `crates/observability/src/langsmith.rs` and `sidecar/src/langsmith_bridge.py`
- OTLP gRPC export from `crates/observability/src/tracing_config.rs`
- Channel API callbacks and sends to Slack, Telegram, Discord, Teams, Mattermost, Google Chat, Google Meet, Gmail, Twilio, Meta, LINE, Viber, WeChat, Matrix, X, and WhatsApp from `crates/channels/src/*.rs`

---

*Integration audit: 2026-04-04*

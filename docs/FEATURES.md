# OpenRustClaw Features

Complete feature listing for OpenRustClaw.

---

## Channel Integrations

| Channel | Status | Notes |
|---------|--------|-------|
| Telegram | ✅ | Polling & webhook modes, rate limiting |
| Discord | ✅ | Slash commands, DMs, Socket Mode |
| Slack | ✅ | App Home, Socket Mode, thread support |
| WhatsApp | ✅ | Via Baileys bridge, QR/pairing auth |
| Microsoft Teams | ✅ | Bot Framework, Azure AD auth |
| Google Chat | ✅ | Service account, Pub/Sub support |
| Gmail Pub/Sub | ✅ | Real-time email notifications |
| Signal | ✅ | signal-cli bridge |
| Matrix | ✅ | matrix-rust-sdk |
| iMessage | ✅ | BlueBubbles server or macOS AppleScript |
| LINE | ✅ | Messaging API |
| Viber | ✅ | Bot API |
| WeChat | ✅ | Work + Official Accounts |
| Messenger | ✅ | Meta Graph API |
| Instagram | ✅ | Meta Graph API |
| SMS (Twilio) | ✅ | Twilio API |
| X (Twitter) | ✅ | X API v2 |
| WebChat | ✅ | Built-in web interface |
| Email | ✅ | IMAP/SMTP |
| IRC | ✅ | Built-in |

**Total: 20 channels**

---

## Voice System

| Feature | Status | Implementation |
|---------|--------|----------------|
| Wake Word Detection | ✅ | Porcupine engine, custom models |
| Talk Mode | ✅ | Continuous conversation mode |
| Speech-to-Text | ✅ | OpenAI Whisper, local models |
| Text-to-Speech | ✅ | OpenAI, ElevenLabs, local |

---

## Visual / Canvas

| Feature | Status | Implementation |
|---------|--------|----------------|
| Live Canvas | ✅ | A2UI workspace, WebSocket |
| Real-time Collaboration | ✅ | Multi-user sessions |
| Visual Elements | ✅ | Text, Image, Chart, Form, Button, Code, Markdown |

---

## Multi-Agent System

| Feature | Status | Description |
|---------|--------|-------------|
| Agent Router | ✅ | Priority-based task routing |
| Capability Discovery | ✅ | Agent skill registry |
| Inter-Agent Tools | ✅ | `sessions_*` tool protocol |
| Heartbeat Scheduler | ✅ | Distributed coordination |

---

## LLM Providers

| Provider | Status | SDK |
|----------|--------|-----|
| Anthropic (Claude) | ✅ | `anthropic_rust` crate |
| OpenAI (GPT-4) | ✅ | `async_openai` crate |
| OpenRouter | ✅ | `openrouter_api` crate |
| Ollama (Local) | ✅ | Direct API |

> **Note**: Additional LLM providers and comprehensive native SDK coverage is planned for future releases.

---

## User Experience

| Feature | Status | Description |
|---------|--------|-------------|
| Interactive Onboarding | ✅ | `openrustclaw onboard` wizard |
| Chat Commands | ✅ | /status, /new, /think, /verbose, /help |
| Skills Registry | ✅ | ClawHub-like skill management |

---

## Integrations

| Feature | Status | Description |
|---------|--------|-------------|
| Webhooks | ✅ | GitHub, Stripe, Generic with HMAC |
| Gmail Pub/Sub | ✅ | Email notifications |

---

## Mobile

| Feature | Status | Description |
|---------|--------|-------------|
| iOS SDK | ✅ | Swift FFI bindings |
| Android SDK | ✅ | Kotlin JNI bindings |

---

## Security

| Feature | Status |
|---------|--------|
| Enterprise SSO (OIDC/SAML) | ✅ |
| HMAC Webhook Verification | ✅ |
| JWT Token Auth | ✅ |
| Channel Allowlists | ✅ |
| Rate Limiting | ✅ |
| Ed25519 Skill Signing | ✅ |

---

## Roadmap

### Completed
- 20 messaging channels
- Voice system
- Live Canvas
- Multi-agent system
- Mobile SDKs

### Planned
- Additional LLM providers (Gemini, Mistral, etc.)
- Enhanced LLM SDK coverage
- Whiteboard / screen sharing
- Voice diarization
- WASM sandbox for skills

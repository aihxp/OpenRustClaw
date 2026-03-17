# OpenRustClaw 🦀

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-300%2B-brightgreen.svg)]()

> **A powerful Rust-based AI agent platform with extensive channel support.**

## 🚀 Quick Start

```bash
# Clone and build
git clone https://github.com/yourusername/OpenRustClaw.git
cd OpenRustClaw
cargo build --release

# Interactive onboarding
./target/release/openrustclaw onboard

# Start with your favorite channels
openrustclaw start --channels=telegram,discord,slack
```

## ✨ Features

### 20 Messaging Channels

| Channel | Status | Features |
|---------|--------|----------|
| Telegram | ✅ | Polling, Webhooks, Commands |
| Discord | ✅ | Gateway, Socket Mode, Reactions |
| Slack | ✅ | App Home, Socket Mode |
| WhatsApp | ✅ | Baileys Bridge, Groups |
| Microsoft Teams | ✅ | Bot Framework, Cards |
| Google Chat | ✅ | Service Account, Threads |
| Gmail | ✅ | Pub/Sub, Label Filters |
| Signal | ✅ | signal-cli Bridge |
| Matrix | ✅ | E2E Encryption, SDK |
| iMessage | ✅ | BlueBubbles, AppleScript |
| LINE | ✅ | Stickers, Rich Menu |
| Viber | ✅ | Keyboard, Welcome Msg |
| WeChat | ✅ | Work + Official Accounts |
| Messenger | ✅ | Templates, Quick Replies |
| Instagram | ✅ | DMs, Story Mentions |
| SMS (Twilio) | ✅ | MMS, Webhooks |
| X/Twitter | ✅ | Mentions, DMs, API v2 |
| WebChat | ✅ | Built-in Interface |
| Email | ✅ | IMAP/SMTP |
| IRC | ✅ | Multi-server |

### Voice System

```bash
openrustclaw talk --wake-word "Hey Assistant"
```

- ✅ **Wake Word** - Porcupine engine
- ✅ **Speech-to-Text** - Whisper integration
- ✅ **Text-to-Speech** - OpenAI, ElevenLabs
- ✅ **Talk Mode** - Continuous conversation

### Live Canvas (A2UI)

```rust
let canvas = Canvas::new("Workspace");
canvas.push(vec![
    CanvasElement::text("Hello"),
    CanvasElement::chart(data),
    CanvasElement::form(fields),
]);
```

- Real-time collaboration via WebSocket
- 8+ element types (Text, Image, Chart, Form, Code, etc.)
- Multi-user sessions

### Multi-Agent System

```rust
// Priority-based routing
let router = AgentRouter::new(agent_id)
    .with_rule("Code Requests", KeywordMatcher::new(vec!["code", "function"]))
    .with_rule("Urgent", ChannelMatcher::new(vec!["slack"]));
```

- Agent Router with 9+ matchers
- Inter-agent communication (`sessions_send`, `sessions_spawn`)
- Heartbeat Scheduler for automation
- Workspace isolation

### LLM Providers

Current LLM support:
- ✅ Anthropic (Claude) via `anthropic_rust` crate
- ✅ OpenAI (GPT-4) via `async_openai` crate
- ✅ OpenRouter via `openrouter_api` crate
- ✅ Ollama (local models)

> **Note**: Additional LLM providers and native SDKs are planned for future releases.

### Chat Commands (All Channels)

```
/status - Session info
/new, /reset - Fresh session
/compact - Compress context
/think <level> - Thinking mode
/verbose on|off - Output level
/help - Show all commands
```

### ClawHub Skills Registry

```bash
# Search and install skills
openrustclaw skills search "github"
openrustclaw skills install github
openrustclaw skills list

# With progress bars and verification
openrustclaw skills install memory --verify
```

### Webhooks

```bash
# GitHub integration
openrustclaw webhooks create github \
  --secret $GITHUB_SECRET \
  --template "{{action}} on {{repository.name}}"

# Test it
openrustclaw webhooks test github --event push
```

### Mobile SDK

```swift
// iOS
let node = MobileNode(config: .init(
    gatewayUrl: "wss://...",
    authToken: "..."
))
try await node.start()
```

```kotlin
// Android
val node = MobileNode(config)
node.start()
```

## 📦 Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     OpenRustClaw                            │
├─────────────┬─────────────┬─────────────┬───────────────────┤
│   Channels  │    Voice    │   Canvas    │    Multi-Agent    │
│  (20 total) │  Wake/STT   │   A2UI      │   Router/Sessions │
├─────────────┴─────────────┴─────────────┴───────────────────┤
│                      Gateway (Axum)                          │
├─────────────────────────────────────────────────────────────┤
│  Agent  │  Skills  │  Memory  │  Scheduler  │  Security     │
├─────────────────────────────────────────────────────────────┤
│  LLM Providers (Claude, GPT, Ollama, OpenRouter)           │
├─────────────────────────────────────────────────────────────┤
│  MCP Client/Server  │  Tools  │  Native SDKs               │
└─────────────────────────────────────────────────────────────┘
```

## 🔧 Configuration

```toml
[channels]
telegram = { enabled = true, token = "${TG_TOKEN}" }
discord = { enabled = true, token = "${DISCORD_TOKEN}" }
slack = { enabled = true, bot_token = "${SLACK_TOKEN}" }
whatsapp = { enabled = true, session_path = "./data/whatsapp" }

[llm]
provider = "anthropic"
api_key = "${ANTHROPIC_KEY}"
model = "claude-3-5-sonnet"

[voice]
wake_word = "Hey Assistant"
stt_provider = "whisper"
tts_provider = "openai"
```

## 🧪 Testing

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p openrustclaw-channels
cargo test -p openrustclaw-agent

# E2E tests
cargo test --test e2e
```

**Current Status**: 300+ tests passing ✅

## 📚 Documentation

- [FEATURES.md](docs/FEATURES.md) - Complete feature matrix
- [CHANNELS.md](docs/CHANNELS.md) - Channel setup guide
- [DEPLOYMENT.md](docs/DEPLOYMENT.md) - Production deployment
- [SECURITY.md](docs/SECURITY.md) - Security best practices

## 🔒 Security

- ✅ Enterprise SSO (OIDC/SAML)
- ✅ HMAC Webhook Verification
- ✅ JWT Authentication
- ✅ Channel Allowlists
- ✅ Rate Limiting
- ✅ Ed25519 Skill Signing
- ✅ Secret Scanning (GitLeaks)

## 🤝 Contributing

```bash
# Setup
git clone https://github.com/yourusername/OpenRustClaw.git
cd OpenRustClaw
cargo build

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy -- -D warnings
```

## 📄 License

MIT License - See [LICENSE](LICENSE)

## 🙏 Acknowledgments

- Built with [Rust](https://www.rust-lang.org)
- LLM integrations: Anthropic, OpenAI, Ollama, OpenRouter

---

**🦀 OpenRustClaw - A powerful Rust-based AI agent platform** 🚀

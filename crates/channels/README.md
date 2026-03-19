# OpenRustClaw Channels

Messaging platform integrations for the OpenRustClaw AI agent.

## Supported Channels

| Channel | Status | Features |
|---------|--------|----------|
| WebChat | ✅ Ready | WebSocket-based chat |
| Telegram | ✅ Ready | Auth probe, outbound send, Bot API polling receive, and local agent/session routing |
| Discord | ⚠️ Partial | Outbound send plus verified Interactions HTTP ingress; full Gateway message-event runtime is still incomplete |
| Slack | ✅ Ready | Auth probe, outbound send, built-in Events API ingress, and local agent/session routing |
| Microsoft Teams | ⚠️ Partial | Bot Framework webhook ingress, JWT verification, outbound sends, attachment metadata capture, adaptive-card file links, and local agent/session routing are on the shipped runtime path |
| Google Chat | ⚠️ Partial | Webhook ingress, token-backed or service-account outbound auth, response-mode gating, slash-command plus mention metadata capture, file-reference cards, attachment metadata capture, and local agent/session routing are on the shipped runtime path |
| WhatsApp | ✅ Ready | Baileys bridge pairing/QR, group/DM routing, mentions, replies, media send/receive, delivery acknowledgements, and local agent/session routing |
| Gmail Pub/Sub | ⚠️ Partial | Gmail watch setup, Pub/Sub webhook ingress, history fetch, message hydration, structured attachment metadata, direct sends, reply attachments, mail-triggered local agent/session routing, label/archive/delete actions, and forwarding are on the shipped runtime path |
| Matrix | ⚠️ Partial | Access-token or password auth, `/sync` polling ingress, outbound room sends, reply/thread relations, inbound reaction events, room join/leave, joined-room inspection, file uploads, and local agent/session routing are on the shipped runtime path |
| Signal | ⚠️ Partial | `signal-cli` daemon-backed direct/group send-receive, allowlist handling, registration/verify/link helpers, normalized route metadata, attachment/file-reference capture, normalized group-member and mention metadata, receipt lifecycle events, and local agent/session routing are on the shipped runtime path |
| Meta (Messenger/Instagram) | 🚧 Gated | Repo surface exists, not part of the current shipped runtime |
| LINE | 🚧 Gated | Repo surface exists, not part of the current shipped runtime |
| Viber | 🚧 Gated | Repo surface exists, not part of the current shipped runtime |
| WeChat | 🚧 Gated | Repo surface exists, not part of the current shipped runtime |
| iMessage | ⚠️ Partial | BlueBubbles/macOS direct send, BlueBubbles inbound webhook ingress, tapbacks including normal channel-send reactions, group/participant metadata normalization, structured attachment metadata, contact routing, and local agent/session routing are on the shipped runtime path |

## Quick Start

```rust
use openrustclaw_channels::ChannelFactory;
use openrustclaw_core::config::ChannelsConfig;

// Load configuration
let config = ChannelsConfig::default();

// Create enabled channels
let channels = ChannelFactory::create_channels(&config);

for channel in channels {
    println!("Created channel: {:?}", channel.platform());
}
```

## Creating a Channel

```rust
use openrustclaw_channels::TelegramChannel;
use openrustclaw_core::config::TelegramConfig;

let config = TelegramConfig {
    enabled: true,
    token: std::env::var("TELEGRAM_TOKEN")?,
    ..Default::default()
};

let channel = ChannelFactory::create_telegram(config)?;
```

## Implementing a Custom Channel

```rust
use openrustclaw_core::traits::Channel;
use async_trait::async_trait;

pub struct MyChannel {
    // ...
}

#[async_trait]
impl Channel for MyChannel {
    fn platform(&self) -> Platform {
        Platform::Custom("my_platform".to_string())
    }
    
    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        // Implementation
    }
    
    async fn receive(&self) -> Result<IncomingMessage> {
        // Implementation
    }
}
```

## Security

All channels implement:
- Rate limiting
- Input validation
- Authentication verification
- Secure credential handling

## License

MIT OR Apache-2.0

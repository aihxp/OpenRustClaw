# OpenRustClaw Channels

Messaging platform integrations for the OpenRustClaw AI agent.

## Supported Channels

| Channel | Status | Features |
|---------|--------|----------|
| WebChat | ✅ Ready | WebSocket-based chat |
| Telegram | ✅ Ready | Auth probe, outbound send, Bot API polling receive, and local agent/session routing |
| Discord | ⚠️ Partial | Outbound send plus verified Interactions HTTP ingress; full Gateway message-event runtime is still incomplete |
| Slack | ✅ Ready | Auth probe, outbound send, built-in Events API ingress, and local agent/session routing |
| Microsoft Teams | 🚧 Gated | Repo surface exists, not part of the current shipped runtime |
| Google Chat | ⚠️ Partial | Channel scaffolding present; service-account auth not implemented |
| WhatsApp | 🚧 Gated | Repo surface exists, not part of the current shipped runtime |
| Gmail Pub/Sub | ⚠️ Partial | Channel scaffolding present; service-account auth not implemented |
| Matrix | ⚠️ Partial | Config/model scaffolding present; matrix-sdk runtime client not implemented |
| Meta (Messenger/Instagram) | 🚧 Gated | Repo surface exists, not part of the current shipped runtime |
| LINE | 🚧 Gated | Repo surface exists, not part of the current shipped runtime |
| Viber | 🚧 Gated | Repo surface exists, not part of the current shipped runtime |
| WeChat | 🚧 Gated | Repo surface exists, not part of the current shipped runtime |
| iMessage | 🚧 Gated | Repo surface exists, not part of the current shipped runtime |

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

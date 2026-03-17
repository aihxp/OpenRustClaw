# OpenRustClaw Channels

Messaging platform integrations for the OpenRustClaw AI agent.

## Supported Channels

| Channel | Status | Features |
|---------|--------|----------|
| WebChat | ✅ Ready | WebSocket-based chat |
| Telegram | ⚠️ Partial | Config/model scaffolding present; runtime client/send path not implemented |
| Discord | ⚠️ Partial | Config/model scaffolding present; runtime client/send path not implemented |
| Slack | ⚠️ Partial | Config/model scaffolding present; runtime client/send path not implemented |
| Microsoft Teams | ✅ Ready | Bot Framework, Adaptive Cards |
| Google Chat | ⚠️ Partial | Channel scaffolding present; service-account auth not implemented |
| WhatsApp | ✅ Ready | Web bridge, media |
| Gmail Pub/Sub | ⚠️ Partial | Channel scaffolding present; service-account auth not implemented |
| Matrix | ⚠️ Partial | Config/model scaffolding present; matrix-sdk runtime client not implemented |
| Meta (Messenger/Instagram) | ✅ Ready | Graph API |
| LINE | ✅ Ready | Messaging API |
| Viber | ✅ Ready | Bot API |
| WeChat | ✅ Ready | Work & Official Accounts |
| iMessage | ✅ Ready | macOS integration |

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

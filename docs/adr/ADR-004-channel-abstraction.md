# ADR-004: Channel Abstraction for Multi-Platform Support

## Status
Accepted

## Context
OpenRustClaw needed to support multiple messaging platforms (Slack, Discord, Telegram, etc.) with:
- Unified interface for message handling
- Platform-specific features
- Easy addition of new platforms

## Decision
We created a `Channel` trait with platform-specific implementations:

```rust
#[async_trait]
pub trait Channel: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    async fn send(&self, message: OutgoingMessage) -> Result<()>;
    async fn receive(&self) -> Result<IncomingMessage>;
    fn platform(&self) -> Platform;
}
```

### Channel Factory
```rust
pub struct ChannelFactory;

impl ChannelFactory {
    pub fn create_channels(config: &ChannelsConfig) -> Vec<Box<dyn Channel>> {
        // Instantiate enabled channels based on config
    }
}
```

### Platform Adapters
Each platform has its own crate:
- `telegram.rs`: Teloxide integration
- `discord.rs`: Serenity integration
- `slack.rs`: Slack-morphism integration
- etc.

## Consequences

### Positive
- **Extensibility**: New platforms by implementing trait
- **Testability**: Mock channels for testing
- **Consistency**: Same agent logic across platforms
- **Type Safety**: Compile-time verification

### Negative
- **Boilerplate**: Each platform needs adapter
- **Feature parity**: Some platform features don't map cleanly
- **Dependencies**: Each adapter adds dependencies

## Implementation

See [channels crate](../../crates/channels/src/lib.rs) for full implementation.

## Supported Platforms

| Platform | Library | Status |
|----------|---------|--------|
| WebChat | Built-in | ✅ Stable |
| Telegram | teloxide | ✅ Stable |
| Discord | serenity | ✅ Stable |
| Slack | slack-morphism | ✅ Stable |
| Teams | reqwest | ✅ Stable |
| WhatsApp | Baileys bridge | ⚠️ Beta |
| Matrix | matrix-rust-sdk | ⚠️ Beta |

## References
- [Channels README](../../crates/channels/README.md)

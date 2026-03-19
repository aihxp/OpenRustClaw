# OpenRustClaw Channels

Messaging platform integrations for the OpenRustClaw AI agent.

## Supported Channels

| Channel | Status | Features |
|---------|--------|----------|
| WebChat | ✅ Ready | WebSocket-based chat |
| Telegram | ✅ Ready | Auth probe, outbound send, Bot API polling receive, and local agent/session routing |
| Discord | ⚠️ Partial | Outbound send plus verified Interactions HTTP ingress; full Gateway message-event runtime is still incomplete |
| Slack | ✅ Ready | Auth probe, outbound send, built-in Events API ingress, and local agent/session routing |
| Microsoft Teams | ✅ Ready | Bot Framework webhook ingress, JWT verification, connector-`/v3` outbound sends, native local-file uploads through Connector attachments, metadata-driven typing/update/delete actions, normalized reply/reaction/lifecycle metadata, normalized member/reaction presence/count metadata, normalized team/channel/tenant IDs, richer message body/reply/update/delete flags, mention presence/count metadata, richer attachment metadata plus structured file references, attachment name/type/url presence flags, attachment URL counts, file-reference counts, local download paths, adaptive-card file links, and local agent/session routing are on the shipped runtime path |
| Mattermost | ✅ Ready | Rust-native outbound sends, slash-command and outgoing-webhook ingress, shared-token validation, user/channel allowlists, trigger-word stripping, bot-mention detection, threaded replies via `root_id`, REST-backed local file uploads from `file_references`, and local agent/session routing are on the shipped runtime path |
| Google Chat | ⚠️ Partial | Webhook ingress, Pub/Sub push-envelope decoding, token-backed or service-account outbound auth, response-mode gating, normalized space/user IDs, slash-command plus mention metadata capture, message body/thread/argument/slash/mention presence metadata, user-email and space-display-name presence flags, card-click and space lifecycle routing, card action parameter plus form-input metadata and action-parameter presence flags, structured attachment metadata capture with attachment-name/type/data-ref presence flags and file-reference counts, and local agent/session routing are on the shipped runtime path |
| Google Meet | ✅ Ready | Native Rust Meet support covers space creation/inspection, active-conference termination, conference-record/participant/recording/transcript inspection, direct-event plus Google Workspace Events / Pub/Sub payload decoding with transcript hydration, live webhook ingress on `openrustclaw start`, and durable runtime-event publication for workflow automation |
| WhatsApp | ✅ Ready | Baileys bridge pairing/QR, group/DM routing, mentions, replies, media send/receive, delivery acknowledgements, and local agent/session routing |
| Gmail Pub/Sub | ⚠️ Partial | Gmail watch setup, Pub/Sub webhook ingress, history fetch, message hydration, structured attachment metadata, normalized recipient/label counts plus recipient domains, recipient/domain/label/header presence flags, file-reference counts, attachment total-size metadata, unread/received-at metadata, parsed header/thread metadata, history/body-length metadata, direct sends, reply attachments, mail-triggered local agent/session routing, label/archive/delete actions, and forwarding are on the shipped runtime path |
| Matrix | ✅ Ready | Access-token or password auth, `/sync` polling ingress, outbound room sends, reply/thread relations with presence flags, formatted-body and media-presence metadata, richer inbound reaction/redaction/membership lifecycle metadata plus target/reason/avatar/display-name presence flags, room join/leave, joined-room inspection, file uploads, inbound media download metadata plus media MIME/size fields and download/content-uri/filename/size presence flags, first-class CLI operator flows for join/leave/list/send-formatted/react/send-file/typing/redact, and local agent/session routing are on the shipped runtime path |
| Signal | ✅ Ready | `signal-cli` daemon-backed direct/group send-receive, allowlist handling, first-class CLI operator flows for registration/verify/link/list-groups, normalized route metadata, source/group presence flags, attachment/file-reference capture with daemon-provided local attachment path enrichment where available, attachment-only inbound routing, normalized attachment ids/names/mime types plus caption counts and attachment-id/name presence flags, flat quote plus reply-target metadata with quote presence/text-length flags, sync-message lifecycle visibility, normalized group-member and mention metadata with presence flags, receipt lifecycle events, and local agent/session routing are on the shipped runtime path |
| Meta (Messenger/Instagram) | 🚧 Gated | Repo surface exists, not part of the current shipped runtime |
| LINE | 🚧 Gated | Repo surface exists, not part of the current shipped runtime |
| Viber | 🚧 Gated | Repo surface exists, not part of the current shipped runtime |
| WeChat | 🚧 Gated | Repo surface exists, not part of the current shipped runtime |
| iMessage | ✅ Ready | BlueBubbles/macOS direct send, verified BlueBubbles ping on connect, webhook-password validation for BlueBubbles ingress, direct-handle or chat-guid targeting, structured group/chat mapping from webhook payloads, structured attachment metadata plus outbound local-file sends in BlueBubbles mode, tapbacks including normal channel-send reactions, first-class CLI operator flows for ping/server/chats/contacts/send/send-file/tapback, and local agent/session routing are on the shipped runtime path |

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

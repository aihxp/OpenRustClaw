# Baileys Bridge for OpenRustClaw

This Node.js bridge enables WhatsApp Web integration for OpenRustClaw using the [Baileys](https://github.com/WhiskeySockets/Baileys) library.

## Overview

The bridge acts as a proxy between the Rust OpenRustClaw agent and WhatsApp Web:

```
OpenRustClaw (Rust) <--JSON-RPC--> Baileys Bridge (Node.js) <--WebSocket--> WhatsApp Web
```

## Installation

### Prerequisites

- Node.js 18+ (required for Baileys)
- npm or yarn

### Setup

```bash
cd crates/channels/baileys-bridge
npm install
```

## Usage

The bridge is automatically started by the Rust WhatsApp channel when you call `connect()`. Manual execution is only needed for testing.

### Manual Testing

```bash
node index.js
```

Then send JSON commands via stdin:

```json
{"type": "connect", "session_path": "/tmp/whatsapp-session", "pairing_mode": false}
```

## Protocol

Communication happens over stdin/stdout using newline-delimited JSON.

### Messages from Rust to Bridge

| Type | Description |
|------|-------------|
| `connect` | Start WhatsApp connection |
| `send_message` | Send a text message |
| `send_media` | Send a media message |
| `ping` | Keepalive ping |
| `disconnect` | Gracefully disconnect |

### Messages from Bridge to Rust

| Type | Description |
|------|-------------|
| `connected` | Successfully connected |
| `qr_code` | QR code for pairing |
| `pairing_code` | Pairing code for linking |
| `disconnected` | Connection closed |
| `message` | Incoming text message |
| `media_message` | Incoming media message |
| `message_sent` | Confirm message delivery |
| `error` | Error response |
| `pong` | Ping response |

## Supported Features

### Message Types

- **Text**: Plain text and extended text messages
- **Image**: Photos with optional captions
- **Video**: Video files
- **Audio**: Audio files
- **Voice**: Voice notes (PTT)
- **Document**: Any file type
- **Sticker**: Animated and static stickers
- **Location**: GPS coordinates
- **Contact**: Shared contacts
- **Poll**: Poll creation messages

### Group Support

- Receive messages from groups
- Group metadata (name, participants)
- Mention handling
- Quoted message context

### Connection Modes

1. **QR Code** (default): Scan QR code from WhatsApp mobile app
2. **Pairing Code**: Link with 8-character code (desktop app style)

## Session Management

Session credentials are stored in the configured `session_path` directory:

```
/sessions/
  ├── creds.json          # Authentication credentials
  ├── pre-keys.json       # Encryption keys
  ├── sender-keys/        # Group encryption keys
  └── baileys_store.json  # Message store cache
```

Sessions persist across restarts - you only need to pair once.

## Rate Limits

WhatsApp has rate limits to prevent spam:

- DMs: ~60 messages/minute
- Groups: ~15 messages/minute
- Media: Larger files have longer delays

The Rust channel includes a rate limiter to respect these limits.

## Error Handling

Common error codes:

| Code | Description |
|------|-------------|
| `START_ERROR` | Failed to start connection |
| `SEND_ERROR` | Failed to send message |
| `SEND_MEDIA_ERROR` | Failed to send media |
| `NOT_CONNECTED` | Not connected to WhatsApp |
| `PARSE_ERROR` | Invalid message from Rust |
| `UNCAUGHT_EXCEPTION` | Internal bridge error |

## Security Considerations

1. **Session Storage**: Keep `session_path` secure - credentials allow account access
2. **Allowlist**: Use the `allowlist` config to restrict DMs to known numbers
3. **Group Security**: Group messages bypass allowlist (handle in your agent logic)
4. **Media Downloads**: Media is not auto-downloaded; URLs are provided when available

## Debugging

Enable debug logging:

```bash
LOG_LEVEL=debug node index.js
```

Or in Rust:

```rust
std::env::set_var("RUST_LOG", "debug");
```

## Troubleshooting

### "Baileys bridge not found"

Ensure you've run `npm install` in the `baileys-bridge` directory.

### "Node.js is required but not found"

Install Node.js 18+ from [nodejs.org](https://nodejs.org/).

### Connection keeps dropping

- Check internet connectivity
- Verify phone has WhatsApp installed and running
- Try clearing the session directory and re-pairing

### QR code not appearing

- Check stderr logs for errors
- Ensure terminal supports the output
- Try pairing mode instead

## License

MIT - See LICENSE file in project root.

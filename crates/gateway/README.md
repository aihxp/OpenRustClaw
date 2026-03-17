# OpenRustClaw Gateway

HTTP/WebSocket gateway for the OpenRustClaw AI agent platform.

## Overview

The gateway provides:

- **REST API**: HTTP endpoints for all functionality
- **WebSocket**: Real-time bidirectional communication
- **Webhook Handlers**: Receive events from external services
- **Rate Limiting**: Protect against abuse
- **Authentication**: JWT and API key authentication

## API Endpoints

### Messages

- `POST /api/v1/messages` - Send a message
- `GET /api/v1/messages/:id` - Get message by ID
- `DELETE /api/v1/messages/:id` - Delete a message

### Sessions

- `POST /api/v1/sessions` - Create a session
- `GET /api/v1/sessions/:id` - Get session
- `DELETE /api/v1/sessions/:id` - End session

### Webhooks

- `POST /webhooks/:provider` - Receive webhook events

## Quick Start

```rust
use openrustclaw_gateway::server::GatewayServer;
use openrustclaw_core::config::GatewayConfig;

let config = GatewayConfig::default();
let server = GatewayServer::new(config).await?;

server.run().await?;
```

## WebSocket Protocol

```json
{
  "type": "message",
  "payload": {
    "content": "Hello!",
    "session_id": "..."
  }
}
```

## License

MIT OR Apache-2.0

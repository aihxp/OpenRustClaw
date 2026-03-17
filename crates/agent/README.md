# OpenRustClaw Agent

AI agent runtime and orchestration for OpenRustClaw.

## Overview

The agent crate provides the core AI agent functionality:

- **Message Processing**: Handle incoming messages and generate responses
- **Tool Execution**: Execute tools and skills on behalf of the user
- **Memory Management**: Short-term and long-term memory for conversations
- **Routing**: Route requests to appropriate handlers

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Gateway   │────▶│    Agent    │────▶│   LLM API   │
└─────────────┘     └─────────────┘     └─────────────┘
                           │
                    ┌──────┴──────┐
                    ▼             ▼
              ┌─────────┐   ┌──────────┐
              │  Tools  │   │  Memory  │
              └─────────┘   └──────────┘
```

## Quick Start

```rust
use openrustclaw_agent::AgentRuntime;
use openrustclaw_core::config::AgentConfig;

let config = AgentConfig::default();
let agent = AgentRuntime::new(config).await?;

// Process a message
let response = agent.process_message(message).await?;
```

## Features

- Multi-turn conversations
- Context window management
- Tool calling with structured outputs
- Streaming responses
- Conversation memory

## License

MIT OR Apache-2.0

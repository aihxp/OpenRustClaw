# OpenRustClaw Core

Core types, traits, and configuration for the OpenRustClaw AI agent platform.

## Overview

This crate provides the foundational components used by all other OpenRustClaw crates:

- **Types**: Core data structures (Message, Session, etc.)
- **Traits**: Abstractions for channels, memory, and skills
- **Configuration**: Configuration structures for all components
- **Errors**: Error types and handling utilities

## Key Components

### Types

```rust
use openrustclaw_core::types::{Message, Session, Platform};

let msg = Message::user("Hello, AI!");
let session = Session::new_dm("user123", Platform::WebChat);
```

### Configuration

```rust
use openrustclaw_core::config::{Config, ChannelsConfig};

let config = Config::load_from_file("config.yaml")?;
```

### Traits

```rust
use openrustclaw_core::traits::{Channel, MemoryStore};
```

## Architecture

The core crate follows a layered architecture:

1. **Types Layer**: Domain models and data structures
2. **Traits Layer**: Interface definitions
3. **Config Layer**: Configuration management
4. **Error Layer**: Error handling and propagation

## Features

- `serde`: Serialization support (enabled by default)
- `uuid`: UUID generation for entities
- `chrono`: Timestamp handling

## License

MIT OR Apache-2.0

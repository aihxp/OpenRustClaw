# OpenRustClaw Skills

Skill management and execution for the OpenRustClaw AI agent.

## Overview

The skills crate provides:

- **Skill Registry**: Manage available skills
- **Skill Loader**: Load skills from various sources
- **Skill Sandbox**: Secure WASM-based execution
- **Skill Marketplace**: Integration with skill marketplace

## Skill Structure

```yaml
# skill.yaml
name: weather
version: 1.0.0
description: Get weather information
author: OpenRustClaw Team
capabilities:
  - http_client
  - filesystem_read

entrypoint: skill.wasm
```

## Creating a Skill

```rust
use openrustclaw_skills::{Skill, SkillManifest};

let manifest = SkillManifest {
    name: "weather".to_string(),
    version: "1.0.0".to_string(),
    description: "Get weather information".to_string(),
    capabilities: vec!["http_client".to_string()],
};

let skill = Skill::new(manifest, wasm_bytes)?;
```

## Security

Skills run in a WebAssembly sandbox with:
- Memory isolation
- CPU time limits
- Capability-based permissions
- Network access controls

## License

MIT OR Apache-2.0

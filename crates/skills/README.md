# OpenRustClaw Skills

Skill management and execution for the OpenRustClaw AI agent.

## Overview

The skills crate provides:

- **Skill Registry**: Manage available skills
- **Skill Loader**: Load skills from various sources
- **Skill Sandbox**: WASM sandbox scaffolding and capability checks
- **Skill Marketplace**: ClawHub registry integration plus marketplace scaffolding

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

The crate currently provides capability metadata and verification plumbing.
The WASM executor is planned, but not yet implemented.

## License

MIT OR Apache-2.0

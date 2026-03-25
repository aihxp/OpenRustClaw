# OpenRustClaw Skills

Skill management and execution for the OpenRustClaw AI agent.

## Overview

The skills crate provides:

- **Skill Registry**: Manage available skills
- **Skill Loader**: Load skills from various sources
- **Skill Compiler**: Emit cached help/schema/extension artifacts under `.claw/skills/compiled/`
- **Skill Sandbox**: Real WASM sandbox execution plus capability checks
- **Skill Marketplace**: ClawHub registry integration plus marketplace scaffolding
- **Extension Manifest Layer**: Define Rust-native extension/runtime contracts for later Phase 7 work

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

## Security

The crate now provides:

- capability metadata and verification plumbing
- compile-time scan reports and blocked-status handling
- a real WASM executor with timeout and memory limits
- explicit extension manifests for future Rust-native plugin/runtime growth

## License

MIT OR Apache-2.0

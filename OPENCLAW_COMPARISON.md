# OpenClaw vs OpenRustClaw: Feature Comparison

This document provides a detailed side-by-side comparison of OpenClaw and OpenRustClaw features.

## Executive Summary

| Aspect | OpenClaw | OpenRustClaw | Status |
|--------|----------|--------------|--------|
| **Language** | TypeScript/Node.js | Rust + Python | ✅ Rust for performance |
| **Stars** | 200,000+ | New | Growing |
| **Architecture** | Gateway + Pi runtime | 19 crates + sidecar | ✅ More modular |
| **Security** | CVE-2026-25253 | Fixed | ✅ Hardened |
| **Memory** | MEMORY.md injection | 3-tier recall-only | ✅ 93.5% token savings |

---

## Detailed Feature Comparison

### 1. Channel Integrations

| Channel | OpenClaw | OpenRustClaw | Priority |
|---------|----------|--------------|----------|
| WhatsApp | ✅ Baileys | ❌ | **HIGH** |
| Telegram | ✅ grammY | ✅ teloxide | ✅ Done |
| Slack | ✅ Bolt | ✅ slack-morphism | ✅ Done |
| Discord | ✅ discord.js | ✅ serenity | ✅ Done |
| Google Chat | ✅ Chat API | ❌ | Medium |
| Signal | ✅ signal-cli | ❌ | Medium |
| BlueBubbles (iMessage) | ✅ | ❌ | Medium |
| iMessage (legacy) | ✅ | ❌ | Low |
| IRC | ✅ | ❌ | Low |
| Microsoft Teams | ✅ Bot Framework | ❌ | **HIGH** |
| Matrix | ✅ | ❌ | Low |
| Feishu | ✅ | ❌ | Low |
| LINE | ✅ | ❌ | Low |
| Mattermost | ✅ | ❌ | Low |
| Nextcloud Talk | ✅ | ❌ | Low |
| Nostr | ✅ | ❌ | Low |
| Synology Chat | ✅ | ❌ | Low |
| Tlon | ✅ | ❌ | Low |
| Twitch | ✅ | ❌ | Low |
| Zalo | ✅ | ❌ | Low |
| WebChat | ✅ Built-in | ✅ Gateway WS | ✅ Done |

**Gap Analysis**: OpenRustClaw has 3/20 channels. Need WhatsApp, Teams, Google Chat, Signal.

---

### 2. Voice & Audio

| Feature | OpenClaw | OpenRustClaw | Priority |
|---------|----------|--------------|----------|
| Voice Wake | ✅ macOS/iOS | ❌ | **HIGH** |
| Talk Mode | ✅ Continuous | ❌ | **HIGH** |
| Android Voice | ✅ | ❌ | Medium |
| TTS (Text-to-Speech) | ✅ ElevenLabs | ❌ | Medium |
| Transcription | ✅ | ❌ | Medium |

---

### 3. Visual Interface

| Feature | OpenClaw | OpenRustClaw | Priority |
|---------|----------|--------------|----------|
| Live Canvas | ✅ A2UI | ❌ | **HIGH** |
| WebChat UI | ✅ | ✅ Gateway | ✅ Done |
| Control UI | ✅ | ❌ | Medium |
| Dashboard | ✅ | ❌ | Medium |

---

### 4. Browser & Automation

| Feature | OpenClaw | OpenRustClaw | Priority |
|---------|----------|--------------|----------|
| Chrome CDP Control | ✅ Dedicated | ✅ automation crate | ✅ Done |
| Browser Profiles | ✅ | ❌ | Medium |
| Snapshots | ✅ | ❌ | Low |
| File Uploads | ✅ | ❌ | Low |

---

### 5. Device Integration (Nodes)

| Feature | OpenClaw | OpenRustClaw | Priority |
|---------|----------|--------------|----------|
| iOS Node | ✅ Camera, Voice | ❌ | **HIGH** |
| Android Node | ✅ Full device | ❌ | **HIGH** |
| macOS Node | ✅ system.run | ❌ | Medium |
| Camera Access | ✅ | ❌ | Medium |
| Screen Recording | ✅ | ❌ | Medium |
| Location | ✅ | ❌ | Low |
| Notifications | ✅ | ❌ | Low |

---

### 6. Agent System

| Feature | OpenClaw | OpenRustClaw | Priority |
|---------|----------|--------------|----------|
| Pi Runtime | ✅ Minimal (4 tools) | ❌ | Medium |
| Multi-agent Routing | ✅ | ❌ | **HIGH** |
| Agent-to-Agent Tools | ✅ sessions_* | ❌ | **HIGH** |
| Session Spawning | ✅ | ❌ | Medium |
| Workspace Isolation | ✅ | ✅ Per-session | ✅ Done |

---

### 7. Skills & Extensibility

| Feature | OpenClaw | OpenRustClaw | Priority |
|---------|----------|--------------|----------|
| ClawHub Registry | ✅ | ❌ | **HIGH** |
| SKILL.md Format | ✅ | ✅ | ✅ Done |
| WASM Sandboxing | ❌ | ✅ wasmtime | ✅ Better |
| Ed25519 Signing | ❌ | ✅ | ✅ Better |
| Auto-install Skills | ✅ | ❌ | Medium |

---

### 8. Scheduling & Automation

| Feature | OpenClaw | OpenRustClaw | Priority |
|---------|----------|--------------|----------|
| Heartbeat Scheduler | ✅ Unprompted | ❌ | **HIGH** |
| Durable Scheduler | ❌ | ✅ | ✅ Better |
| Cron Jobs | ✅ | ✅ | ✅ Done |
| Webhooks | ✅ External | ❌ | **HIGH** |
| Gmail Pub/Sub | ✅ | ❌ | Medium |

---

### 9. Security Model

| Feature | OpenClaw | OpenRustClaw | Priority |
|---------|----------|--------------|----------|
| Origin Validation | ❌ CVE-2026-25253 | ✅ Fixed | ✅ Better |
| JWT Auth | ✅ | ✅ | ✅ Done |
| Prompt Injection Defense | ❌ 17% | ✅ Multi-layer | ✅ Better |
| Docker Sandboxing | ✅ | ❌ | Medium |
| Per-session FS Isolation | ❌ | ✅ | ✅ Better |
| SSO/OIDC | ❌ | ✅ | ✅ Better |
| SAML | ❌ | ✅ | ✅ Better |

---

### 10. Deployment & Operations

| Feature | OpenClaw | OpenRustClaw | Priority |
|---------|----------|--------------|----------|
| Docker | ✅ | ✅ | ✅ Done |
| Kubernetes | ❌ | ✅ Helm | ✅ Better |
| Terraform | ❌ | ✅ AWS/GCP/Azure | ✅ Better |
| Tailscale Integration | ✅ Serve/Funnel | ❌ | Medium |
| Nix Mode | ✅ | ❌ | Low |

---

### 11. CLI & Developer Experience

| Feature | OpenClaw | OpenRustClaw | Priority |
|---------|----------|--------------|----------|
| Interactive Onboarding | ✅ `openclaw onboard` | ❌ | **HIGH** |
| Chat Commands | ✅ /status, /new, etc | ❌ | **HIGH** |
| Doctor/Diagnostics | ✅ | ✅ | ✅ Done |
| macOS Menu Bar | ✅ | ❌ | Low |
| TUI | ❌ | ✅ Ratatui | ✅ Better |

---

### 12. Memory System

| Feature | OpenClaw | OpenRustClaw | Priority |
|---------|----------|--------------|----------|
| Core Memory | ✅ | ✅ | ✅ Done |
| Recall Memory | ✅ | ✅ | ✅ Done |
| Archive | ✅ | ✅ | ✅ Done |
| Auto-injection | ✅ MEMORY.md | ❌ Recall-only | ✅ Better |
| Token Efficiency | ❌ 93.5% waste | ✅ Optimized | ✅ Better |

---

## Priority Matrix

### Critical (P0) - Must Have for Feature Parity
1. **WhatsApp Integration** - Most requested channel
2. **Voice Wake + Talk Mode** - Differentiating feature
3. **Live Canvas/A2UI** - Visual workspace
4. **Interactive Onboarding** - User experience
5. **Chat Commands** - Power user feature
6. **Multi-agent Routing** - Core architecture
7. **Agent-to-Agent Communication** - sessions_* tools
8. **Heartbeat Scheduler** - Unprompted automation

### High (P1) - Important for Adoption
1. Microsoft Teams
2. Google Chat
3. iOS/Android Nodes
4. Webhooks
5. ClawHub Skills Registry
6. Gmail Pub/Sub

### Medium (P2) - Nice to Have
1. Signal
2. Docker Sandboxing
3. Browser Profiles
4. macOS Menu Bar
5. Tailscale Integration

### Low (P3) - Future Consideration
1. IRC, Matrix, Nostr (niche channels)
2. Nix Mode
3. Legacy iMessage
4. Feishu, LINE, Zalo (regional)

---

## Recommended Implementation Order

### Phase 1: Core Channels (Weeks 1-4)
- [ ] WhatsApp (Baileys integration)
- [ ] Microsoft Teams
- [ ] Google Chat

### Phase 2: Voice & Canvas (Weeks 5-8)
- [ ] Voice Wake (macOS/iOS)
- [ ] Talk Mode
- [ ] Live Canvas/A2UI

### Phase 3: Agent System (Weeks 9-12)
- [ ] Multi-agent routing
- [ ] Agent-to-agent tools (sessions_*)
- [ ] Heartbeat scheduler

### Phase 4: DX & Onboarding (Weeks 13-16)
- [ ] Interactive onboarding
- [ ] Chat commands
- [ ] ClawHub registry

### Phase 5: Device Integration (Weeks 17-20)
- [ ] iOS Node
- [ ] Android Node
- [ ] Webhooks

---

## Summary

OpenRustClaw has **superior**:
- Security (fixes CVE-2026-25253)
- Memory efficiency (93.5% token savings)
- Deployment (K8s, Terraform)
- Code quality (Rust vs TypeScript)
- SSO (OIDC/SAML)

OpenRustClaw is **missing**:
- 17 channel integrations
- Voice features
- Live Canvas
- Device nodes
- Interactive onboarding
- Chat commands
- Heartbeat scheduler

**Recommendation**: Focus on P0 features to achieve feature parity, then leverage Rust advantages for performance and security leadership.

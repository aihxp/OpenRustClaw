# OpenRustClaw Implementation Plan: Achieving Feature Parity

This document outlines the detailed plan to implement missing OpenClaw features in OpenRustClaw.

---

## Phase 1: Critical Channels (4 weeks)

### 1.1 WhatsApp Integration
**Crate**: `crates/channels/src/whatsapp.rs`

**Implementation**:
```rust
// Use whatsapp-rs or baileys-rs (Rust bindings)
pub struct WhatsAppChannel {
    client: WhatsAppClient,
    config: WhatsAppConfig,
    message_handler: Arc<MessageHandler>,
}

#[async_trait]
impl Channel for WhatsAppChannel {
    async fn connect(&mut self) -> Result<()>;
    async fn send(&self, message: OutgoingMessage) -> Result<()>;
    async fn receive(&self) -> Result<IncomingMessage>;
    fn platform(&self) -> Platform { Platform::WhatsApp }
}
```

**Dependencies**:
- `whatsapp-rs` or custom Baileys bridge
- QR code generation for pairing
- SQLite for credential storage

**Tasks**:
- [ ] QR code pairing flow
- [ ] Message sending/receiving
- [ ] Group chat support
- [ ] Media handling (images, audio)
- [ ] DM allowlist (pairing mode)

---

### 1.2 Microsoft Teams Integration
**Crate**: `crates/channels/src/teams.rs`

**Implementation**:
```rust
// Bot Framework SDK integration
pub struct TeamsChannel {
    bot_framework: BotFrameworkClient,
    config: TeamsConfig,
}
```

**Features**:
- Bot Framework authentication
- Teams app manifest
- Mention handling
- Group/channel routing

---

### 1.3 Google Chat Integration
**Crate**: `crates/channels/src/google_chat.rs`

**Implementation**:
```rust
// Google Chat API
pub struct GoogleChatChannel {
    client: ChatServiceClient,
    webhook_handler: WebhookHandler,
}
```

---

## Phase 2: Voice & Canvas (4 weeks)

### 2.1 Voice Wake System
**Crate**: `crates/voice/src/wake.rs`

**Implementation**:
```rust
// Porcupine or Whisper-based wake word
pub struct VoiceWake {
    detector: WakeWordDetector,
    config: WakeConfig,
}

impl VoiceWake {
    pub async fn listen(&self) -> Stream<WakeEvent>;
    pub async fn handle_wake(&self, context: AudioContext);
}
```

**Dependencies**:
- `pv_porcupine` (wake word)
- `whisper-rs` (transcription)
- `cpal` (audio I/O)

---

### 2.2 Talk Mode
**Crate**: `crates/voice/src/talk.rs`

**Implementation**:
```rust
pub struct TalkMode {
    stt: SpeechToText,
    tts: TextToSpeech,
    agent: AgentHandle,
}

impl TalkMode {
    pub async fn run(&mut self) -> Result<()> {
        // Continuous voice loop
        loop {
            let audio = self.capture_audio().await?;
            let text = self.stt.transcribe(audio).await?;
            let response = self.agent.process(text).await?;
            self.tts.speak(response).await?;
        }
    }
}
```

---

### 2.3 Live Canvas / A2UI
**Crate**: `crates/canvas/src/lib.rs`

**Implementation**:
```rust
// Agent-to-User Interface
pub struct Canvas {
    state: CanvasState,
    renderer: CanvasRenderer,
    websocket: WebSocket,
}

pub struct CanvasState {
    pub elements: Vec<CanvasElement>,
    pub layout: Layout,
    pub interactions: Vec<Interaction>,
}

pub enum CanvasElement {
    Text { content: String, style: TextStyle },
    Image { url: String, alt: String },
    Chart { data: ChartData, type: ChartType },
    Form { fields: Vec<FormField> },
    Button { label: String, action: Action },
}
```

**WebSocket Protocol**:
```typescript
// A2UI Protocol
interface CanvasMessage {
    type: 'push' | 'reset' | 'eval' | 'snapshot';
    id: string;
    content: CanvasElement[];
}
```

---

## Phase 3: Agent System (4 weeks)

### 3.1 Multi-Agent Routing
**Crate**: `crates/agent/src/routing.rs`

**Implementation**:
```rust
pub struct AgentRouter {
    agents: HashMap<AgentId, AgentHandle>,
    routing_rules: Vec<RoutingRule>,
    default_agent: AgentId,
}

pub struct RoutingRule {
    pub matcher: Box<dyn Fn(&Message) -> bool + Send + Sync>,
    pub target_agent: AgentId,
    pub priority: u32,
}

impl AgentRouter {
    pub async fn route(&self, message: Message) -> Result<AgentId> {
        // Check rules in priority order
        for rule in &self.routing_rules {
            if (rule.matcher)(&message) {
                return Ok(rule.target_agent.clone());
            }
        }
        Ok(self.default_agent.clone())
    }
}
```

---

### 3.2 Agent-to-Agent Communication
**Crate**: `crates/agent/src/inter_agent.rs`

**Tools to implement**:
```rust
// sessions_list - List active sessions/agents
#[derive(Tool)]
#[tool(name = "sessions_list", description = "List all active agent sessions")]
struct SessionsListTool;

#[derive(Tool)]
#[tool(name = "sessions_history", description = "Get message history for a session")]
struct SessionsHistoryTool {
    session_id: String,
    limit: Option<usize>,
}

#[derive(Tool)]
#[tool(name = "sessions_send", description = "Send a message to another session")]
struct SessionsSendTool {
    target_session_id: String,
    message: String,
    expect_reply: bool,
    timeout_secs: Option<u64>,
}

#[derive(Tool)]
#[tool(name = "sessions_spawn", description = "Spawn a new agent session")]
struct SessionsSpawnTool {
    workspace: String,
    initial_prompt: String,
    model: Option<String>,
}
```

---

### 3.3 Heartbeat Scheduler
**Crate**: `crates/scheduler/src/heartbeat.rs`

**Implementation**:
```rust
pub struct HeartbeatScheduler {
    tasks: HashMap<TaskId, HeartbeatTask>,
    trigger: broadcast::Sender<HeartbeatEvent>,
}

pub struct HeartbeatTask {
    pub id: TaskId,
    pub condition: HeartbeatCondition,
    pub action: HeartbeatAction,
    pub last_check: DateTime<Utc>,
}

pub enum HeartbeatCondition {
    Time { cron: String },
    FileChanged { path: PathBuf },
    EmailReceived { query: String },
    IdleDuration { duration: Duration },
    Custom { check: Box<dyn Fn() -> bool + Send> },
}

impl HeartbeatScheduler {
    pub async fn run(&self) {
        loop {
            for (id, task) in &self.tasks {
                if self.check_condition(&task.condition).await {
                    self.execute_action(&task.action).await;
                }
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}
```

---

## Phase 4: Developer Experience (4 weeks)

### 4.1 Interactive Onboarding
**Crate**: `crates/cli/src/onboard.rs`

**Implementation**:
```rust
pub struct OnboardingWizard {
    steps: Vec<Box<dyn OnboardingStep>>,
    state: OnboardingState,
}

#[async_trait]
pub trait OnboardingStep: Send + Sync {
    fn name(&self) -> &str;
    async fn run(&self, state: &mut OnboardingState) -> Result<bool>;
}

pub struct SetupGatewayStep;
pub struct SetupChannelsStep;
pub struct SetupModelStep;
pub struct InstallDaemonStep;

impl OnboardingWizard {
    pub async fn run(&mut self) -> Result<()> {
        println!("🦀 Welcome to OpenRustClaw Onboarding!");
        
        for step in &self.steps {
            let success = step.run(&mut self.state).await?;
            if !success {
                println!("⚠️  Step '{}' failed, but continuing...", step.name());
            }
        }
        
        println!("✅ Onboarding complete!");
    }
}
```

---

### 4.2 Chat Commands
**Crate**: `crates/channels/src/commands.rs`

**Implementation**:
```rust
pub struct CommandParser {
    commands: HashMap<String, Box<dyn ChatCommand>>,
}

#[async_trait]
pub trait ChatCommand: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, args: &[String], ctx: CommandContext) -> CommandResult;
}

// Commands to implement:
pub struct StatusCommand;      // /status
pub struct ResetCommand;       // /new, /reset
pub struct CompactCommand;     // /compact
pub struct ThinkCommand;       // /think <level>
pub struct VerboseCommand;     // /verbose on|off
pub struct UsageCommand;       // /usage off|tokens|full
pub struct RestartCommand;     // /restart (owner only)
pub struct ActivationCommand;  // /activation mention|always
```

---

### 4.3 ClawHub Skills Registry
**Crate**: `crates/skills/src/registry.rs`

**Implementation**:
```rust
pub struct ClawHubRegistry {
    client: reqwest::Client,
    endpoint: String,
    cache: SkillCache,
}

pub struct SkillMetadata {
    pub name: String,
    pub version: Version,
    pub author: String,
    pub description: String,
    pub repository: String,
    pub signature: Option<Ed25519Signature>,
    pub downloads: u64,
    pub rating: f32,
}

impl ClawHubRegistry {
    pub async fn search(&self, query: &str) -> Result<Vec<SkillMetadata>>;
    pub async fn install(&self, name: &str, version: Option<Version>) -> Result<()>;
    pub async fn verify(&self, metadata: &SkillMetadata) -> Result<bool>;
}
```

---

## Phase 5: Device Integration (4 weeks)

### 5.1 iOS Node
**Repository**: `openrustclaw/ios-node`

**Features**:
- Swift-based iOS app
- WebSocket connection to Gateway
- Voice Wake (on-device)
- Camera access
- Screen recording
- Location services
- Push notifications

**Protocol**:
```swift
// Node capability advertisement
struct NodeCapabilities {
    let canWake: Bool
    let canCamera: Bool
    let canScreenRecord: Bool
    let canLocation: Bool
    let canNotify: Bool
}
```

---

### 5.2 Android Node
**Repository**: `openrustclaw/android-node`

**Features**:
- Kotlin-based Android app
- Connect tab (pairing)
- Chat tab
- Voice tab
- Canvas surface
- Camera/screen capture
- Device commands (SMS, contacts, calendar)

---

### 5.3 Webhooks
**Crate**: `crates/gateway/src/webhooks.rs`

**Implementation**:
```rust
pub struct WebhookManager {
    handlers: HashMap<String, WebhookHandler>,
    config: WebhookConfig,
}

pub struct WebhookHandler {
    pub path: String,
    pub secret: String,
    pub action: WebhookAction,
}

impl WebhookManager {
    pub async fn handle_request(&self, path: &str, payload: Bytes, signature: &str) -> Result<Response> {
        // Verify signature
        // Route to handler
        // Execute action
    }
}
```

---

## Technical Architecture

### New Crates Structure

```
crates/
├── channels/           # Existing + WhatsApp, Teams, Google Chat
├── voice/             # NEW: Voice wake, TTS, STT
├── canvas/            # NEW: A2UI visual workspace
├── agent/             # Extended: Multi-agent, inter-agent
├── scheduler/         # Extended: Heartbeat
├── onboarding/        # NEW: Interactive setup
├── skills/            # Extended: ClawHub registry
└── nodes/             # NEW: Device node protocol
```

### Mobile Apps

```
apps/
├── ios/               # Swift iOS node
├── android/           # Kotlin Android node
└── desktop/           # Rust Tauri desktop app
```

---

## Dependencies to Add

```toml
# Voice
pv_porcupine = "3.0"
whisper-rs = "0.8"
cpal = "0.15"
rodio = "0.17"

# Canvas/WebRTC
tokio-tungstenite = "0.26"
webrtc = "0.11"

# WhatsApp
whatsapp-rs = "0.1"  # or build from Baileys

# iOS/Android communication
bonjour = "0.2"      # mDNS discovery
```

---

## Testing Strategy

1. **Unit Tests**: Each crate has 80%+ coverage
2. **Integration Tests**: E2E channel tests
3. **Device Tests**: Physical iOS/Android devices
4. **Load Tests**: WebSocket connection limits
5. **Security Tests**: Penetration testing for voice/canvas

---

## Timeline Summary

| Phase | Duration | Key Deliverables |
|-------|----------|------------------|
| 1: Channels | 4 weeks | WhatsApp, Teams, Google Chat |
| 2: Voice & Canvas | 4 weeks | Voice Wake, Talk Mode, A2UI |
| 3: Agent System | 4 weeks | Multi-agent, sessions_*, heartbeat |
| 4: DX | 4 weeks | Onboarding, commands, ClawHub |
| 5: Devices | 4 weeks | iOS, Android, webhooks |
| **Total** | **20 weeks** | Feature parity achieved |

---

## Success Metrics

- [ ] 20+ channel integrations (matching OpenClaw)
- [ ] Voice wake accuracy > 95%
- [ ] Canvas latency < 100ms
- [ ] Multi-agent routing < 10ms
- [ ] Heartbeat scheduler 99.9% reliability
- [ ] Onboarding completion rate > 80%
- [ ] ClawHub: 100+ skills available

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| WhatsApp API changes | High | Use stable Baileys, monitor updates |
| Voice accuracy | Medium | Fallback to manual activation |
| iOS App Store rejection | High | TestFlight first, comply with guidelines |
| Performance on low-end devices | Medium | Feature flags, graceful degradation |

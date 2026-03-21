//! OpenRustClaw CLI entry point.

use anyhow::Result;
use clap::{Parser, Subcommand};
use openrustclaw_mobile::DeviceCommandKind;
use serde_json::Value;

mod commands;

#[derive(Parser)]
#[command(name = "openrustclaw")]
#[command(about = "OpenRustClaw - AI Agent Framework", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the gateway server and optional compatibility/experimental sidecar
    Start {
        /// Config file path
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        /// Channels to enable (comma-separated, e.g. webchat,telegram,discord,slack,teams)
        #[arg(short = 'C', long, value_name = "CHANNELS")]
        channels: Option<String>,
    },
    /// Interactive chat with the agent
    Chat {
        /// Provider to use (anthropic, openai, openrouter, ollama)
        #[arg(short, long, default_value = "anthropic")]
        provider: String,
        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
    },
    /// Bounded browser automation and page extraction
    Browser {
        #[command(subcommand)]
        action: BrowserAction,
    },
    /// Manage LLM models
    Models {
        #[command(subcommand)]
        action: ModelsAction,
    },
    /// Manage skills
    Skills {
        #[command(subcommand)]
        action: SkillsAction,
    },
    /// Manage scheduled jobs
    Schedule {
        #[command(subcommand)]
        action: ScheduleAction,
    },
    /// Google Meet operator integration
    Meet {
        #[command(subcommand)]
        action: MeetAction,
    },
    /// Matrix operator integration
    Matrix {
        #[command(subcommand)]
        action: MatrixAction,
    },
    /// Gmail Pub/Sub operator integration
    Gmail {
        #[command(subcommand)]
        action: GmailAction,
    },
    /// Google Chat operator integration
    GoogleChat {
        #[command(subcommand)]
        action: GoogleChatAction,
    },
    /// iMessage / BlueBubbles operator integration
    IMessage {
        #[command(subcommand)]
        action: IMessageAction,
    },
    /// Signal operator integration
    Signal {
        #[command(subcommand)]
        action: SignalAction,
    },
    /// WhatsApp operator integration
    WhatsApp {
        #[command(subcommand)]
        action: WhatsAppAction,
    },
    /// Mobile node registry and operator preview surfaces
    Mobile {
        #[command(subcommand)]
        action: MobileAction,
    },
    /// Bounded local media inspection and text extraction
    Media {
        #[command(subcommand)]
        action: MediaAction,
    },
    /// Manage file-backed control-plane profiles and multi-claw runtime mode
    Control {
        #[command(subcommand)]
        action: ControlAction,
    },
    /// Manage runtime config, vault, and hot-reload/model switching
    Runtime {
        #[command(subcommand)]
        action: RuntimeAction,
    },
    /// Execute bounded routed or orchestrated multi-model runs
    Orchestrate {
        #[command(subcommand)]
        action: OrchestrateAction,
    },
    /// Manage autonomous optimization targets and candidates
    Optimize {
        #[command(subcommand)]
        action: OptimizeAction,
    },
    /// Security audit and management
    Security {
        #[command(subcommand)]
        action: SecurityAction,
    },
    /// Memory management
    Memory {
        #[command(subcommand)]
        action: MemoryAction,
    },
    /// Session inspection and control
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
    /// Channel account, binding, and pairing controls
    Channels {
        #[command(subcommand)]
        action: ChannelAction,
    },
    /// Run diagnostics
    Doctor {
        /// Attempt safe repairs for missing registries/directories
        #[arg(long)]
        repair: bool,
        /// Include deeper optional diagnostics
        #[arg(long)]
        deep: bool,
        /// Avoid interactive remediation hints
        #[arg(long)]
        non_interactive: bool,
    },
    /// Interactive onboarding wizard
    Onboard,
    /// Set up Cursor IDE integration
    #[cfg(feature = "cursor")]
    Cursor {
        #[command(subcommand)]
        action: CursorAction,
    },
    /// Start MCP server for external clients
    McpServer {
        /// Transport type (stdio or sse)
        #[arg(short, long, default_value = "stdio")]
        transport: String,
        /// Config file path for database-backed MCP tools
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// mcp2cli - Token-efficient MCP tool discovery (96-99% savings)
    Mcp2Cli {
        #[command(subcommand)]
        action: Mcp2CliAction,
    },
    /// Talk Mode - continuous voice conversation
    #[cfg(feature = "voice")]
    Talk {
        /// Provider to use (anthropic, openai, openrouter, ollama)
        #[arg(short, long, default_value = "anthropic")]
        provider: String,
        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
        /// Wake word to activate listening
        #[arg(short, long, default_value = "Hey Assistant")]
        wake_word: String,
        /// Silence timeout in seconds (stop listening after silence)
        #[arg(long, default_value = "3")]
        silence_timeout: u64,
        /// Maximum utterance duration in seconds
        #[arg(long, default_value = "30")]
        max_utterance: u64,
        /// Enable barge-in (interrupt TTS with wake word)
        #[arg(long, default_value = "true")]
        barge_in: bool,
    },
    /// Voice runtime inspection and transcription tools
    Voice {
        #[command(subcommand)]
        action: VoiceAction,
    },
    /// Manage webhooks for external integrations
    Webhooks {
        #[command(subcommand)]
        action: WebhooksAction,
    },
}

#[derive(Subcommand)]
enum ModelsAction {
    /// List available models
    List,
    /// Show model details
    Info { name: String },
    /// Scan configured providers and recommend model-role assignments
    Scan,
}

#[derive(Subcommand)]
enum BrowserAction {
    /// Open a durable browser session descriptor for native or agent-browser execution
    OpenSession {
        #[arg(long, default_value = "native_cdp")]
        backend: String,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long)]
        label: Option<String>,
        #[arg(long)]
        timeout_ms: Option<u64>,
    },
    /// List known browser sessions
    Sessions {
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Inspect a page as a bounded DOM/accessibility snapshot or structured surface
    Inspect {
        url: String,
        #[arg(long, default_value = "native_cdp")]
        backend: String,
        #[arg(long, default_value = "snapshot")]
        kind: String,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long)]
        selector: Option<String>,
        #[arg(long, default_value_t = false)]
        interactive_only: bool,
        #[arg(long)]
        snapshot_depth: Option<usize>,
        #[arg(long)]
        wait_until: Option<String>,
        #[arg(long)]
        timeout_ms: Option<u64>,
        #[arg(long)]
        path: Option<String>,
    },
    /// Run a bounded browser action sequence from a JSON spec file
    RunSequence {
        spec: String,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long)]
        timeout_ms: Option<u64>,
        #[arg(long)]
        path: Option<String>,
    },
    /// List browser artifacts already written under `.claw/browser/`
    Artifacts {
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Fetch a page over HTTP and save a normalized read artifact
    ReadPage {
        url: String,
        #[arg(long)]
        max_chars: Option<usize>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        timeout_ms: Option<u64>,
    },
    /// Crawl a small same-domain site map and save a crawl artifact
    CrawlSite {
        url: String,
        #[arg(long)]
        max_pages: Option<usize>,
        #[arg(long)]
        max_chars_per_page: Option<usize>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        timeout_ms: Option<u64>,
    },
    /// Navigate to a page and return basic metadata
    Navigate {
        url: String,
        #[arg(long, default_value = "native_cdp")]
        backend: String,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long)]
        wait_until: Option<String>,
        #[arg(long)]
        timeout_ms: Option<u64>,
    },
    /// Extract text, HTML, links, images, or headings from a page
    Extract {
        url: String,
        #[arg(long, default_value = "native_cdp")]
        backend: String,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long, default_value = "text")]
        what: String,
        #[arg(long)]
        selector: Option<String>,
        #[arg(long)]
        wait_until: Option<String>,
        #[arg(long)]
        timeout_ms: Option<u64>,
        #[arg(long)]
        max_results: Option<usize>,
        #[arg(long)]
        max_chars: Option<usize>,
    },
    /// Capture a screenshot to `.claw/browser/screenshots/` or an explicit path
    Screenshot {
        url: String,
        #[arg(long, default_value = "native_cdp")]
        backend: String,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        selector: Option<String>,
        #[arg(long, default_value_t = false)]
        full_page: bool,
        #[arg(long)]
        format: Option<String>,
        #[arg(long)]
        quality: Option<u8>,
        #[arg(long)]
        wait_until: Option<String>,
        #[arg(long)]
        timeout_ms: Option<u64>,
    },
    /// Render a page to PDF in `.claw/browser/pdf/` or an explicit path
    Pdf {
        url: String,
        #[arg(long, default_value = "native_cdp")]
        backend: String,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        format: Option<String>,
        #[arg(long)]
        print_background: Option<bool>,
        #[arg(long)]
        wait_until: Option<String>,
        #[arg(long)]
        timeout_ms: Option<u64>,
    },
}

#[derive(Subcommand)]
enum RuntimeAction {
    /// Inspect the effective runtime configuration status
    Status {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Inspect or refresh persisted runtime health and fallback validation state
    Health {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        refresh: bool,
    },
    /// Inspect how the current effective config would reload and which changes still require restart
    Reload {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Inspect the last runtime-applied snapshot and current live-reload plan
    ReloadPlan {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Switch the default provider and optionally model/account key reference
    SwitchProvider {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        provider: String,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        api_key_env: Option<String>,
        #[arg(long = "fallback")]
        fallback_chain: Vec<String>,
    },
    /// Switch the configured model for a provider
    SwitchModel {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        provider: String,
        #[arg(long)]
        model: String,
    },
    /// Manage the encrypted runtime secret vault
    Vault {
        #[command(subcommand)]
        action: RuntimeVaultAction,
    },
    /// Inspect deeper service and scheduler diagnostics
    Services {
        #[command(subcommand)]
        action: RuntimeServicesAction,
    },
}

#[derive(Subcommand)]
enum RuntimeVaultAction {
    /// Show vault status and configured secret keys
    Status,
    /// List stored secret keys without revealing values
    List,
    /// Set or replace a secret value
    Set {
        key: String,
        #[arg(long)]
        value: String,
    },
    /// Delete a secret value
    Delete { key: String },
}

#[derive(Subcommand)]
enum RuntimeServicesAction {
    /// Show service-level runtime diagnostics
    Status {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Show scheduler/job/runtime-event health
    Scheduler {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// List recent runtime events
    Events {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
    /// Show readiness probes for enabled channels
    Channels {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        refresh: bool,
    },
    /// Show recent runtime logs captured by the running gateway process
    Logs {
        #[arg(short, long, default_value_t = 50)]
        limit: usize,
    },
    /// Show the current runtime liveness beacon
    Beacon {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        refresh: bool,
    },
}

#[derive(Subcommand)]
enum OrchestrateAction {
    /// Resolve which claw and model profile would handle a task
    Resolve {
        #[arg(long)]
        task_id: Option<String>,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        claw: Option<String>,
        #[arg(long)]
        model_profile: Option<String>,
        #[arg(long)]
        worker_model_profile: Option<String>,
        #[arg(long)]
        autonomy_level: Option<String>,
        #[arg(long)]
        max_delegations: Option<usize>,
        #[arg(long)]
        max_iterations: Option<usize>,
        #[arg(long)]
        max_runtime_secs: Option<u64>,
        #[arg(long)]
        approval_policy: Option<String>,
        #[arg(long, default_value = "auto")]
        mode: String,
    },
    /// List saved orchestration receipts
    List {
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
    /// Inspect one saved orchestration receipt with checkpoints and supervision
    Inspect {
        receipt_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Inspect structured trace entries and parent/child relationships for one receipt
    Trace { receipt_id: String },
    /// Inspect the full persisted parent/child orchestration transcript for one receipt
    Transcript { receipt_id: String },
    /// Inspect estimated token and duration summaries for one receipt
    Resources { receipt_id: String },
    /// Submit a background orchestrated run with active supervision state
    Submit {
        prompt: String,
        #[arg(long)]
        task_id: Option<String>,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        claw: Option<String>,
        #[arg(long)]
        model_profile: Option<String>,
        #[arg(long)]
        worker_model_profile: Option<String>,
        #[arg(long)]
        autonomy_level: Option<String>,
        #[arg(long)]
        max_delegations: Option<usize>,
        #[arg(long)]
        max_iterations: Option<usize>,
        #[arg(long)]
        max_runtime_secs: Option<u64>,
        #[arg(long)]
        approval_policy: Option<String>,
        #[arg(long, default_value = "orchestrated")]
        mode: String,
    },
    /// List active orchestration runs and their live supervision status
    Active {
        #[arg(long, default_value_t = true)]
        active_only: bool,
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
    /// Inspect the latest active-run state and recent events
    Watch {
        run_id: String,
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
    /// Request a pause for an active orchestration run
    Pause { run_id: String },
    /// Resume a paused orchestration run
    Resume { run_id: String },
    /// Kill an active orchestration run
    Kill { run_id: String },
    /// Promote one reflection candidate from a receipt into a decision lesson
    PromoteCandidate {
        receipt_id: String,
        index: usize,
        #[arg(long)]
        lesson_id: Option<String>,
        #[arg(long, default_value_t = true)]
        active: bool,
        #[arg(long)]
        signal: Option<String>,
        #[arg(long)]
        recommendation: Option<String>,
        #[arg(long)]
        rationale: Option<String>,
        #[arg(long)]
        confidence: Option<f32>,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        claw_id: Option<String>,
        #[arg(long)]
        model_profile: Option<String>,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        autonomy_level: Option<String>,
        #[arg(long)]
        execution_mode: Option<String>,
    },
    /// Run a bounded direct or orchestrated multi-model execution
    Run {
        prompt: String,
        #[arg(long)]
        task_id: Option<String>,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        claw: Option<String>,
        #[arg(long)]
        model_profile: Option<String>,
        #[arg(long)]
        worker_model_profile: Option<String>,
        #[arg(long)]
        autonomy_level: Option<String>,
        #[arg(long)]
        max_delegations: Option<usize>,
        #[arg(long)]
        max_iterations: Option<usize>,
        #[arg(long)]
        max_runtime_secs: Option<u64>,
        #[arg(long)]
        approval_policy: Option<String>,
        #[arg(long, default_value = "auto")]
        mode: String,
        #[arg(long)]
        json: bool,
    },
    #[command(hide = true)]
    WorkerRun {
        #[arg(long)]
        run_id: String,
    },
}

#[derive(Subcommand)]
enum SkillsAction {
    /// List installed skills
    List,
    /// List compiled Rust-native extension manifests derived from skills
    ListExtensions,
    /// Compile cached MCP/CLI/help artifacts for one skill or all discoverable skills
    Compile {
        /// Optional skill name. Omit to compile all discoverable skills.
        name: Option<String>,
    },
    /// Refresh compiled skill artifacts for all discoverable skills
    Refresh,
    /// Inspect one compiled skill artifact bundle
    InspectCompiled { name: String },
    /// Inspect one compiled Rust-native extension manifest
    InspectExtension { name: String },
    /// List declared and inferred background services for one compiled skill
    BackgroundServices { name: String },
    /// List channel bindings that currently attach compiled skill background extensions
    ListChannelExtensions,
    /// List configured auth-plugin bindings over compiled skills
    ListAuthPlugins,
    /// List configured voice-call plugin bindings over compiled skills
    ListVoicePlugins,
    /// Bind a compiled skill background service to a file-backed channel binding
    BindChannelExtension {
        binding_id: String,
        skill_name: String,
        #[arg(long)]
        service: Option<String>,
        #[arg(long)]
        component: Option<String>,
        #[arg(long)]
        trigger: Option<String>,
    },
    /// Bind a compiled skill to a bounded OIDC auth-provider lane
    BindAuthPlugin {
        provider_id: String,
        skill_name: String,
        #[arg(long)]
        redirect_uri: Option<String>,
        #[arg(long)]
        issuer: Option<String>,
        #[arg(long)]
        authorization_endpoint: Option<String>,
        #[arg(long)]
        token_endpoint: Option<String>,
        #[arg(long)]
        client_id_key: Option<String>,
        #[arg(long)]
        client_secret_key: Option<String>,
        #[arg(long)]
        scopes: Option<String>,
        #[arg(long)]
        vault_key_prefix: Option<String>,
        #[arg(long)]
        service: Option<String>,
        #[arg(long)]
        component: Option<String>,
    },
    /// Bind a compiled skill to a bounded voice-call plugin lane
    BindVoicePlugin {
        plugin_id: String,
        skill_name: String,
        #[arg(long)]
        service: Option<String>,
        #[arg(long)]
        component: Option<String>,
        #[arg(long)]
        greeting_text: Option<String>,
        #[arg(long)]
        default_voice: Option<String>,
    },
    /// Start an authorization-code flow for a configured auth plugin
    AuthAuthorize {
        provider_id: String,
        #[arg(long)]
        redirect_uri: Option<String>,
    },
    /// Exchange an authorization code for tokens and store them in the runtime vault
    AuthExchange {
        provider_id: String,
        #[arg(long)]
        code: String,
        #[arg(long)]
        state: String,
        #[arg(long)]
        redirect_uri: Option<String>,
    },
    /// List persisted bounded voice-call session receipts
    ListVoiceCalls,
    /// Summarize bounded voice-call lifecycle health
    VoiceCallHealth,
    /// Start a bounded voice-call session for a configured plugin
    StartVoiceCall {
        plugin_id: String,
        #[arg(long)]
        remote: Option<String>,
        #[arg(long)]
        greeting_text: Option<String>,
        #[arg(long)]
        voice: Option<String>,
        #[arg(long)]
        metadata: Option<String>,
        #[arg(long)]
        stale_after_secs: Option<u64>,
    },
    /// End a bounded voice-call session
    EndVoiceCall {
        call_id: String,
        #[arg(long)]
        reason: Option<String>,
        #[arg(long)]
        metadata: Option<String>,
    },
    /// Reconnect a bounded voice-call session and refresh its receipt
    ReconnectVoiceCall {
        call_id: String,
        #[arg(long)]
        remote: Option<String>,
        #[arg(long)]
        greeting_text: Option<String>,
        #[arg(long)]
        voice: Option<String>,
        #[arg(long)]
        metadata: Option<String>,
        #[arg(long)]
        stale_after_secs: Option<u64>,
    },
    /// Prewarm greeting audio for a configured voice-call plugin
    PrewarmVoicePlugin {
        plugin_id: String,
        #[arg(long)]
        greeting_text: Option<String>,
        #[arg(long)]
        voice: Option<String>,
    },
    /// Reap stale bounded voice-call sessions
    ReapVoiceCalls {
        #[arg(long)]
        stale_after_secs: Option<u64>,
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Execute a bounded `.wasm` or `.wat` component from a compiled skill
    Execute {
        name: String,
        #[arg(long)]
        component: Option<String>,
        #[arg(long)]
        input: Option<String>,
    },
    /// Schedule a compiled skill background workflow through the durable scheduler
    ScheduleBackground {
        name: String,
        #[arg(long)]
        service: Option<String>,
        #[arg(long)]
        component: Option<String>,
        #[arg(long)]
        input: Option<String>,
        #[arg(long)]
        every_seconds: Option<u64>,
        #[arg(long)]
        at: Option<String>,
        #[arg(long, default_value_t = 100)]
        priority: i64,
    },
    /// Invoke the generated CLI/help bridge for one compiled skill
    Invoke {
        name: String,
        #[arg(long)]
        args: Option<String>,
        #[arg(long)]
        reference: Option<String>,
        #[arg(long)]
        detail: bool,
    },
    /// Search for skills in the registry
    Search {
        query: String,
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,
        /// Sort by: relevance, downloads, rating, recent
        #[arg(short, long, default_value = "relevance")]
        sort: String,
    },
    /// Install a skill from the workspace or ClawHub registry
    Install { name: String },
    /// Update an installed skill
    Update { name: String },
    /// Uninstall a skill
    Uninstall { name: String },
    /// Verify skill signatures
    Verify { name: String },
    /// Show popular skills
    Popular {
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Show trending skills
    Trending {
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum ScheduleAction {
    /// List scheduled jobs
    List,
    /// Create a new job
    Create {
        #[arg(short, long, required_unless_present = "file")]
        name: Option<String>,
        #[arg(short, long, required_unless_present = "file")]
        workflow: Option<String>,
        #[arg(short, long)]
        description: Option<String>,
        /// Import a file-backed task manifest instead of providing inline fields
        #[arg(long)]
        file: Option<String>,
        /// Run every N seconds
        #[arg(long)]
        every_seconds: Option<u64>,
        /// Run once at an RFC3339 timestamp
        #[arg(long)]
        at: Option<String>,
        /// JSON workflow payload passed to the sidecar
        #[arg(long)]
        payload: Option<String>,
        /// Lower numbers run first
        #[arg(long, default_value_t = 100)]
        priority: i64,
        #[arg(long)]
        owner: Option<String>,
        #[arg(long = "tag")]
        tags: Vec<String>,
    },
    /// Sync `.claw/tasks/`-style manifests into the durable scheduler
    Sync {
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Initialize the standard task-manifest folder with starter templates
    Init {
        #[arg(long)]
        path: Option<String>,
    },
    /// Export a scheduled job as a task manifest
    Export {
        id: String,
        #[arg(long)]
        output: Option<String>,
    },
    /// Inspect one task/job with manifest and status details
    Inspect { id: String },
    /// Pause a job
    Pause { id: String },
    /// Resume a job
    Resume { id: String },
    /// Force a task to become due immediately
    RunNow { id: String },
    /// Change task priority (lower numbers run first)
    Reprioritize { id: String, priority: i64 },
    /// Rebind a task to a different workflow target
    Rebind { id: String, workflow: String },
    /// Disable a task until a future RFC3339 timestamp
    DisableUntil { id: String, until: String },
    /// Clear task disabled-until state
    Enable { id: String },
    /// List recent job attempts
    Runs {
        #[arg(long)]
        job: Option<String>,
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
    /// List dead-letter entries
    DeadLetters {
        #[arg(long)]
        job: Option<String>,
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
    /// Replay a dead-letter entry by id
    ReplayDeadLetter { id: String },
    /// List recent runtime events
    Events {
        #[arg(long)]
        name: Option<String>,
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum MeetAction {
    /// Create a new Google Meet space
    CreateSpace {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Fetch one Meet space by resource name
    GetSpace {
        name: String,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// End the active conference for a Meet space
    EndSpace {
        name: String,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// List conference records
    ListRecords {
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// List participants for a conference record
    ListParticipants {
        conference_record: String,
        #[arg(short, long, default_value_t = 100)]
        limit: usize,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// List recordings for a conference record
    ListRecordings {
        conference_record: String,
        #[arg(short, long, default_value_t = 100)]
        limit: usize,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// List transcripts for a conference record
    ListTranscripts {
        conference_record: String,
        #[arg(short, long, default_value_t = 100)]
        limit: usize,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Show transcript entries for a transcript resource
    ShowTranscript {
        transcript: String,
        #[arg(short, long, default_value_t = 500)]
        limit: usize,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Decode a Google Workspace Events / Pub/Sub push payload
    DecodeEvent {
        /// File path containing the raw JSON envelope, or '-' for stdin
        input: String,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
}

#[derive(Subcommand)]
enum MatrixAction {
    /// Join a Matrix room by room id or alias
    Join {
        room: String,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Leave a Matrix room
    Leave {
        room: String,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// List joined Matrix rooms
    Rooms {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Send formatted HTML/plaintext content to a Matrix room
    SendFormatted {
        room: String,
        text: String,
        html: String,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// React to a Matrix event
    React {
        room: String,
        event_id: String,
        emoji: String,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Upload and send a file to a Matrix room
    SendFile {
        room: String,
        file_path: String,
        #[arg(long)]
        filename: Option<String>,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Toggle typing indicator in a Matrix room
    Typing {
        room: String,
        #[arg(long, default_value_t = true)]
        typing: bool,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Redact a Matrix event
    Redact {
        room: String,
        event_id: String,
        #[arg(long)]
        reason: Option<String>,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
}

#[derive(Subcommand)]
enum GmailAction {
    /// Show Gmail Pub/Sub config/operator state
    Status {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Validate Gmail auth/watch setup by connecting once
    Connect {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Replay a Gmail notification payload from a file or stdin (`-`)
    ProcessNotification {
        input: String,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Send a new Gmail message
    Send {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long = "to")]
        to: Vec<String>,
        #[arg(long = "cc")]
        cc: Vec<String>,
        #[arg(long = "bcc")]
        bcc: Vec<String>,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        body: String,
        #[arg(long)]
        thread_id: Option<String>,
        #[arg(long = "file")]
        files: Vec<String>,
    },
    /// Reply to a Gmail message
    Reply {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        message_id: String,
        #[arg(long)]
        body: String,
        #[arg(long = "file")]
        files: Vec<String>,
    },
    /// Modify labels on a Gmail message
    Label {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        message_id: String,
        #[arg(long = "add")]
        add: Vec<String>,
        #[arg(long = "remove")]
        remove: Vec<String>,
    },
    /// Archive a Gmail message
    Archive {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        message_id: String,
    },
    /// Delete a Gmail message
    Delete {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        message_id: String,
    },
    /// Forward a Gmail message
    Forward {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        message_id: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        body: String,
        #[arg(long = "file")]
        files: Vec<String>,
    },
}

#[derive(Subcommand)]
enum GoogleChatAction {
    /// Show Google Chat config/operator state
    Status {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Validate Google Chat auth/config by connecting once
    Connect {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Send a text message to a Google Chat space
    Send {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        space: String,
        #[arg(long)]
        message: String,
        #[arg(long)]
        thread: Option<String>,
    },
    /// Send a Google Chat card message
    SendCard {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        space: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        content: String,
        #[arg(long)]
        thread: Option<String>,
        #[arg(long)]
        subtitle: Option<String>,
        #[arg(long)]
        image_url: Option<String>,
        #[arg(long)]
        section_header: Option<String>,
    },
    /// Send a file-reference/open-link card
    SendLinkCard {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        space: String,
        #[arg(long)]
        url: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        thread: Option<String>,
    },
}

#[derive(Subcommand)]
enum IMessageAction {
    /// Check iMessage / BlueBubbles health
    Ping {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Show iMessage / BlueBubbles server info
    Server {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// List recent BlueBubbles chats
    Chats {
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value_t = 0)]
        offset: usize,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// List BlueBubbles contacts
    Contacts {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Send an iMessage to a handle or chat target
    Send {
        to: String,
        message: String,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Send a local file attachment over iMessage
    SendFile {
        to: String,
        file_path: String,
        #[arg(long)]
        filename: Option<String>,
        #[arg(long)]
        message: Option<String>,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Send a tapback reaction
    Tapback {
        chat_guid: String,
        message_guid: String,
        reaction: String,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
}

#[derive(Subcommand)]
enum SignalAction {
    /// Register the configured Signal number
    Register {
        #[arg(long)]
        voice: bool,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Verify the configured Signal number with a received code
    Verify {
        code: String,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Link a secondary Signal device and print the linking URI
    Link {
        #[arg(long, default_value = "OpenRustClaw")]
        device_name: String,
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// List known Signal groups for the configured account
    ListGroups {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
}

#[derive(Subcommand)]
enum WhatsAppAction {
    /// Show current WhatsApp bridge/operator state from config
    Status {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Start a WhatsApp connection and print the resulting state
    Connect {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Wait for a QR code or pairing code and print it
    Pair {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long, default_value_t = 30)]
        timeout_secs: u64,
    },
    /// Send a text message
    Send {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        message: String,
    },
    /// Send media from a local path
    SendMedia {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        media_type: String,
        #[arg(long)]
        path: String,
        #[arg(long)]
        caption: Option<String>,
    },
}

#[derive(Subcommand)]
enum MobileAction {
    /// List configured mobile nodes
    List,
    /// Pair or register a mobile node manifest
    Pair {
        #[arg(long)]
        id: String,
        #[arg(long)]
        gateway_url: String,
        #[arg(long)]
        auth_token_env: String,
        #[arg(long)]
        device_name: Option<String>,
        #[arg(long)]
        platform: Option<String>,
        #[arg(long = "capability")]
        capabilities: Vec<String>,
        #[arg(long, default_value_t = true)]
        enabled: bool,
    },
    /// List persisted bounded mobile pairing lifecycle receipts
    Pairings {
        #[arg(long)]
        node_id: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Remove a bounded mobile node manifest and record an unpair lifecycle receipt
    Unpair {
        #[arg(long)]
        id: String,
        #[arg(long)]
        requested_by: Option<String>,
        #[arg(long)]
        reason: Option<String>,
        #[arg(long, default_value_t = false)]
        keep_runtime_state: bool,
    },
    /// Inspect a configured mobile node manifest
    Inspect { id: String },
    /// Show derived readiness for a configured mobile node
    Status { id: String },
    /// Show bounded runtime state for a configured mobile node
    Runtime { id: String },
    /// Show bounded push-registration/runtime state for a configured mobile node
    PushStatus { id: String },
    /// Record bounded push-registration/runtime state for a configured mobile node
    RegisterPush {
        #[arg(long)]
        id: String,
        #[arg(long)]
        push_provider: Option<String>,
        #[arg(long)]
        push_token_present: Option<bool>,
        #[arg(long)]
        notifications_authorized: Option<bool>,
    },
    /// Show bounded sync-runtime state for a configured mobile node
    SyncStatus { id: String },
    /// Record bounded sync-runtime state for a configured mobile node
    ReportSync {
        #[arg(long)]
        id: String,
        #[arg(long)]
        sync_state: Option<String>,
        #[arg(long)]
        pending_change_count: Option<usize>,
        #[arg(long)]
        last_sync_result: Option<String>,
    },
    /// Show bounded capability inventory for a configured mobile node
    Capabilities { id: String },
    /// Show bounded aggregated runtime activity for a configured mobile node
    Activity {
        #[arg(long)]
        id: String,
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Record a bounded mobile node heartbeat/runtime receipt
    Heartbeat {
        #[arg(long)]
        id: String,
        #[arg(long)]
        app_state: Option<String>,
        #[arg(long)]
        network: Option<String>,
        #[arg(long)]
        reachable: Option<bool>,
        #[arg(long)]
        push_token_present: Option<bool>,
        #[arg(long)]
        battery_percent: Option<u8>,
    },
    /// Request a bounded wake ping for a mobile node
    WakeNode {
        #[arg(long)]
        id: String,
        #[arg(long)]
        requested_by: Option<String>,
        #[arg(long)]
        reason: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        body: Option<String>,
    },
    /// Request bounded disconnected-node rehydration for a mobile node
    RehydrateNode {
        #[arg(long)]
        id: String,
        #[arg(long)]
        requested_by: Option<String>,
        #[arg(long)]
        reason: Option<String>,
        #[arg(long)]
        pending_change_count: Option<usize>,
    },
    /// Preview a mobile message envelope without sending it
    PreviewMessage {
        #[arg(long)]
        source_node_id: String,
        #[arg(long)]
        target: String,
        #[arg(long)]
        content: String,
        #[arg(long)]
        content_type: Option<String>,
    },
    /// Preview a mobile push notification payload
    PreviewNotification {
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: String,
        #[arg(long)]
        priority: Option<String>,
        #[arg(long = "type")]
        notification_type: Option<String>,
        #[arg(long = "data")]
        data: Vec<String>,
    },
    /// List persisted bounded mobile notification receipts
    Notifications {
        #[arg(long)]
        node_id: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Inspect one bounded mobile notification receipt
    NotificationStatus { id: String },
    /// Send a bounded mobile notification through the shipped runtime lane
    SendNotification {
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: String,
        #[arg(long)]
        priority: Option<String>,
        #[arg(long = "type")]
        notification_type: Option<String>,
        #[arg(long = "data")]
        data: Vec<String>,
        #[arg(long)]
        requested_by: Option<String>,
    },
    /// Acknowledge one bounded mobile notification receipt
    AcknowledgeNotification {
        #[arg(long)]
        id: String,
        #[arg(long)]
        acknowledged_by: String,
    },
    /// List persisted bounded mobile inbound message receipts
    Inbox {
        #[arg(long)]
        node_id: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Inspect one bounded mobile inbound message receipt
    MessageStatus { id: String },
    /// Report one bounded mobile inbound message through the shipped runtime lane
    ReportMessage {
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        source: String,
        #[arg(long)]
        target: String,
        #[arg(long)]
        content: String,
        #[arg(long)]
        content_type: Option<String>,
    },
    /// Acknowledge one bounded mobile inbound message receipt
    AcknowledgeMessage {
        #[arg(long)]
        id: String,
        #[arg(long)]
        acknowledged_by: String,
    },
    /// List persisted bounded mobile outbound message receipts
    Outbox {
        #[arg(long)]
        node_id: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Inspect one bounded mobile outbound message receipt
    OutboundMessageStatus { id: String },
    /// Send one bounded mobile outbound message through the shipped runtime lane
    SendMessage {
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        target: String,
        #[arg(long)]
        content: String,
        #[arg(long)]
        content_type: Option<String>,
        #[arg(long)]
        requested_by: Option<String>,
    },
    /// Acknowledge one bounded mobile outbound message receipt
    AcknowledgeOutboundMessage {
        #[arg(long)]
        id: String,
        #[arg(long)]
        acknowledged_by: String,
    },
    /// Preview whether a node would sync under the given conditions
    PreviewSync {
        #[arg(long)]
        node_id: String,
        #[arg(long, default_value_t = 50)]
        battery_percent: u8,
        #[arg(long, default_value_t = 0)]
        pending_change_count: usize,
    },
    /// Preview a bounded mobile capability lane without executing it
    PreviewCapability {
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        capability: String,
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        query: Option<String>,
        #[arg(long)]
        note: Option<String>,
    },
    /// Execute a bounded mobile capability lane and persist an execution receipt
    ExecuteCapability {
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        capability: String,
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        query: Option<String>,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        requested_by: Option<String>,
    },
    /// List persisted bounded mobile capability execution receipts
    CapabilityExecutions {
        #[arg(long)]
        node_id: Option<String>,
        #[arg(long)]
        capability: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Inspect one bounded mobile capability execution receipt
    CapabilityExecutionStatus {
        #[arg(long)]
        id: String,
    },
    /// List bounded mobile media-artifact receipts derived from executed capability lanes
    MediaArtifacts {
        #[arg(long)]
        node_id: Option<String>,
        #[arg(long)]
        capability: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Inspect one bounded mobile media-artifact receipt
    MediaArtifactStatus {
        #[arg(long)]
        id: String,
    },
    /// List persisted mobile command receipts
    Commands {
        #[arg(long)]
        node_id: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Inspect one mobile command receipt
    CommandStatus { id: String },
    /// Dispatch a capability-gated mobile command
    DispatchCommand {
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        command: String,
        #[arg(long = "payload")]
        payload: Vec<String>,
        #[arg(long)]
        approved_by: Option<String>,
        #[arg(long)]
        require_approval: Option<bool>,
    },
    /// Approve and execute a pending mobile command
    ApproveCommand {
        #[arg(long)]
        id: String,
        #[arg(long)]
        decided_by: String,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Reject a pending mobile command
    RejectCommand {
        #[arg(long)]
        id: String,
        #[arg(long)]
        decided_by: String,
        #[arg(long)]
        reason: Option<String>,
    },
}

#[derive(Subcommand)]
enum MediaAction {
    /// Show bounded media provider and extractor readiness
    Providers {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Inspect a local media artifact and report bounded metadata
    Inspect {
        #[arg(long)]
        path: String,
    },
    /// Extract bounded text from a supported local media artifact
    ExtractText {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        path: String,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        language: Option<String>,
        #[arg(long)]
        prompt: Option<String>,
    },
    /// Describe a supported local media artifact through a bounded provider-backed lane
    Describe {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        path: String,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        prompt: Option<String>,
        #[arg(long)]
        max_tokens: Option<u32>,
    },
}

#[derive(Subcommand)]
enum VoiceAction {
    /// Show current voice runtime configuration and readiness
    Status {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Show supported voice providers and current readiness for STT/TTS lanes
    Providers {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// List persisted bounded voice sessions
    Sessions {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Summarize bounded voice session health and stale-idle state
    SessionHealth {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        stale_after_secs: Option<u64>,
    },
    /// Prewarm the shipped voice runtime through a bounded synthesis request
    Prewarm {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        voice: Option<String>,
        #[arg(long)]
        format: Option<String>,
        #[arg(long)]
        greeting: Option<String>,
        #[arg(long)]
        output_path: Option<String>,
    },
    /// Reap stale bounded voice sessions by idle timeout
    ReapSessions {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        stale_after_secs: Option<u64>,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Start a bounded voice session receipt
    StartSession {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long)]
        assistant_prompt: Option<String>,
        #[arg(long)]
        voice: Option<String>,
    },
    /// Inspect one bounded voice session receipt
    SessionStatus {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        id: String,
    },
    /// Summarize derived metrics across bounded voice sessions
    Metrics {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Inspect derived metrics for one bounded voice session receipt
    SessionMetrics {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        id: String,
    },
    /// Inspect the indexed transcript for one bounded voice session receipt
    Transcript {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        id: String,
    },
    /// Inspect synthesized output artifacts for one bounded voice session receipt
    Artifacts {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        id: String,
    },
    /// Inspect the derived event timeline for one bounded voice session receipt
    Events {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        id: String,
    },
    /// Append a user turn to one bounded voice session receipt
    AppendUser {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        id: String,
        #[arg(long)]
        text: String,
    },
    /// Generate a bounded assistant response for one voice session receipt
    Respond {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        id: String,
        #[arg(long)]
        text: String,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        voice: Option<String>,
        #[arg(long)]
        format: Option<String>,
        #[arg(long)]
        output_path: Option<String>,
    },
    /// Reconnect one bounded voice session receipt and optionally emit a greeting
    ReconnectSession {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        id: String,
        #[arg(long)]
        assistant_prompt: Option<String>,
        #[arg(long)]
        voice: Option<String>,
        #[arg(long)]
        greeting: Option<String>,
        #[arg(long)]
        format: Option<String>,
        #[arg(long)]
        output_path: Option<String>,
    },
    /// Pause one bounded voice session receipt
    PauseSession {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        id: String,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Resume one bounded voice session receipt
    ResumeSession {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        id: String,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Interrupt one bounded voice session receipt
    InterruptSession {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        id: String,
        #[arg(long)]
        reason: Option<String>,
    },
    /// End one bounded voice session receipt
    EndSession {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        id: String,
        #[arg(long)]
        reason: Option<String>,
    },
    /// List the current provider's discoverable TTS voices
    Voices {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Transcribe a local audio file or remote audio URL through the configured provider lane
    Transcribe {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        language: Option<String>,
        #[arg(long)]
        prompt: Option<String>,
    },
    /// Synthesize text to an audio artifact through the configured provider lane
    Synthesize {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        #[arg(long)]
        text: String,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        voice: Option<String>,
        #[arg(long)]
        format: Option<String>,
        #[arg(long)]
        output_path: Option<String>,
    },
}

#[derive(Subcommand)]
enum OptimizeAction {
    /// List registered optimization targets
    ListTargets,
    /// Show one optimization target
    ShowTarget { id: String },
    /// Register a new optimization target
    RegisterTarget {
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        tier: String,
        #[arg(long, default_value = "safe_config")]
        risk_class: String,
        #[arg(long, default_value = "experimental")]
        ship_status: String,
        #[arg(long, default_value = ".")]
        workspace_root: String,
        #[arg(long = "allowed-path")]
        allowed_paths: Vec<String>,
        #[arg(long = "forbidden-path")]
        forbidden_paths: Vec<String>,
        #[arg(long = "allowed-field")]
        allowed_fields: Vec<String>,
        #[arg(long, default_value_t = 8)]
        max_changed_files: usize,
        #[arg(long, default_value_t = 32768)]
        max_total_bytes: usize,
        #[arg(long, default_value_t = 400)]
        max_diff_lines: usize,
        #[arg(long = "required-test")]
        required_tests: Vec<String>,
        #[arg(long = "mandatory-eval")]
        mandatory_evals: Vec<String>,
        #[arg(long = "eval")]
        evals: Vec<String>,
        #[arg(long)]
        metadata: Option<String>,
    },
    /// List optimization candidates
    ListCandidates {
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Show an optimization candidate and its history
    ShowCandidate { id: String },
    /// Submit a new optimization candidate from a JSON change set
    SubmitCandidate {
        #[arg(long)]
        target: String,
        #[arg(long)]
        hypothesis: String,
        #[arg(long, default_value = "operator")]
        proposed_by: String,
        #[arg(long)]
        change_set: String,
        #[arg(long)]
        trace_id: Option<String>,
    },
    /// Run the bounded experiment loop for one candidate
    RunCandidate { id: String },
    /// Approve a candidate without promoting it
    ApproveCandidate {
        id: String,
        #[arg(long, default_value = "operator")]
        decided_by: String,
        #[arg(long)]
        notes: Option<String>,
    },
    /// Reject a candidate
    RejectCandidate {
        id: String,
        #[arg(long, default_value = "operator")]
        decided_by: String,
        #[arg(long)]
        notes: Option<String>,
    },
    /// Record a promotion decision
    PromoteCandidate {
        id: String,
        #[arg(long)]
        decision: String,
        #[arg(long, default_value = "operator")]
        decided_by: String,
        #[arg(long)]
        notes: Option<String>,
        #[arg(long)]
        rollback_reference: Option<String>,
    },
}

#[derive(Subcommand)]
enum SecurityAction {
    /// Run security audit
    Audit,
    /// Generate Ed25519 keypair for skill signing
    GenerateKeys,
}

#[derive(Subcommand)]
enum MemoryAction {
    /// Export memory to markdown for inspection
    Export {
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        user_id: Option<String>,
    },
    /// Import legacy OpenClaw MEMORY.md
    Import {
        #[arg(short, long)]
        file: String,
        #[arg(short, long)]
        user_id: String,
    },
    /// Show memory statistics
    Stats,
    /// Get one memory entry by id
    Get {
        #[arg(long)]
        id: String,
    },
    /// Show recent memory timeline
    Timeline {
        #[arg(long)]
        namespace: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// List memory namespaces
    Namespaces,
    /// Inspect archive summaries
    Archive {
        #[arg(long)]
        namespace: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Export file-backed memory/persona views under .claw/
    ViewsExport {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(short, long)]
        user_id: Option<String>,
    },
    /// Import file-backed memory/persona views from .claw/
    ViewsImport {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(short, long)]
        user_id: String,
    },
    /// Scan model-aware workspace artifacts
    ArtifactsScan {
        #[arg(long, default_value = ".")]
        root: String,
    },
    /// Render the merged artifact bundle for a model
    ArtifactsRender {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        model: String,
    },
    /// Sync preferred artifact files for a model family
    ArtifactsSync {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        model: String,
    },
}

#[derive(Subcommand)]
enum SessionAction {
    /// List sessions
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Show one session plus history
    Show {
        id: String,
        #[arg(long, default_value_t = 50)]
        history_limit: usize,
    },
    /// Spawn a session explicitly
    Spawn {
        #[arg(long)]
        user_id: String,
        #[arg(long, default_value = "webchat")]
        platform: String,
        #[arg(long, default_value = "dm")]
        session_type: String,
        #[arg(long)]
        route_key: Option<String>,
        #[arg(long)]
        workspace_id: Option<String>,
    },
    /// Send a message through a persisted session
    Send {
        id: String,
        #[arg(long)]
        content: String,
    },
    /// Close or archive a session
    Close {
        id: String,
        #[arg(long)]
        archive: bool,
        #[arg(long)]
        reason: Option<String>,
    },
}

#[derive(Subcommand)]
enum ChannelAction {
    /// Initialize the standard .claw/channels/ registry layout
    Init {
        #[arg(long)]
        path: Option<String>,
    },
    /// List channel accounts and bindings
    List {
        #[arg(long)]
        path: Option<String>,
    },
    /// Show one channel account manifest
    ShowAccount {
        id: String,
        #[arg(long)]
        path: Option<String>,
    },
    /// Create or replace a channel account manifest
    CreateAccount {
        id: String,
        #[arg(long)]
        platform: String,
        #[arg(long)]
        external_user_id: String,
        #[arg(long)]
        display_name: Option<String>,
        #[arg(long)]
        workspace_id: Option<String>,
        #[arg(long)]
        channel_scope: Option<String>,
        #[arg(long)]
        workspace_target: Option<String>,
        #[arg(long)]
        agent_id: Option<String>,
        #[arg(long, default_value_t = false)]
        approved: bool,
        #[arg(long, default_value_t = false)]
        blocked: bool,
        #[arg(long, default_value_t = true)]
        enabled: bool,
        #[arg(long)]
        activation_mode: Option<String>,
        #[arg(long)]
        path: Option<String>,
    },
    /// Delete a channel account manifest
    DeleteAccount {
        id: String,
        #[arg(long)]
        path: Option<String>,
    },
    /// Approve a pending channel account
    Approve {
        id: String,
        #[arg(long)]
        path: Option<String>,
    },
    /// Block a channel account
    Block {
        id: String,
        #[arg(long)]
        path: Option<String>,
    },
    /// Set activation mode for a channel account
    Activation {
        id: String,
        mode: String,
        #[arg(long)]
        path: Option<String>,
    },
    /// Write or replace a binding manifest
    Bind {
        id: String,
        #[arg(long)]
        platform: String,
        #[arg(long)]
        workspace_match: Option<String>,
        #[arg(long)]
        account_match: Option<String>,
        #[arg(long)]
        channel_match: Option<String>,
        #[arg(long)]
        workspace_target: Option<String>,
        #[arg(long)]
        agent_id: Option<String>,
        #[arg(long)]
        activation_mode: Option<String>,
        #[arg(long)]
        path: Option<String>,
    },
    /// Show one binding manifest
    ShowBinding {
        id: String,
        #[arg(long)]
        path: Option<String>,
    },
    /// Delete a binding manifest
    DeleteBinding {
        id: String,
        #[arg(long)]
        path: Option<String>,
    },
}

#[derive(Subcommand)]
enum ControlAction {
    /// Initialize the standard .claw/control/ registry layout
    Init {
        #[arg(long)]
        path: Option<String>,
    },
    /// List agent profiles, model profiles, claws, and runtime mode
    List {
        #[arg(long)]
        path: Option<String>,
    },
    /// Show one manifest as YAML
    Show {
        kind: String,
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        path: Option<String>,
    },
    /// Validate registry references and runtime mode bindings
    Validate {
        #[arg(long)]
        path: Option<String>,
    },
    /// Machine-readable runtime self-description for users or Claw
    Describe {
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Create or replace an agent profile manifest
    CreateAgent {
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        model_profile: Option<String>,
        #[arg(long = "extends")]
        extends: Vec<String>,
        #[arg(long, default_value_t = 90)]
        timeout_secs: u64,
        #[arg(long)]
        memory_scope: Option<String>,
        #[arg(long)]
        output_policy: Option<String>,
        #[arg(long = "tool-allow")]
        tool_allow: Vec<String>,
        #[arg(long = "tool-deny")]
        tool_deny: Vec<String>,
        #[arg(long)]
        path: Option<String>,
    },
    /// Create or replace a model profile manifest
    CreateModel {
        id: String,
        #[arg(long)]
        provider: String,
        #[arg(long)]
        model: String,
        #[arg(long)]
        context_window: Option<usize>,
        #[arg(long)]
        max_output_tokens: Option<usize>,
        #[arg(long)]
        latency_hint: Option<String>,
        #[arg(long)]
        cost_hint: Option<String>,
        #[arg(long)]
        reasoning_hint: Option<String>,
        #[arg(long = "role-tag")]
        role_tags: Vec<String>,
        #[arg(long = "artifact-preference")]
        artifact_preferences: Vec<String>,
        #[arg(long = "fallback")]
        fallback_order: Vec<String>,
        #[arg(long)]
        path: Option<String>,
    },
    /// Create or replace a Claw manifest
    CreateClaw {
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        agent_profile: String,
        #[arg(long)]
        model_profile: String,
        #[arg(long)]
        role: Option<String>,
        #[arg(long = "category")]
        categories: Vec<String>,
        #[arg(long)]
        memory_scope: Option<String>,
        #[arg(long)]
        path: Option<String>,
    },
    /// Set the top-level runtime execution mode
    Mode {
        mode: String,
        #[arg(long)]
        default_claw: Option<String>,
        #[arg(long)]
        orchestrator_claw: Option<String>,
        #[arg(long)]
        allow_shared_context: bool,
        #[arg(long)]
        isolation_mode: Option<String>,
        #[arg(long)]
        autonomy_level: Option<String>,
        #[arg(long)]
        yolo_mode: Option<bool>,
        #[arg(long)]
        steering_enabled: Option<bool>,
        #[arg(long)]
        decision_learning_enabled: Option<bool>,
        #[arg(long)]
        critic_enabled: Option<bool>,
        #[arg(long)]
        max_delegations: Option<usize>,
        #[arg(long)]
        max_iterations: Option<usize>,
        #[arg(long)]
        max_runtime_secs: Option<u64>,
        #[arg(long)]
        max_lesson_hints: Option<usize>,
        #[arg(long)]
        approval_policy: Option<String>,
        #[arg(long)]
        path: Option<String>,
    },
    /// List decision lessons from the control registry
    Lessons {
        #[arg(long)]
        active_only: bool,
        #[arg(long)]
        path: Option<String>,
    },
    /// Create or replace a decision lesson manifest
    AddLesson {
        id: String,
        #[arg(long, default_value_t = true)]
        active: bool,
        #[arg(long)]
        signal: String,
        #[arg(long)]
        recommendation: String,
        #[arg(long)]
        rationale: Option<String>,
        #[arg(long, default_value_t = 0.7)]
        confidence: f32,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        task_id: Option<String>,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        claw_id: Option<String>,
        #[arg(long)]
        model_profile: Option<String>,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        autonomy_level: Option<String>,
        #[arg(long)]
        execution_mode: Option<String>,
        #[arg(long)]
        path: Option<String>,
    },
    /// Mark a decision lesson inactive without deleting it
    DeactivateLesson {
        id: String,
        #[arg(long)]
        path: Option<String>,
    },
    /// Assign one task id to a Claw
    AssignTask {
        task_id: String,
        claw_id: String,
        #[arg(long)]
        path: Option<String>,
    },
    /// Assign one task category to a Claw
    AssignCategory {
        category: String,
        claw_id: String,
        #[arg(long)]
        path: Option<String>,
    },
}

#[cfg(feature = "cursor")]
#[derive(Subcommand)]
enum CursorAction {
    /// Generate .cursor/mcp.json and .cursor/rules/
    Setup,
    /// Start the Cursor ACP server
    Start {
        /// Transport type (stdio or tcp)
        #[arg(short, long, default_value = "stdio")]
        transport: String,
        /// Port for TCP transport
        #[arg(short, long, default_value = "9000")]
        port: u16,
    },
    /// Check Cursor IDE integration status
    Status,
}

#[derive(Subcommand)]
#[clap(rename_all = "kebab-case")]
#[command(disable_help_subcommand = true)]
enum Mcp2CliAction {
    /// List available tools (~16 tokens/tool)
    List {
        #[arg(long, group = "source")]
        saved: Option<String>,
        #[arg(long, group = "source")]
        mcp: Option<String>,
        #[arg(long, group = "source")]
        mcp_stdio: Option<String>,
        #[arg(long, group = "source")]
        spec: Option<String>,
        #[arg(long)]
        base_url: Option<String>,
        #[arg(long)]
        refresh: bool,
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Get tool help (~80-200 tokens)
    Help {
        #[arg(long, group = "source")]
        saved: Option<String>,
        #[arg(long, group = "source")]
        mcp: Option<String>,
        #[arg(long, group = "source")]
        mcp_stdio: Option<String>,
        #[arg(long, group = "source")]
        spec: Option<String>,
        tool: String,
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Execute a tool
    Run {
        #[arg(long, group = "source")]
        saved: Option<String>,
        #[arg(long, group = "source")]
        mcp: Option<String>,
        #[arg(long, group = "source")]
        mcp_stdio: Option<String>,
        #[arg(long, group = "source")]
        spec: Option<String>,
        tool: String,
        #[arg(long)]
        args: Option<String>,
        #[arg(long)]
        stdin: bool,
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Analyze token costs
    Analyze {
        #[arg(short, long, default_value = "30")]
        tools: usize,
        #[arg(short, long, default_value = "15")]
        turns: usize,
        #[arg(short, long, default_value = "5")]
        used: usize,
    },
    /// Convert to/from TOON format
    Toon {
        input: Option<String>,
        #[arg(long)]
        decode: bool,
    },
    /// Cache management
    Cache {
        #[command(subcommand)]
        action: Mcp2CliCacheAction,
    },
    /// Save and manage reusable MCP/OpenAPI sources
    Sources {
        #[command(subcommand)]
        action: Mcp2CliSourcesAction,
    },
}

#[derive(Subcommand)]
enum Mcp2CliCacheAction {
    /// Clear all cached data
    Clear,
    /// Show cache statistics
    Stats,
}

#[derive(Subcommand)]
enum Mcp2CliSourcesAction {
    /// List saved sources
    List,
    /// Save or update a source
    Add {
        name: String,
        #[arg(long, group = "source")]
        mcp: Option<String>,
        #[arg(long, group = "source")]
        mcp_stdio: Option<String>,
        #[arg(long, group = "source")]
        spec: Option<String>,
        #[arg(long)]
        base_url: Option<String>,
        #[arg(long)]
        description: Option<String>,
    },
    /// Show one saved source
    Show { name: String },
    /// Remove one saved source
    Remove { name: String },
}

#[derive(Subcommand)]
enum WebhooksAction {
    /// List configured webhooks
    List,
    /// Create a new webhook
    Create {
        /// Webhook path (e.g., "github", "stripe")
        path: String,
    },
    /// Delete a webhook
    Delete {
        /// Webhook path to delete
        path: String,
    },
    /// Enable a webhook
    Enable {
        /// Webhook path to enable
        path: String,
    },
    /// Disable a webhook
    Disable {
        /// Webhook path to disable
        path: String,
    },
    /// Show webhook details
    Info {
        /// Webhook path
        path: String,
    },
    /// Test a webhook by sending a sample request
    Test {
        /// Webhook path to test
        path: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { config, channels } => {
            commands::start::run(&config, channels.as_deref()).await
        }
        Commands::Chat { provider, model } => {
            commands::chat::run(&provider, model.as_deref()).await
        }
        Commands::Browser { action } => {
            let workspace_root = std::env::current_dir()?;
            match action {
                BrowserAction::OpenSession {
                    backend,
                    session_id,
                    label,
                    timeout_ms,
                } => {
                    let result = commands::browser::open_session(
                        &workspace_root,
                        commands::browser::BrowserOpenSessionRequest {
                            backend: Some(backend),
                            session_id,
                            label,
                            timeout_ms,
                        },
                    )?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                    Ok(())
                }
                BrowserAction::Sessions { limit } => {
                    let result = commands::browser::list_sessions(&workspace_root, limit)?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                    Ok(())
                }
                BrowserAction::Inspect {
                    url,
                    backend,
                    kind,
                    session_id,
                    selector,
                    interactive_only,
                    snapshot_depth,
                    wait_until,
                    timeout_ms,
                    path,
                } => {
                    let result = commands::browser::inspect(
                        &workspace_root,
                        commands::browser::BrowserInspectRequest {
                            url,
                            backend,
                            kind,
                            session_id,
                            selector,
                            interactive_only,
                            snapshot_depth,
                            wait_until,
                            timeout_ms,
                            path,
                        },
                    )
                    .await?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                    Ok(())
                }
                BrowserAction::RunSequence {
                    spec,
                    backend,
                    session_id,
                    timeout_ms,
                    path,
                } => {
                    let raw = std::fs::read_to_string(&spec)?;
                    let mut request: commands::browser::BrowserRunSequenceRequest =
                        serde_json::from_str(&raw)?;
                    if let Some(backend) = backend {
                        request.backend = backend;
                    }
                    if session_id.is_some() {
                        request.session_id = session_id;
                    }
                    if timeout_ms.is_some() {
                        request.timeout_ms = timeout_ms;
                    }
                    if path.is_some() {
                        request.path = path;
                    }
                    let result = commands::browser::run_sequence(&workspace_root, request).await?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                    Ok(())
                }
                BrowserAction::Artifacts { limit } => {
                    let result = commands::browser::list_artifacts(&workspace_root, limit)?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                    Ok(())
                }
                BrowserAction::ReadPage {
                    url,
                    max_chars,
                    path,
                    timeout_ms,
                } => {
                    let result = commands::browser::read_page(
                        &workspace_root,
                        commands::browser::BrowserReadPageRequest {
                            url,
                            max_chars,
                            path,
                            timeout_ms,
                        },
                    )
                    .await?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                    Ok(())
                }
                BrowserAction::CrawlSite {
                    url,
                    max_pages,
                    max_chars_per_page,
                    path,
                    timeout_ms,
                } => {
                    let result = commands::browser::crawl_site(
                        &workspace_root,
                        commands::browser::BrowserCrawlRequest {
                            url,
                            max_pages,
                            max_chars_per_page,
                            path,
                            timeout_ms,
                        },
                    )
                    .await?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                    Ok(())
                }
                BrowserAction::Navigate {
                    url,
                    backend,
                    session_id,
                    wait_until,
                    timeout_ms,
                } => {
                    let result = commands::browser::navigate(
                        &workspace_root,
                        commands::browser::BrowserNavigateRequest {
                            url,
                            backend,
                            session_id,
                            wait_until,
                            timeout_ms,
                        },
                    )
                    .await?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                    Ok(())
                }
                BrowserAction::Extract {
                    url,
                    backend,
                    session_id,
                    what,
                    selector,
                    wait_until,
                    timeout_ms,
                    max_results,
                    max_chars,
                } => {
                    let result = commands::browser::extract(
                        &workspace_root,
                        commands::browser::BrowserExtractRequest {
                            url,
                            backend,
                            session_id,
                            what,
                            selector,
                            wait_until,
                            timeout_ms,
                            max_results,
                            max_chars,
                        },
                    )
                    .await?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                    Ok(())
                }
                BrowserAction::Screenshot {
                    url,
                    backend,
                    session_id,
                    path,
                    selector,
                    full_page,
                    format,
                    quality,
                    wait_until,
                    timeout_ms,
                } => {
                    let result = commands::browser::screenshot(
                        &workspace_root,
                        commands::browser::BrowserScreenshotRequest {
                            url,
                            backend,
                            session_id,
                            path,
                            selector,
                            full_page,
                            format,
                            quality,
                            wait_until,
                            timeout_ms,
                        },
                    )
                    .await?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                    Ok(())
                }
                BrowserAction::Pdf {
                    url,
                    backend,
                    session_id,
                    path,
                    format,
                    print_background,
                    wait_until,
                    timeout_ms,
                } => {
                    let result = commands::browser::pdf(
                        &workspace_root,
                        commands::browser::BrowserPdfRequest {
                            url,
                            backend,
                            session_id,
                            path,
                            format,
                            print_background,
                            wait_until,
                            timeout_ms,
                        },
                    )
                    .await?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                    Ok(())
                }
            }
        }
        Commands::Models { action } => match action {
            ModelsAction::List => commands::models::list().await,
            ModelsAction::Info { name } => commands::models::info(&name).await,
            ModelsAction::Scan => commands::models::scan().await,
        },
        Commands::Skills { action } => match action {
            SkillsAction::List => commands::skills::list().await,
            SkillsAction::ListExtensions => commands::skills::list_extensions().await,
            SkillsAction::Compile { name } => commands::skills::compile(name.as_deref()).await,
            SkillsAction::Refresh => commands::skills::refresh_compiled().await,
            SkillsAction::InspectCompiled { name } => {
                commands::skills::inspect_compiled(&name).await
            }
            SkillsAction::InspectExtension { name } => {
                commands::skills::inspect_extension(&name).await
            }
            SkillsAction::BackgroundServices { name } => {
                commands::skills::background_services(&name).await
            }
            SkillsAction::ListChannelExtensions => {
                commands::skills::list_channel_extensions().await
            }
            SkillsAction::ListAuthPlugins => commands::skills::list_auth_plugins().await,
            SkillsAction::ListVoicePlugins => commands::skills::list_voice_plugins().await,
            SkillsAction::BindChannelExtension {
                binding_id,
                skill_name,
                service,
                component,
                trigger,
            } => {
                commands::skills::bind_channel_extension(
                    &binding_id,
                    &skill_name,
                    service.as_deref(),
                    component.as_deref(),
                    trigger.as_deref(),
                )
                .await
            }
            SkillsAction::BindAuthPlugin {
                provider_id,
                skill_name,
                redirect_uri,
                issuer,
                authorization_endpoint,
                token_endpoint,
                client_id_key,
                client_secret_key,
                scopes,
                vault_key_prefix,
                service,
                component,
            } => {
                commands::skills::bind_auth_plugin(
                    &provider_id,
                    &skill_name,
                    redirect_uri.as_deref(),
                    issuer.as_deref(),
                    authorization_endpoint.as_deref(),
                    token_endpoint.as_deref(),
                    client_id_key.as_deref(),
                    client_secret_key.as_deref(),
                    scopes.as_deref(),
                    vault_key_prefix.as_deref(),
                    service.as_deref(),
                    component.as_deref(),
                )
                .await
            }
            SkillsAction::BindVoicePlugin {
                plugin_id,
                skill_name,
                service,
                component,
                greeting_text,
                default_voice,
            } => {
                commands::skills::bind_voice_plugin(
                    &plugin_id,
                    &skill_name,
                    service.as_deref(),
                    component.as_deref(),
                    greeting_text.as_deref(),
                    default_voice.as_deref(),
                )
                .await
            }
            SkillsAction::AuthAuthorize {
                provider_id,
                redirect_uri,
            } => {
                commands::skills::authorize_auth_plugin(&provider_id, redirect_uri.as_deref()).await
            }
            SkillsAction::AuthExchange {
                provider_id,
                code,
                state,
                redirect_uri,
            } => {
                commands::skills::exchange_auth_plugin(
                    &provider_id,
                    &code,
                    &state,
                    redirect_uri.as_deref(),
                )
                .await
            }
            SkillsAction::ListVoiceCalls => commands::skills::list_voice_calls().await,
            SkillsAction::VoiceCallHealth => commands::skills::voice_call_health().await,
            SkillsAction::StartVoiceCall {
                plugin_id,
                remote,
                greeting_text,
                voice,
                metadata,
                stale_after_secs,
            } => {
                commands::skills::start_voice_call(
                    &plugin_id,
                    remote.as_deref(),
                    greeting_text.as_deref(),
                    voice.as_deref(),
                    metadata.as_deref(),
                    stale_after_secs,
                )
                .await
            }
            SkillsAction::EndVoiceCall {
                call_id,
                reason,
                metadata,
            } => {
                commands::skills::end_voice_call(&call_id, reason.as_deref(), metadata.as_deref())
                    .await
            }
            SkillsAction::ReconnectVoiceCall {
                call_id,
                remote,
                greeting_text,
                voice,
                metadata,
                stale_after_secs,
            } => {
                commands::skills::reconnect_voice_call(
                    &call_id,
                    remote.as_deref(),
                    greeting_text.as_deref(),
                    voice.as_deref(),
                    metadata.as_deref(),
                    stale_after_secs,
                )
                .await
            }
            SkillsAction::PrewarmVoicePlugin {
                plugin_id,
                greeting_text,
                voice,
            } => {
                commands::skills::prewarm_voice_plugin(
                    &plugin_id,
                    greeting_text.as_deref(),
                    voice.as_deref(),
                )
                .await
            }
            SkillsAction::ReapVoiceCalls {
                stale_after_secs,
                limit,
            } => commands::skills::reap_voice_calls(stale_after_secs, limit).await,
            SkillsAction::Execute {
                name,
                component,
                input,
            } => commands::skills::execute(&name, component.as_deref(), input.as_deref()).await,
            SkillsAction::ScheduleBackground {
                name,
                service,
                component,
                input,
                every_seconds,
                at,
                priority,
            } => {
                commands::skills::schedule_background(
                    &name,
                    service.as_deref(),
                    component.as_deref(),
                    input.as_deref(),
                    every_seconds,
                    at.as_deref(),
                    priority,
                )
                .await
            }
            SkillsAction::Invoke {
                name,
                args,
                reference,
                detail,
            } => {
                commands::skills::invoke(&name, args.as_deref(), reference.as_deref(), detail).await
            }
            SkillsAction::Search {
                query,
                category,
                sort,
            } => commands::skills::search(&query, category.as_deref(), &sort).await,
            SkillsAction::Install { name } => commands::skills::install(&name).await,
            SkillsAction::Update { name } => commands::skills::update(&name).await,
            SkillsAction::Uninstall { name } => commands::skills::uninstall(&name).await,
            SkillsAction::Verify { name } => commands::skills::verify(&name).await,
            SkillsAction::Popular { limit } => commands::skills::popular(limit).await,
            SkillsAction::Trending { limit } => commands::skills::trending(limit).await,
        },
        Commands::Schedule { action } => match action {
            ScheduleAction::List => commands::schedule::list().await,
            ScheduleAction::Create {
                name,
                workflow,
                description,
                file,
                every_seconds,
                at,
                payload,
                priority,
                owner,
                tags,
            } => {
                commands::schedule::create(
                    name.as_deref(),
                    workflow.as_deref(),
                    description.as_deref(),
                    file.as_deref(),
                    every_seconds,
                    at.as_deref(),
                    payload.as_deref(),
                    priority,
                    owner.as_deref(),
                    &tags,
                )
                .await
            }
            ScheduleAction::Sync { path, dry_run } => {
                commands::schedule::sync(path.as_deref(), dry_run).await
            }
            ScheduleAction::Init { path } => commands::schedule::init(path.as_deref()).await,
            ScheduleAction::Export { id, output } => {
                commands::schedule::export(&id, output.as_deref()).await
            }
            ScheduleAction::Inspect { id } => commands::schedule::inspect(&id).await,
            ScheduleAction::Pause { id } => commands::schedule::pause(&id).await,
            ScheduleAction::Resume { id } => commands::schedule::resume(&id).await,
            ScheduleAction::RunNow { id } => commands::schedule::run_now(&id).await,
            ScheduleAction::Reprioritize { id, priority } => {
                commands::schedule::reprioritize(&id, priority).await
            }
            ScheduleAction::Rebind { id, workflow } => {
                commands::schedule::rebind(&id, &workflow).await
            }
            ScheduleAction::DisableUntil { id, until } => {
                commands::schedule::disable_until(&id, &until).await
            }
            ScheduleAction::Enable { id } => commands::schedule::enable(&id).await,
            ScheduleAction::Runs { job, limit } => {
                commands::schedule::runs(job.as_deref(), limit).await
            }
            ScheduleAction::DeadLetters { job, limit } => {
                commands::schedule::dead_letters(job.as_deref(), limit).await
            }
            ScheduleAction::ReplayDeadLetter { id } => {
                commands::schedule::replay_dead_letter(&id).await
            }
            ScheduleAction::Events { name, limit } => {
                commands::schedule::events(name.as_deref(), limit).await
            }
        },
        Commands::Meet { action } => match action {
            MeetAction::CreateSpace { config } => commands::meet::create_space(&config).await,
            MeetAction::GetSpace { name, config } => {
                commands::meet::get_space(&config, &name).await
            }
            MeetAction::EndSpace { name, config } => {
                commands::meet::end_space(&config, &name).await
            }
            MeetAction::ListRecords { limit, config } => {
                commands::meet::list_records(&config, limit).await
            }
            MeetAction::ListParticipants {
                conference_record,
                limit,
                config,
            } => commands::meet::list_participants(&config, &conference_record, limit).await,
            MeetAction::ListRecordings {
                conference_record,
                limit,
                config,
            } => commands::meet::list_recordings(&config, &conference_record, limit).await,
            MeetAction::ListTranscripts {
                conference_record,
                limit,
                config,
            } => commands::meet::list_transcripts(&config, &conference_record, limit).await,
            MeetAction::ShowTranscript {
                transcript,
                limit,
                config,
            } => commands::meet::show_transcript(&config, &transcript, limit).await,
            MeetAction::DecodeEvent { input, config } => {
                commands::meet::decode_event(&config, &input).await
            }
        },
        Commands::Matrix { action } => match action {
            MatrixAction::Join { room, config } => commands::matrix::join(&config, &room).await,
            MatrixAction::Leave { room, config } => commands::matrix::leave(&config, &room).await,
            MatrixAction::Rooms { config } => commands::matrix::rooms(&config).await,
            MatrixAction::SendFormatted {
                room,
                text,
                html,
                config,
            } => commands::matrix::send_formatted(&config, &room, &text, &html).await,
            MatrixAction::React {
                room,
                event_id,
                emoji,
                config,
            } => commands::matrix::react(&config, &room, &event_id, &emoji).await,
            MatrixAction::SendFile {
                room,
                file_path,
                filename,
                config,
            } => commands::matrix::send_file(&config, &room, &file_path, filename.as_deref()).await,
            MatrixAction::Typing {
                room,
                typing,
                config,
            } => commands::matrix::typing(&config, &room, typing).await,
            MatrixAction::Redact {
                room,
                event_id,
                reason,
                config,
            } => commands::matrix::redact(&config, &room, &event_id, reason.as_deref()).await,
        },
        Commands::Gmail { action } => match action {
            GmailAction::Status { config } => commands::gmail::status(&config).await,
            GmailAction::Connect { config } => commands::gmail::connect(&config).await,
            GmailAction::ProcessNotification { input, config } => {
                commands::gmail::process_notification(&config, &input).await
            }
            GmailAction::Send {
                config,
                to,
                cc,
                bcc,
                subject,
                body,
                thread_id,
                files,
            } => {
                commands::gmail::send(
                    &config,
                    &to,
                    &cc,
                    &bcc,
                    &subject,
                    &body,
                    thread_id.as_deref(),
                    &files,
                )
                .await
            }
            GmailAction::Reply {
                config,
                message_id,
                body,
                files,
            } => commands::gmail::reply(&config, &message_id, &body, &files).await,
            GmailAction::Label {
                config,
                message_id,
                add,
                remove,
            } => commands::gmail::label(&config, &message_id, &add, &remove).await,
            GmailAction::Archive { config, message_id } => {
                commands::gmail::archive(&config, &message_id).await
            }
            GmailAction::Delete { config, message_id } => {
                commands::gmail::delete(&config, &message_id).await
            }
            GmailAction::Forward {
                config,
                message_id,
                to,
                body,
                files,
            } => commands::gmail::forward(&config, &message_id, &to, &body, &files).await,
        },
        Commands::GoogleChat { action } => match action {
            GoogleChatAction::Status { config } => commands::google_chat::status(&config).await,
            GoogleChatAction::Connect { config } => commands::google_chat::connect(&config).await,
            GoogleChatAction::Send {
                config,
                space,
                message,
                thread,
            } => commands::google_chat::send(&config, &space, &message, thread.as_deref()).await,
            GoogleChatAction::SendCard {
                config,
                space,
                title,
                content,
                thread,
                subtitle,
                image_url,
                section_header,
            } => {
                commands::google_chat::send_card(
                    &config,
                    &space,
                    &title,
                    &content,
                    thread.as_deref(),
                    subtitle.as_deref(),
                    image_url.as_deref(),
                    section_header.as_deref(),
                )
                .await
            }
            GoogleChatAction::SendLinkCard {
                config,
                space,
                url,
                title,
                thread,
            } => {
                commands::google_chat::send_link_card(
                    &config,
                    &space,
                    &url,
                    title.as_deref(),
                    thread.as_deref(),
                )
                .await
            }
        },
        Commands::IMessage { action } => match action {
            IMessageAction::Ping { config } => commands::imessage::ping(&config).await,
            IMessageAction::Server { config } => commands::imessage::server(&config).await,
            IMessageAction::Chats {
                limit,
                offset,
                config,
            } => commands::imessage::chats(&config, limit, offset).await,
            IMessageAction::Contacts { config } => commands::imessage::contacts(&config).await,
            IMessageAction::Send {
                to,
                message,
                config,
            } => commands::imessage::send(&config, &to, &message).await,
            IMessageAction::SendFile {
                to,
                file_path,
                filename,
                message,
                config,
            } => {
                commands::imessage::send_file(
                    &config,
                    &to,
                    &file_path,
                    filename.as_deref(),
                    message.as_deref(),
                )
                .await
            }
            IMessageAction::Tapback {
                chat_guid,
                message_guid,
                reaction,
                config,
            } => commands::imessage::tapback(&config, &chat_guid, &message_guid, &reaction).await,
        },
        Commands::Signal { action } => match action {
            SignalAction::Register { voice, config } => {
                commands::signal::register(&config, voice).await
            }
            SignalAction::Verify { code, config } => commands::signal::verify(&config, &code).await,
            SignalAction::Link {
                device_name,
                config,
            } => commands::signal::link(&config, &device_name).await,
            SignalAction::ListGroups { config } => commands::signal::list_groups(&config).await,
        },
        Commands::WhatsApp { action } => match action {
            WhatsAppAction::Status { config } => commands::whatsapp::status(&config).await,
            WhatsAppAction::Connect { config } => commands::whatsapp::connect(&config).await,
            WhatsAppAction::Pair {
                config,
                timeout_secs,
            } => commands::whatsapp::pair(&config, timeout_secs).await,
            WhatsAppAction::Send {
                config,
                to,
                message,
            } => commands::whatsapp::send(&config, &to, &message).await,
            WhatsAppAction::SendMedia {
                config,
                to,
                media_type,
                path,
                caption,
            } => {
                commands::whatsapp::send_media(&config, &to, &media_type, &path, caption.as_deref())
                    .await
            }
        },
        Commands::Mobile { action } => {
            let workspace_root = std::env::current_dir()?;
            match action {
                MobileAction::List => commands::mobile::list_nodes(&workspace_root).await,
                MobileAction::Pair {
                    id,
                    gateway_url,
                    auth_token_env,
                    device_name,
                    platform,
                    capabilities,
                    enabled,
                } => {
                    commands::mobile::pair_node(
                        &workspace_root,
                        commands::mobile::MobilePairRequest {
                            id,
                            gateway_url,
                            auth_token_env,
                            device_name,
                            platform,
                            capabilities,
                            enabled,
                            sync: None,
                            notifications: None,
                            metadata: serde_json::Value::Null,
                        },
                    )
                    .await
                }
                MobileAction::Pairings { node_id, limit } => {
                    commands::mobile::list_pairings(&workspace_root, node_id.as_deref(), limit)
                        .await
                }
                MobileAction::Unpair {
                    id,
                    requested_by,
                    reason,
                    keep_runtime_state,
                } => {
                    commands::mobile::unpair_node(
                        &workspace_root,
                        &id,
                        commands::mobile::MobileUnpairRequest {
                            requested_by,
                            reason,
                            remove_runtime_state: !keep_runtime_state,
                        },
                    )
                    .await
                }
                MobileAction::Inspect { id } => {
                    commands::mobile::inspect_node(&workspace_root, &id).await
                }
                MobileAction::Status { id } => {
                    commands::mobile::node_status(&workspace_root, &id).await
                }
                MobileAction::Runtime { id } => {
                    commands::mobile::node_runtime(&workspace_root, &id).await
                }
                MobileAction::PushStatus { id } => {
                    commands::mobile::node_push_state(&workspace_root, &id).await
                }
                MobileAction::RegisterPush {
                    id,
                    push_provider,
                    push_token_present,
                    notifications_authorized,
                } => {
                    commands::mobile::register_push(
                        &workspace_root,
                        &id,
                        commands::mobile::MobilePushRegistrationRequest {
                            push_provider,
                            push_token_present,
                            notifications_authorized,
                        },
                    )
                    .await
                }
                MobileAction::SyncStatus { id } => {
                    commands::mobile::node_sync_state(&workspace_root, &id).await
                }
                MobileAction::ReportSync {
                    id,
                    sync_state,
                    pending_change_count,
                    last_sync_result,
                } => {
                    commands::mobile::report_sync(
                        &workspace_root,
                        &id,
                        commands::mobile::MobileSyncReportRequest {
                            sync_state,
                            pending_change_count,
                            last_sync_result,
                        },
                    )
                    .await
                }
                MobileAction::Capabilities { id } => {
                    commands::mobile::node_capabilities(&workspace_root, &id).await
                }
                MobileAction::Activity { id, limit } => {
                    commands::mobile::node_activity(&workspace_root, &id, limit).await
                }
                MobileAction::Heartbeat {
                    id,
                    app_state,
                    network,
                    reachable,
                    push_token_present,
                    battery_percent,
                } => {
                    commands::mobile::heartbeat_node(
                        &workspace_root,
                        &id,
                        commands::mobile::MobileHeartbeatRequest {
                            app_state,
                            network,
                            reachable,
                            push_token_present,
                            battery_percent,
                            metadata: serde_json::Value::Null,
                        },
                    )
                    .await
                }
                MobileAction::WakeNode {
                    id,
                    requested_by,
                    reason,
                    title,
                    body,
                } => {
                    commands::mobile::wake_node(
                        &workspace_root,
                        &id,
                        commands::mobile::MobileWakeRequest {
                            requested_by,
                            reason,
                            title,
                            body,
                        },
                    )
                    .await
                }
                MobileAction::RehydrateNode {
                    id,
                    requested_by,
                    reason,
                    pending_change_count,
                } => {
                    commands::mobile::rehydrate_node(
                        &workspace_root,
                        &id,
                        commands::mobile::MobileRehydrateRequest {
                            requested_by,
                            reason,
                            pending_change_count,
                        },
                    )
                    .await
                }
                MobileAction::PreviewMessage {
                    source_node_id,
                    target,
                    content,
                    content_type,
                } => {
                    commands::mobile::preview_message(
                        commands::mobile::MobileMessagePreviewRequest {
                            source_node_id,
                            target,
                            content,
                            content_type,
                        },
                    )
                    .await
                }
                MobileAction::PreviewNotification {
                    title,
                    body,
                    priority,
                    notification_type,
                    data,
                } => {
                    let data = data
                        .into_iter()
                        .filter_map(|entry| {
                            let (key, value) = entry.split_once('=')?;
                            Some((key.to_string(), value.to_string()))
                        })
                        .collect();
                    commands::mobile::preview_notification(
                        commands::mobile::MobileNotificationPreviewRequest {
                            title,
                            body,
                            priority,
                            notification_type,
                            data,
                        },
                    )
                    .await
                }
                MobileAction::Notifications { node_id, limit } => {
                    commands::mobile::list_notifications(&workspace_root, node_id.as_deref(), limit)
                        .await
                }
                MobileAction::NotificationStatus { id } => {
                    commands::mobile::inspect_notification(&workspace_root, &id).await
                }
                MobileAction::SendNotification {
                    node_id,
                    title,
                    body,
                    priority,
                    notification_type,
                    data,
                    requested_by,
                } => {
                    let data = data
                        .into_iter()
                        .filter_map(|entry| {
                            let (key, value) = entry.split_once('=')?;
                            Some((key.to_string(), value.to_string()))
                        })
                        .collect();
                    commands::mobile::send_notification(
                        &workspace_root,
                        commands::mobile::MobileNotificationSendRequest {
                            node_id,
                            title,
                            body,
                            priority,
                            notification_type,
                            data,
                            requested_by,
                        },
                    )
                    .await
                }
                MobileAction::AcknowledgeNotification {
                    id,
                    acknowledged_by,
                } => {
                    commands::mobile::acknowledge_notification(
                        &workspace_root,
                        &id,
                        commands::mobile::MobileNotificationAckRequest { acknowledged_by },
                    )
                    .await
                }
                MobileAction::Inbox { node_id, limit } => {
                    commands::mobile::list_inbox(&workspace_root, node_id.as_deref(), limit).await
                }
                MobileAction::MessageStatus { id } => {
                    commands::mobile::inspect_inbox_message(&workspace_root, &id).await
                }
                MobileAction::ReportMessage {
                    node_id,
                    source,
                    target,
                    content,
                    content_type,
                } => {
                    commands::mobile::report_inbox_message(
                        &workspace_root,
                        commands::mobile::MobileInboundMessageReportRequest {
                            node_id,
                            source,
                            target,
                            content,
                            content_type,
                            metadata: Value::Null,
                        },
                    )
                    .await
                }
                MobileAction::AcknowledgeMessage {
                    id,
                    acknowledged_by,
                } => {
                    commands::mobile::acknowledge_inbox_message(
                        &workspace_root,
                        &id,
                        commands::mobile::MobileInboundMessageAckRequest { acknowledged_by },
                    )
                    .await
                }
                MobileAction::Outbox { node_id, limit } => {
                    commands::mobile::list_outbox(&workspace_root, node_id.as_deref(), limit).await
                }
                MobileAction::OutboundMessageStatus { id } => {
                    commands::mobile::inspect_outbox_message(&workspace_root, &id).await
                }
                MobileAction::SendMessage {
                    node_id,
                    target,
                    content,
                    content_type,
                    requested_by,
                } => {
                    commands::mobile::send_outbox_message(
                        &workspace_root,
                        commands::mobile::MobileOutboundMessageSendRequest {
                            node_id,
                            target,
                            content,
                            content_type,
                            requested_by,
                            metadata: Value::Null,
                        },
                    )
                    .await
                }
                MobileAction::AcknowledgeOutboundMessage {
                    id,
                    acknowledged_by,
                } => {
                    commands::mobile::acknowledge_outbox_message(
                        &workspace_root,
                        &id,
                        commands::mobile::MobileOutboundMessageAckRequest { acknowledged_by },
                    )
                    .await
                }
                MobileAction::PreviewSync {
                    node_id,
                    battery_percent,
                    pending_change_count,
                } => {
                    commands::mobile::preview_sync(
                        &workspace_root,
                        commands::mobile::MobileSyncPreviewRequest {
                            node_id,
                            battery_percent,
                            pending_change_count,
                        },
                    )
                    .await
                }
                MobileAction::PreviewCapability {
                    node_id,
                    capability,
                    target,
                    query,
                    note,
                } => {
                    commands::mobile::preview_capability(
                        &workspace_root,
                        commands::mobile::MobileCapabilityPreviewRequest {
                            node_id,
                            capability,
                            target,
                            query,
                            note,
                        },
                    )
                    .await
                }
                MobileAction::ExecuteCapability {
                    node_id,
                    capability,
                    target,
                    query,
                    note,
                    requested_by,
                } => {
                    commands::mobile::execute_capability(
                        &workspace_root,
                        commands::mobile::MobileCapabilityExecuteRequest {
                            node_id,
                            capability,
                            target,
                            query,
                            note,
                            requested_by,
                        },
                    )
                    .await
                }
                MobileAction::CapabilityExecutions {
                    node_id,
                    capability,
                    limit,
                } => {
                    commands::mobile::list_capability_executions(
                        &workspace_root,
                        node_id.as_deref(),
                        capability.as_deref(),
                        limit,
                    )
                    .await
                }
                MobileAction::CapabilityExecutionStatus { id } => {
                    commands::mobile::inspect_capability_execution(&workspace_root, &id).await
                }
                MobileAction::MediaArtifacts {
                    node_id,
                    capability,
                    limit,
                } => {
                    commands::mobile::list_media_artifacts(
                        &workspace_root,
                        node_id.as_deref(),
                        capability.as_deref(),
                        limit,
                    )
                    .await
                }
                MobileAction::MediaArtifactStatus { id } => {
                    commands::mobile::inspect_media_artifact(&workspace_root, &id).await
                }
                MobileAction::Commands { node_id, limit } => {
                    commands::mobile::list_commands(&workspace_root, node_id.as_deref(), limit)
                        .await
                }
                MobileAction::CommandStatus { id } => {
                    commands::mobile::inspect_command(&workspace_root, &id).await
                }
                MobileAction::DispatchCommand {
                    node_id,
                    command,
                    payload,
                    approved_by,
                    require_approval,
                } => {
                    let command = command
                        .parse::<DeviceCommandKind>()
                        .map_err(anyhow::Error::msg)?;
                    let payload = payload
                        .into_iter()
                        .filter_map(|entry| {
                            let (key, value) = entry.split_once('=')?;
                            Some((
                                key.to_string(),
                                serde_json::Value::String(value.to_string()),
                            ))
                        })
                        .collect::<serde_json::Map<String, serde_json::Value>>();
                    commands::mobile::dispatch_command(
                        &workspace_root,
                        commands::mobile::MobileCommandDispatchRequest {
                            node_id,
                            command,
                            payload: serde_json::Value::Object(payload),
                            approved_by,
                            require_approval,
                        },
                    )
                    .await
                }
                MobileAction::ApproveCommand {
                    id,
                    decided_by,
                    reason,
                } => {
                    commands::mobile::approve_command(
                        &workspace_root,
                        &id,
                        commands::mobile::MobileCommandDecisionRequest { decided_by, reason },
                    )
                    .await
                }
                MobileAction::RejectCommand {
                    id,
                    decided_by,
                    reason,
                } => {
                    commands::mobile::reject_command(
                        &workspace_root,
                        &id,
                        commands::mobile::MobileCommandDecisionRequest { decided_by, reason },
                    )
                    .await
                }
            }
        }
        Commands::Media { action } => match action {
            MediaAction::Providers { config } => commands::media::providers(&config).await,
            MediaAction::Inspect { path } => {
                commands::media::inspect(commands::media::MediaInspectRequest { path }).await
            }
            MediaAction::ExtractText {
                config,
                path,
                provider,
                model,
                language,
                prompt,
            } => {
                commands::media::extract_text(
                    &config,
                    commands::media::MediaExtractTextRequest {
                        path,
                        provider,
                        model,
                        language,
                        prompt,
                        max_audio_bytes: None,
                        timeout_secs: None,
                    },
                )
                .await
            }
            MediaAction::Describe {
                config,
                path,
                provider,
                model,
                prompt,
                max_tokens,
            } => {
                commands::media::describe(
                    &config,
                    commands::media::MediaDescribeRequest {
                        path,
                        provider,
                        model,
                        prompt,
                        max_tokens,
                    },
                )
                .await
            }
        },
        Commands::Control { action } => match action {
            ControlAction::Init { path } => commands::control::init(path.as_deref()),
            ControlAction::List { path } => commands::control::list(path.as_deref()),
            ControlAction::Show { kind, id, path } => {
                commands::control::show(path.as_deref(), &kind, id.as_deref())
            }
            ControlAction::Validate { path } => commands::control::validate(path.as_deref()),
            ControlAction::Describe { path, json } => {
                commands::control::describe(path.as_deref(), json)
            }
            ControlAction::CreateAgent {
                id,
                name,
                model_profile,
                extends,
                timeout_secs,
                memory_scope,
                output_policy,
                tool_allow,
                tool_deny,
                path,
            } => commands::control::create_agent(
                path.as_deref(),
                &id,
                name.as_deref(),
                model_profile.as_deref(),
                &extends,
                timeout_secs,
                memory_scope.as_deref(),
                output_policy.as_deref(),
                &tool_allow,
                &tool_deny,
            ),
            ControlAction::CreateModel {
                id,
                provider,
                model,
                context_window,
                max_output_tokens,
                latency_hint,
                cost_hint,
                reasoning_hint,
                role_tags,
                artifact_preferences,
                fallback_order,
                path,
            } => commands::control::create_model(
                path.as_deref(),
                &id,
                &provider,
                &model,
                context_window,
                max_output_tokens,
                latency_hint.as_deref(),
                cost_hint.as_deref(),
                reasoning_hint.as_deref(),
                &role_tags,
                &artifact_preferences,
                &fallback_order,
            ),
            ControlAction::CreateClaw {
                id,
                name,
                agent_profile,
                model_profile,
                role,
                categories,
                memory_scope,
                path,
            } => commands::control::create_claw(
                path.as_deref(),
                &id,
                name.as_deref(),
                &agent_profile,
                &model_profile,
                role.as_deref(),
                &categories,
                memory_scope.as_deref(),
            ),
            ControlAction::Mode {
                mode,
                default_claw,
                orchestrator_claw,
                allow_shared_context,
                isolation_mode,
                autonomy_level,
                yolo_mode,
                steering_enabled,
                decision_learning_enabled,
                critic_enabled,
                max_delegations,
                max_iterations,
                max_runtime_secs,
                max_lesson_hints,
                approval_policy,
                path,
            } => commands::control::configure_mode(
                path.as_deref(),
                &mode,
                default_claw.as_deref(),
                orchestrator_claw.as_deref(),
                allow_shared_context,
                isolation_mode.as_deref(),
                autonomy_level.as_deref(),
                yolo_mode,
                steering_enabled,
                decision_learning_enabled,
                critic_enabled,
                max_delegations,
                max_iterations,
                max_runtime_secs,
                max_lesson_hints,
                approval_policy.as_deref(),
            ),
            ControlAction::Lessons { active_only, path } => {
                commands::control::list_lessons(path.as_deref(), active_only)
            }
            ControlAction::AddLesson {
                id,
                active,
                signal,
                recommendation,
                rationale,
                confidence,
                source,
                task_id,
                category,
                claw_id,
                model_profile,
                provider,
                autonomy_level,
                execution_mode,
                path,
            } => commands::control::create_lesson(
                path.as_deref(),
                commands::control::NewLessonInput {
                    id: &id,
                    active,
                    signal: &signal,
                    recommendation: &recommendation,
                    rationale: rationale.as_deref(),
                    confidence,
                    source: source.as_deref(),
                    task_id: task_id.as_deref(),
                    category: category.as_deref(),
                    claw_id: claw_id.as_deref(),
                    model_profile_id: model_profile.as_deref(),
                    provider: provider.as_deref(),
                    autonomy_level: autonomy_level.as_deref(),
                    execution_mode: execution_mode.as_deref(),
                },
            ),
            ControlAction::DeactivateLesson { id, path } => {
                commands::control::deactivate_lesson(path.as_deref(), &id)
            }
            ControlAction::AssignTask {
                task_id,
                claw_id,
                path,
            } => commands::control::assign_task(path.as_deref(), &task_id, &claw_id),
            ControlAction::AssignCategory {
                category,
                claw_id,
                path,
            } => commands::control::assign_category(path.as_deref(), &category, &claw_id),
        },
        Commands::Runtime { action } => match action {
            RuntimeAction::Status { config } => {
                let workspace_root = std::env::current_dir()?;
                let status = commands::runtime::runtime_status(&config, &workspace_root)?;
                println!("{}", serde_json::to_string_pretty(&status)?);
                Ok(())
            }
            RuntimeAction::Health { config, refresh } => {
                let workspace_root = std::env::current_dir()?;
                let report =
                    commands::runtime::runtime_health_status(&config, &workspace_root, refresh)
                        .await?;
                println!("{}", serde_json::to_string_pretty(&report)?);
                Ok(())
            }
            RuntimeAction::Reload { config } => {
                let workspace_root = std::env::current_dir()?;
                let status = commands::runtime::validate_runtime_reload(&config, &workspace_root)?;
                let plan = commands::runtime::runtime_reload_plan(&config, &workspace_root)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "validated",
                        "runtime": status,
                        "reload_plan": plan,
                    }))?
                );
                Ok(())
            }
            RuntimeAction::ReloadPlan { config } => {
                let workspace_root = std::env::current_dir()?;
                let plan = commands::runtime::runtime_reload_plan(&config, &workspace_root)?;
                println!("{}", serde_json::to_string_pretty(&plan)?);
                Ok(())
            }
            RuntimeAction::SwitchProvider {
                config,
                provider,
                model,
                api_key_env,
                fallback_chain,
            } => {
                let workspace_root = std::env::current_dir()?;
                let updated = commands::runtime::switch_provider(
                    &config,
                    &workspace_root,
                    &provider,
                    model.as_deref(),
                    api_key_env.as_deref(),
                    (!fallback_chain.is_empty()).then_some(fallback_chain),
                )?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "ok",
                        "default_provider": updated.providers.default_provider,
                        "fallback_chain": updated.providers.fallback_chain,
                    }))?
                );
                Ok(())
            }
            RuntimeAction::SwitchModel {
                config,
                provider,
                model,
            } => {
                let workspace_root = std::env::current_dir()?;
                let _updated =
                    commands::runtime::switch_model(&config, &workspace_root, &provider, &model)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "ok",
                        "provider": provider,
                        "model": model,
                    }))?
                );
                Ok(())
            }
            RuntimeAction::Vault { action } => {
                let workspace_root = std::env::current_dir()?;
                match action {
                    RuntimeVaultAction::Status => {
                        let vault_path = commands::runtime::vault_path_for(&workspace_root);
                        let keys =
                            commands::runtime::list_vault_keys(&workspace_root).unwrap_or_default();
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "path": vault_path,
                                "present": vault_path.exists(),
                                "entries": keys,
                                "count": keys.len(),
                            }))?
                        );
                        Ok(())
                    }
                    RuntimeVaultAction::List => {
                        let keys = commands::runtime::list_vault_keys(&workspace_root)?;
                        println!("{}", serde_json::to_string_pretty(&keys)?);
                        Ok(())
                    }
                    RuntimeVaultAction::Set { key, value } => {
                        commands::runtime::set_vault_secret(&workspace_root, &key, &value)?;
                        println!("stored {}", key);
                        Ok(())
                    }
                    RuntimeVaultAction::Delete { key } => {
                        commands::runtime::delete_vault_secret(&workspace_root, &key)?;
                        println!("deleted {}", key);
                        Ok(())
                    }
                }
            }
            RuntimeAction::Services { action } => {
                let workspace_root = std::env::current_dir()?;
                match action {
                    RuntimeServicesAction::Status { config } => {
                        let report = commands::services::status(&config, &workspace_root).await?;
                        println!("{}", serde_json::to_string_pretty(&report)?);
                        Ok(())
                    }
                    RuntimeServicesAction::Scheduler { config } => {
                        let report =
                            commands::services::scheduler(&config, &workspace_root).await?;
                        println!("{}", serde_json::to_string_pretty(&report)?);
                        Ok(())
                    }
                    RuntimeServicesAction::Events {
                        config,
                        name,
                        limit,
                    } => {
                        let events = commands::services::runtime_events(
                            &config,
                            &workspace_root,
                            name.as_deref(),
                            limit,
                        )
                        .await?;
                        println!("{}", serde_json::to_string_pretty(&events)?);
                        Ok(())
                    }
                    RuntimeServicesAction::Channels { config, refresh } => {
                        let report = commands::services::channel_probes_status(
                            &config,
                            &workspace_root,
                            refresh,
                        )
                        .await?;
                        println!("{}", serde_json::to_string_pretty(&report)?);
                        Ok(())
                    }
                    RuntimeServicesAction::Logs { limit } => {
                        let entries = commands::logs::read_recent_logs(&workspace_root, limit)?;
                        println!("{}", serde_json::to_string_pretty(&entries)?);
                        Ok(())
                    }
                    RuntimeServicesAction::Beacon { config, refresh } => {
                        let beacon = commands::runtime::runtime_beacon_status(
                            &config,
                            &workspace_root,
                            refresh,
                            "not_running",
                            None,
                            false,
                        )
                        .await?;
                        println!("{}", serde_json::to_string_pretty(&beacon)?);
                        Ok(())
                    }
                }
            }
        },
        Commands::Orchestrate { action } => match action {
            OrchestrateAction::Resolve {
                task_id,
                category,
                claw,
                model_profile,
                worker_model_profile,
                autonomy_level,
                max_delegations,
                max_iterations,
                max_runtime_secs,
                approval_policy,
                mode,
            } => {
                let workspace_root = std::env::current_dir()?;
                let decision = commands::orchestrate::resolve(
                    commands::orchestrate::OrchestrationRequest {
                        prompt: String::new(),
                        task_id,
                        category,
                        claw_id: claw,
                        mode,
                        overrides: commands::orchestrate::OrchestrationRequestOverrides {
                            model_profile_id: model_profile,
                            worker_model_profile_id: worker_model_profile,
                            autonomy_level,
                            max_delegations,
                            max_iterations,
                            max_runtime_secs,
                            approval_policy,
                        },
                    },
                    &workspace_root,
                )?;
                println!("{}", serde_json::to_string_pretty(&decision)?);
                Ok(())
            }
            OrchestrateAction::List { limit } => {
                let workspace_root = std::env::current_dir()?;
                let runs = commands::orchestrate::list_runs(&workspace_root, limit)?;
                println!("{}", serde_json::to_string_pretty(&runs)?);
                Ok(())
            }
            OrchestrateAction::Inspect { receipt_id, json } => {
                let workspace_root = std::env::current_dir()?;
                let payload =
                    commands::orchestrate::read_run_supervision(&workspace_root, &receipt_id)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                } else {
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                }
                Ok(())
            }
            OrchestrateAction::Trace { receipt_id } => {
                let workspace_root = std::env::current_dir()?;
                let payload = commands::orchestrate::read_run_trace(&workspace_root, &receipt_id)?;
                println!("{}", serde_json::to_string_pretty(&payload)?);
                Ok(())
            }
            OrchestrateAction::Transcript { receipt_id } => {
                let workspace_root = std::env::current_dir()?;
                let payload =
                    commands::orchestrate::read_run_transcript(&workspace_root, &receipt_id)?;
                println!("{}", serde_json::to_string_pretty(&payload)?);
                Ok(())
            }
            OrchestrateAction::Resources { receipt_id } => {
                let workspace_root = std::env::current_dir()?;
                let payload =
                    commands::orchestrate::read_run_resources(&workspace_root, &receipt_id)?;
                println!("{}", serde_json::to_string_pretty(&payload)?);
                Ok(())
            }
            OrchestrateAction::Submit {
                prompt,
                task_id,
                category,
                claw,
                model_profile,
                worker_model_profile,
                autonomy_level,
                max_delegations,
                max_iterations,
                max_runtime_secs,
                approval_policy,
                mode,
            } => {
                let workspace_root = std::env::current_dir()?;
                let result = commands::orchestrate::submit(
                    commands::orchestrate::OrchestrationRequest {
                        prompt,
                        task_id,
                        category,
                        claw_id: claw,
                        mode,
                        overrides: commands::orchestrate::OrchestrationRequestOverrides {
                            model_profile_id: model_profile,
                            worker_model_profile_id: worker_model_profile,
                            autonomy_level,
                            max_delegations,
                            max_iterations,
                            max_runtime_secs,
                            approval_policy,
                        },
                    },
                    &workspace_root,
                )
                .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
                Ok(())
            }
            OrchestrateAction::Active { active_only, limit } => {
                let workspace_root = std::env::current_dir()?;
                let payload =
                    commands::orchestrate::list_active_runs(&workspace_root, active_only, limit)?;
                println!("{}", serde_json::to_string_pretty(&payload)?);
                Ok(())
            }
            OrchestrateAction::Watch { run_id, limit } => {
                let workspace_root = std::env::current_dir()?;
                let state = commands::orchestrate::read_active_run(&workspace_root, &run_id)?;
                let events =
                    commands::orchestrate::read_active_run_events(&workspace_root, &run_id, limit)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "state": state,
                        "events": events,
                    }))?
                );
                Ok(())
            }
            OrchestrateAction::Pause { run_id } => {
                let workspace_root = std::env::current_dir()?;
                let payload = commands::orchestrate::pause_active_run(&workspace_root, &run_id)?;
                println!("{}", serde_json::to_string_pretty(&payload)?);
                Ok(())
            }
            OrchestrateAction::Resume { run_id } => {
                let workspace_root = std::env::current_dir()?;
                let payload = commands::orchestrate::resume_active_run(&workspace_root, &run_id)?;
                println!("{}", serde_json::to_string_pretty(&payload)?);
                Ok(())
            }
            OrchestrateAction::Kill { run_id } => {
                let workspace_root = std::env::current_dir()?;
                let payload = commands::orchestrate::kill_active_run(&workspace_root, &run_id)?;
                println!("{}", serde_json::to_string_pretty(&payload)?);
                Ok(())
            }
            OrchestrateAction::PromoteCandidate {
                receipt_id,
                index,
                lesson_id,
                active,
                signal,
                recommendation,
                rationale,
                confidence,
                source,
                category,
                claw_id,
                model_profile,
                provider,
                autonomy_level,
                execution_mode,
            } => {
                let workspace_root = std::env::current_dir()?;
                let payload = commands::orchestrate::promote_reflection_candidate(
                    &workspace_root,
                    &receipt_id,
                    index,
                    commands::orchestrate::PromoteReflectionInput {
                        lesson_id,
                        active,
                        signal,
                        recommendation,
                        rationale,
                        confidence,
                        source,
                        category,
                        claw_id,
                        model_profile_id: model_profile,
                        provider,
                        autonomy_level,
                        execution_mode,
                    },
                )?;
                println!("{}", serde_json::to_string_pretty(&payload)?);
                Ok(())
            }
            OrchestrateAction::Run {
                prompt,
                task_id,
                category,
                claw,
                model_profile,
                worker_model_profile,
                autonomy_level,
                max_delegations,
                max_iterations,
                max_runtime_secs,
                approval_policy,
                mode,
                json,
            } => {
                let workspace_root = std::env::current_dir()?;
                let result = commands::orchestrate::run(
                    commands::orchestrate::OrchestrationRequest {
                        prompt,
                        task_id,
                        category,
                        claw_id: claw,
                        mode,
                        overrides: commands::orchestrate::OrchestrationRequestOverrides {
                            model_profile_id: model_profile,
                            worker_model_profile_id: worker_model_profile,
                            autonomy_level,
                            max_delegations,
                            max_iterations,
                            max_runtime_secs,
                            approval_policy,
                        },
                    },
                    &workspace_root,
                )
                .await?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("{}", result.final_output);
                    println!();
                    println!("receipt: {}", result.receipt_path);
                    println!(
                        "route: {} -> {} ({}/{})",
                        result.routing.route_source,
                        result.final_claw_id,
                        result.final_provider,
                        result.final_model
                    );
                }
                Ok(())
            }
            OrchestrateAction::WorkerRun { run_id } => {
                let workspace_root = std::env::current_dir()?;
                let payload = commands::orchestrate::worker_run(&run_id, &workspace_root).await?;
                println!("{}", serde_json::to_string_pretty(&payload)?);
                Ok(())
            }
        },
        Commands::Optimize { action } => match action {
            OptimizeAction::ListTargets => commands::optimize::list_targets().await,
            OptimizeAction::ShowTarget { id } => commands::optimize::show_target(&id).await,
            OptimizeAction::RegisterTarget {
                name,
                description,
                kind,
                tier,
                risk_class,
                ship_status,
                workspace_root,
                allowed_paths,
                forbidden_paths,
                allowed_fields,
                max_changed_files,
                max_total_bytes,
                max_diff_lines,
                required_tests,
                mandatory_evals,
                evals,
                metadata,
            } => {
                commands::optimize::register_target(
                    &name,
                    description.as_deref(),
                    &kind,
                    &tier,
                    &risk_class,
                    &ship_status,
                    &workspace_root,
                    &allowed_paths,
                    &forbidden_paths,
                    &allowed_fields,
                    max_changed_files,
                    max_total_bytes,
                    max_diff_lines,
                    &required_tests,
                    &mandatory_evals,
                    &evals,
                    metadata.as_deref(),
                )
                .await
            }
            OptimizeAction::ListCandidates {
                target,
                status,
                limit,
            } => {
                commands::optimize::list_candidates(target.as_deref(), status.as_deref(), limit)
                    .await
            }
            OptimizeAction::ShowCandidate { id } => commands::optimize::show_candidate(&id).await,
            OptimizeAction::SubmitCandidate {
                target,
                hypothesis,
                proposed_by,
                change_set,
                trace_id,
            } => {
                commands::optimize::submit_candidate(
                    &target,
                    &hypothesis,
                    &proposed_by,
                    &change_set,
                    trace_id.as_deref(),
                )
                .await
            }
            OptimizeAction::RunCandidate { id } => commands::optimize::run_candidate(&id).await,
            OptimizeAction::ApproveCandidate {
                id,
                decided_by,
                notes,
            } => commands::optimize::approve_candidate(&id, &decided_by, notes.as_deref()).await,
            OptimizeAction::RejectCandidate {
                id,
                decided_by,
                notes,
            } => commands::optimize::reject_candidate(&id, &decided_by, notes.as_deref()).await,
            OptimizeAction::PromoteCandidate {
                id,
                decision,
                decided_by,
                notes,
                rollback_reference,
            } => {
                commands::optimize::promote_candidate(
                    &id,
                    &decision,
                    &decided_by,
                    notes.as_deref(),
                    rollback_reference.as_deref(),
                )
                .await
            }
        },
        Commands::Security { action } => match action {
            SecurityAction::Audit => commands::security::audit().await,
            SecurityAction::GenerateKeys => commands::security::generate_keys().await,
        },
        Commands::Memory { action } => match action {
            MemoryAction::Export { output, user_id } => {
                commands::memory::export(&output, user_id.as_deref()).await
            }
            MemoryAction::Import { file, user_id } => {
                commands::memory::import(&file, &user_id).await
            }
            MemoryAction::Stats => commands::memory::stats().await,
            MemoryAction::Get { id } => commands::memory::get(&id).await,
            MemoryAction::Timeline { namespace, limit } => {
                commands::memory::timeline(namespace.as_deref(), limit).await
            }
            MemoryAction::Namespaces => commands::memory::namespaces().await,
            MemoryAction::Archive { namespace, limit } => {
                commands::memory::archive(namespace.as_deref(), limit).await
            }
            MemoryAction::ViewsExport { root, user_id } => {
                commands::memory::views_export(&root, user_id.as_deref()).await
            }
            MemoryAction::ViewsImport { root, user_id } => {
                commands::memory::views_import(&root, &user_id).await
            }
            MemoryAction::ArtifactsScan { root } => commands::memory::artifacts_scan(&root).await,
            MemoryAction::ArtifactsRender { root, model } => {
                commands::memory::artifacts_render(&root, &model).await
            }
            MemoryAction::ArtifactsSync { root, model } => {
                commands::memory::artifacts_sync(&root, &model).await
            }
        },
        Commands::Session { action } => match action {
            SessionAction::List { status, limit } => {
                commands::session::list(status.as_deref(), limit).await
            }
            SessionAction::Show { id, history_limit } => {
                commands::session::show(&id, history_limit).await
            }
            SessionAction::Spawn {
                user_id,
                platform,
                session_type,
                route_key,
                workspace_id,
            } => {
                commands::session::spawn(
                    &user_id,
                    &platform,
                    &session_type,
                    route_key.as_deref(),
                    workspace_id.as_deref(),
                )
                .await
            }
            SessionAction::Send { id, content } => commands::session::send(&id, &content).await,
            SessionAction::Close {
                id,
                archive,
                reason,
            } => commands::session::close(&id, archive, reason.as_deref()).await,
        },
        Commands::Channels { action } => match action {
            ChannelAction::Init { path } => commands::channels::init(path.as_deref()),
            ChannelAction::List { path } => commands::channels::list(path.as_deref()),
            ChannelAction::ShowAccount { id, path } => {
                commands::channels::show_account(path.as_deref(), &id)
            }
            ChannelAction::CreateAccount {
                id,
                platform,
                external_user_id,
                display_name,
                workspace_id,
                channel_scope,
                workspace_target,
                agent_id,
                approved,
                blocked,
                enabled,
                activation_mode,
                path,
            } => commands::channels::create_account(
                path.as_deref(),
                &id,
                &platform,
                &external_user_id,
                display_name.as_deref(),
                workspace_id.as_deref(),
                channel_scope.as_deref(),
                workspace_target.as_deref(),
                agent_id.as_deref(),
                approved,
                blocked,
                enabled,
                activation_mode.as_deref(),
            ),
            ChannelAction::DeleteAccount { id, path } => {
                commands::channels::delete_account(path.as_deref(), &id)
            }
            ChannelAction::Approve { id, path } => {
                commands::channels::approve(path.as_deref(), &id)
            }
            ChannelAction::Block { id, path } => commands::channels::block(path.as_deref(), &id),
            ChannelAction::Activation { id, mode, path } => {
                commands::channels::activation(path.as_deref(), &id, &mode)
            }
            ChannelAction::Bind {
                id,
                platform,
                workspace_match,
                account_match,
                channel_match,
                workspace_target,
                agent_id,
                activation_mode,
                path,
            } => commands::channels::bind(
                path.as_deref(),
                &id,
                &platform,
                workspace_match.as_deref(),
                account_match.as_deref(),
                channel_match.as_deref(),
                workspace_target.as_deref(),
                agent_id.as_deref(),
                activation_mode.as_deref(),
            ),
            ChannelAction::ShowBinding { id, path } => {
                commands::channels::show_binding(path.as_deref(), &id)
            }
            ChannelAction::DeleteBinding { id, path } => {
                commands::channels::delete_binding(path.as_deref(), &id)
            }
        },
        Commands::Doctor {
            repair,
            deep,
            non_interactive,
        } => commands::doctor::run(repair, deep, non_interactive).await,
        Commands::Onboard => commands::onboard::run().await,
        #[cfg(feature = "cursor")]
        Commands::Cursor { action } => match action {
            CursorAction::Setup => commands::cursor::setup().await,
            CursorAction::Start { transport, port } => {
                commands::cursor::start(&transport, port).await
            }
            CursorAction::Status => commands::cursor::status().await,
        },
        Commands::McpServer { transport, config } => {
            commands::start::run_mcp_server(&transport, &config).await
        }
        Commands::Mcp2Cli { action } => match action {
            Mcp2CliAction::List {
                saved,
                mcp,
                mcp_stdio,
                spec,
                base_url,
                refresh,
                format,
            } => {
                commands::mcp2cli::list(
                    saved,
                    mcp,
                    mcp_stdio,
                    spec,
                    base_url,
                    refresh,
                    parse_format(&format),
                )
                .await
            }
            Mcp2CliAction::Help {
                saved,
                mcp,
                mcp_stdio,
                spec,
                tool,
                format,
            } => {
                commands::mcp2cli::help_cmd(
                    saved,
                    mcp,
                    mcp_stdio,
                    spec,
                    tool,
                    parse_format(&format),
                )
                .await
            }
            Mcp2CliAction::Run {
                saved,
                mcp,
                mcp_stdio,
                spec,
                tool,
                args,
                stdin,
                format,
            } => {
                commands::mcp2cli::run(
                    saved,
                    mcp,
                    mcp_stdio,
                    spec,
                    tool,
                    args,
                    stdin,
                    parse_format(&format),
                )
                .await
            }
            Mcp2CliAction::Analyze { tools, turns, used } => {
                commands::mcp2cli::analyze(tools, turns, used).await
            }
            Mcp2CliAction::Toon { input, decode } => {
                commands::mcp2cli::toon_cmd(input, decode).await
            }
            Mcp2CliAction::Cache { action } => match action {
                Mcp2CliCacheAction::Clear => commands::mcp2cli::cache_clear().await,
                Mcp2CliCacheAction::Stats => commands::mcp2cli::cache_stats().await,
            },
            Mcp2CliAction::Sources { action } => match action {
                Mcp2CliSourcesAction::List => commands::mcp2cli::sources_list().await,
                Mcp2CliSourcesAction::Add {
                    name,
                    mcp,
                    mcp_stdio,
                    spec,
                    base_url,
                    description,
                } => {
                    commands::mcp2cli::sources_add(
                        name,
                        mcp,
                        mcp_stdio,
                        spec,
                        base_url,
                        description,
                    )
                    .await
                }
                Mcp2CliSourcesAction::Show { name } => commands::mcp2cli::sources_show(name).await,
                Mcp2CliSourcesAction::Remove { name } => {
                    commands::mcp2cli::sources_remove(name).await
                }
            },
        },
        #[cfg(feature = "voice")]
        Commands::Talk {
            provider,
            model,
            wake_word,
            silence_timeout,
            max_utterance,
            barge_in,
        } => {
            commands::talk::run(
                &provider,
                model.as_deref(),
                &wake_word,
                silence_timeout,
                max_utterance,
                barge_in,
            )
            .await
        }
        Commands::Voice { action } => match action {
            VoiceAction::Status { config } => commands::voice::status(&config).await,
            VoiceAction::Providers { config } => commands::voice::providers(&config).await,
            VoiceAction::Sessions { config } => commands::voice::sessions(&config).await,
            VoiceAction::SessionHealth {
                config,
                stale_after_secs,
            } => commands::voice::session_health(&config, stale_after_secs).await,
            VoiceAction::Prewarm {
                config,
                provider,
                model,
                voice,
                format,
                greeting,
                output_path,
            } => {
                commands::voice::prewarm(
                    &config,
                    provider.as_deref(),
                    model.as_deref(),
                    voice.as_deref(),
                    format.as_deref(),
                    greeting.as_deref(),
                    output_path.as_deref(),
                )
                .await
            }
            VoiceAction::ReapSessions {
                config,
                stale_after_secs,
                reason,
            } => commands::voice::reap_sessions(&config, stale_after_secs, reason.as_deref()).await,
            VoiceAction::StartSession {
                config,
                session_id,
                assistant_prompt,
                voice,
            } => {
                commands::voice::start_session(
                    &config,
                    session_id.as_deref(),
                    assistant_prompt.as_deref(),
                    voice.as_deref(),
                )
                .await
            }
            VoiceAction::SessionStatus { config, id } => {
                commands::voice::session_status(&config, &id).await
            }
            VoiceAction::Metrics { config } => commands::voice::metrics(&config).await,
            VoiceAction::SessionMetrics { config, id } => {
                commands::voice::session_metrics(&config, &id).await
            }
            VoiceAction::Transcript { config, id } => {
                commands::voice::transcript(&config, &id).await
            }
            VoiceAction::Artifacts { config, id } => commands::voice::artifacts(&config, &id).await,
            VoiceAction::Events { config, id } => commands::voice::events(&config, &id).await,
            VoiceAction::AppendUser { config, id, text } => {
                commands::voice::append_user(&config, &id, &text).await
            }
            VoiceAction::Respond {
                config,
                id,
                text,
                provider,
                model,
                voice,
                format,
                output_path,
            } => {
                commands::voice::respond(
                    &config,
                    &id,
                    &text,
                    provider.as_deref(),
                    model.as_deref(),
                    voice.as_deref(),
                    format.as_deref(),
                    output_path.as_deref(),
                )
                .await
            }
            VoiceAction::ReconnectSession {
                config,
                id,
                assistant_prompt,
                voice,
                greeting,
                format,
                output_path,
            } => {
                commands::voice::reconnect_session(
                    &config,
                    &id,
                    assistant_prompt.as_deref(),
                    voice.as_deref(),
                    greeting.as_deref(),
                    format.as_deref(),
                    output_path.as_deref(),
                )
                .await
            }
            VoiceAction::PauseSession { config, id, reason } => {
                commands::voice::pause_session(&config, &id, reason.as_deref()).await
            }
            VoiceAction::ResumeSession { config, id, reason } => {
                commands::voice::resume_session(&config, &id, reason.as_deref()).await
            }
            VoiceAction::InterruptSession { config, id, reason } => {
                commands::voice::interrupt_session(&config, &id, reason.as_deref()).await
            }
            VoiceAction::EndSession { config, id, reason } => {
                commands::voice::end_session(&config, &id, reason.as_deref()).await
            }
            VoiceAction::Voices { config } => commands::voice::voices(&config).await,
            VoiceAction::Transcribe {
                config,
                path,
                url,
                provider,
                model,
                language,
                prompt,
            } => {
                commands::voice::transcribe(
                    &config,
                    path.as_deref(),
                    url.as_deref(),
                    provider.as_deref(),
                    model.as_deref(),
                    language.as_deref(),
                    prompt.as_deref(),
                )
                .await
            }
            VoiceAction::Synthesize {
                config,
                text,
                provider,
                model,
                voice,
                format,
                output_path,
            } => {
                commands::voice::synthesize(
                    &config,
                    &text,
                    provider.as_deref(),
                    model.as_deref(),
                    voice.as_deref(),
                    format.as_deref(),
                    output_path.as_deref(),
                )
                .await
            }
        },
        Commands::Webhooks { action } => match action {
            WebhooksAction::List => commands::webhooks::list().await,
            WebhooksAction::Create { path } => commands::webhooks::create(&path).await,
            WebhooksAction::Delete { path } => commands::webhooks::delete(&path).await,
            WebhooksAction::Enable { path } => commands::webhooks::enable(&path).await,
            WebhooksAction::Disable { path } => commands::webhooks::disable(&path).await,
            WebhooksAction::Info { path } => commands::webhooks::info(&path).await,
            WebhooksAction::Test { path } => commands::webhooks::test(&path).await,
        },
    }
}

fn parse_format(s: &str) -> commands::mcp2cli::OutputFormat {
    match s.to_lowercase().as_str() {
        "json" => commands::mcp2cli::OutputFormat::Json,
        "toon" => commands::mcp2cli::OutputFormat::Toon,
        _ => commands::mcp2cli::OutputFormat::Table,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    // --- parse_format tests ---

    #[test]
    fn test_parse_format_json() {
        matches!(parse_format("json"), commands::mcp2cli::OutputFormat::Json);
    }

    #[test]
    fn test_parse_format_json_uppercase() {
        matches!(parse_format("JSON"), commands::mcp2cli::OutputFormat::Json);
    }

    #[test]
    fn test_parse_format_toon() {
        matches!(parse_format("toon"), commands::mcp2cli::OutputFormat::Toon);
    }

    #[test]
    fn test_parse_format_table() {
        matches!(
            parse_format("table"),
            commands::mcp2cli::OutputFormat::Table
        );
    }

    #[test]
    fn test_parse_format_unknown_defaults_to_table() {
        matches!(parse_format("xyz"), commands::mcp2cli::OutputFormat::Table);
    }

    #[test]
    fn test_parse_format_empty_defaults_to_table() {
        matches!(parse_format(""), commands::mcp2cli::OutputFormat::Table);
    }

    // --- CLI argument parsing tests ---

    #[test]
    fn test_cli_parse_doctor() {
        let cli = Cli::try_parse_from(["openrustclaw", "doctor"]);
        assert!(cli.is_ok());
        match cli.unwrap().command {
            Commands::Doctor {
                repair,
                deep,
                non_interactive,
            } => {
                assert!(!repair);
                assert!(!deep);
                assert!(!non_interactive);
            }
            _ => panic!("Expected Doctor command"),
        }
    }

    #[test]
    fn test_cli_parse_onboard() {
        let cli = Cli::try_parse_from(["openrustclaw", "onboard"]);
        assert!(cli.is_ok());
        matches!(cli.unwrap().command, Commands::Onboard);
    }

    #[test]
    fn test_cli_parse_control_init() {
        let cli = Cli::try_parse_from(["openrustclaw", "control", "init"]).unwrap();
        match cli.command {
            Commands::Control {
                action: ControlAction::Init { path },
            } => assert!(path.is_none()),
            _ => panic!("Expected Control::Init command"),
        }
    }

    #[test]
    fn test_cli_parse_start_defaults() {
        let cli = Cli::try_parse_from(["openrustclaw", "start"]).unwrap();
        match cli.command {
            Commands::Start { config, channels } => {
                assert_eq!(config, "config/default.toml");
                assert!(channels.is_none());
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_cli_parse_start_with_config() {
        let cli =
            Cli::try_parse_from(["openrustclaw", "start", "--config", "my_config.toml"]).unwrap();
        match cli.command {
            Commands::Start { config, channels } => {
                assert_eq!(config, "my_config.toml");
                assert!(channels.is_none());
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_cli_parse_start_with_channels() {
        let cli = Cli::try_parse_from(["openrustclaw", "start", "-C", "telegram,discord"]).unwrap();
        match cli.command {
            Commands::Start {
                config: _,
                channels,
            } => {
                assert_eq!(channels.as_deref(), Some("telegram,discord"));
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_cli_parse_chat_defaults() {
        let cli = Cli::try_parse_from(["openrustclaw", "chat"]).unwrap();
        match cli.command {
            Commands::Chat { provider, model } => {
                assert_eq!(provider, "anthropic");
                assert!(model.is_none());
            }
            _ => panic!("Expected Chat command"),
        }
    }

    #[test]
    fn test_cli_parse_chat_with_provider() {
        let cli = Cli::try_parse_from(["openrustclaw", "chat", "--provider", "openai"]).unwrap();
        match cli.command {
            Commands::Chat { provider, model } => {
                assert_eq!(provider, "openai");
                assert!(model.is_none());
            }
            _ => panic!("Expected Chat command"),
        }
    }

    #[test]
    fn test_cli_parse_chat_with_model() {
        let cli = Cli::try_parse_from([
            "openrustclaw",
            "chat",
            "--provider",
            "ollama",
            "--model",
            "llama3.2",
        ])
        .unwrap();
        match cli.command {
            Commands::Chat { provider, model } => {
                assert_eq!(provider, "ollama");
                assert_eq!(model.as_deref(), Some("llama3.2"));
            }
            _ => panic!("Expected Chat command"),
        }
    }

    #[test]
    fn test_cli_parse_models_list() {
        let cli = Cli::try_parse_from(["openrustclaw", "models", "list"]).unwrap();
        matches!(
            cli.command,
            Commands::Models {
                action: ModelsAction::List
            }
        );
    }

    #[test]
    fn test_cli_parse_models_info() {
        let cli = Cli::try_parse_from(["openrustclaw", "models", "info", "gpt-4o"]).unwrap();
        match cli.command {
            Commands::Models {
                action: ModelsAction::Info { name },
            } => {
                assert_eq!(name, "gpt-4o");
            }
            _ => panic!("Expected Models Info command"),
        }
    }

    #[test]
    fn test_cli_parse_security_audit() {
        let cli = Cli::try_parse_from(["openrustclaw", "security", "audit"]).unwrap();
        matches!(
            cli.command,
            Commands::Security {
                action: SecurityAction::Audit
            }
        );
    }

    #[test]
    fn test_cli_parse_security_generate_keys() {
        let cli = Cli::try_parse_from(["openrustclaw", "security", "generate-keys"]).unwrap();
        matches!(
            cli.command,
            Commands::Security {
                action: SecurityAction::GenerateKeys
            }
        );
    }

    #[test]
    fn test_cli_parse_memory_stats() {
        let cli = Cli::try_parse_from(["openrustclaw", "memory", "stats"]).unwrap();
        matches!(
            cli.command,
            Commands::Memory {
                action: MemoryAction::Stats
            }
        );
    }

    #[test]
    fn test_cli_parse_memory_export() {
        let cli = Cli::try_parse_from(["openrustclaw", "memory", "export", "--output", "dump.md"])
            .unwrap();
        match cli.command {
            Commands::Memory {
                action: MemoryAction::Export { output, user_id },
            } => {
                assert_eq!(output, "dump.md");
                assert!(user_id.is_none());
            }
            _ => panic!("Expected Memory Export command"),
        }
    }

    #[test]
    fn test_cli_parse_memory_import() {
        let cli = Cli::try_parse_from([
            "openrustclaw",
            "memory",
            "import",
            "--file",
            "MEMORY.md",
            "--user-id",
            "user1",
        ])
        .unwrap();
        match cli.command {
            Commands::Memory {
                action: MemoryAction::Import { file, user_id },
            } => {
                assert_eq!(file, "MEMORY.md");
                assert_eq!(user_id, "user1");
            }
            _ => panic!("Expected Memory Import command"),
        }
    }

    #[test]
    fn test_cli_parse_schedule_list() {
        let cli = Cli::try_parse_from(["openrustclaw", "schedule", "list"]).unwrap();
        matches!(
            cli.command,
            Commands::Schedule {
                action: ScheduleAction::List
            }
        );
    }

    #[test]
    fn test_cli_parse_schedule_create() {
        let cli = Cli::try_parse_from([
            "openrustclaw",
            "schedule",
            "create",
            "--name",
            "daily-check",
            "--workflow",
            "health_check",
        ])
        .unwrap();
        match cli.command {
            Commands::Schedule {
                action:
                    ScheduleAction::Create {
                        name,
                        workflow,
                        description,
                        file,
                        every_seconds,
                        at,
                        payload,
                        priority,
                        owner,
                        tags,
                    },
            } => {
                assert_eq!(name.as_deref(), Some("daily-check"));
                assert_eq!(workflow.as_deref(), Some("health_check"));
                assert!(description.is_none());
                assert!(file.is_none());
                assert!(every_seconds.is_none());
                assert!(at.is_none());
                assert!(payload.is_none());
                assert_eq!(priority, 100);
                assert!(owner.is_none());
                assert!(tags.is_empty());
            }
            _ => panic!("Expected Schedule Create command"),
        }
    }

    #[test]
    fn test_cli_parse_optimize_register_target() {
        let cli = Cli::try_parse_from([
            "openrustclaw",
            "optimize",
            "register-target",
            "--name",
            "skills.instructions",
            "--kind",
            "skill",
            "--tier",
            "rust_native",
            "--allowed-path",
            "skills/",
            "--eval",
            "smoke=bash -lc 'echo ok'",
        ])
        .unwrap();
        match cli.command {
            Commands::Optimize {
                action:
                    OptimizeAction::RegisterTarget {
                        name,
                        kind,
                        tier,
                        allowed_paths,
                        evals,
                        ..
                    },
            } => {
                assert_eq!(name, "skills.instructions");
                assert_eq!(kind, "skill");
                assert_eq!(tier, "rust_native");
                assert_eq!(allowed_paths, vec!["skills/"]);
                assert_eq!(evals, vec!["smoke=bash -lc 'echo ok'"]);
            }
            _ => panic!("Expected Optimize RegisterTarget command"),
        }
    }

    #[test]
    fn test_cli_parse_optimize_submit_candidate() {
        let cli = Cli::try_parse_from([
            "openrustclaw",
            "optimize",
            "submit-candidate",
            "--target",
            "skills.instructions",
            "--hypothesis",
            "reduce noise",
            "--change-set",
            "changes.json",
        ])
        .unwrap();
        match cli.command {
            Commands::Optimize {
                action:
                    OptimizeAction::SubmitCandidate {
                        target,
                        hypothesis,
                        change_set,
                        ..
                    },
            } => {
                assert_eq!(target, "skills.instructions");
                assert_eq!(hypothesis, "reduce noise");
                assert_eq!(change_set, "changes.json");
            }
            _ => panic!("Expected Optimize SubmitCandidate command"),
        }
    }

    #[test]
    fn test_cli_parse_skills_list() {
        let cli = Cli::try_parse_from(["openrustclaw", "skills", "list"]).unwrap();
        matches!(
            cli.command,
            Commands::Skills {
                action: SkillsAction::List
            }
        );
    }

    #[test]
    fn test_cli_parse_skills_search() {
        let cli = Cli::try_parse_from(["openrustclaw", "skills", "search", "web"]).unwrap();
        match cli.command {
            Commands::Skills {
                action:
                    SkillsAction::Search {
                        query,
                        category,
                        sort,
                    },
            } => {
                assert_eq!(query, "web");
                assert!(category.is_none());
                assert_eq!(sort, "relevance");
            }
            _ => panic!("Expected Skills Search command"),
        }
    }

    #[test]
    fn test_cli_parse_skills_install() {
        let cli = Cli::try_parse_from(["openrustclaw", "skills", "install", "web_search"]).unwrap();
        match cli.command {
            Commands::Skills {
                action: SkillsAction::Install { name },
            } => {
                assert_eq!(name, "web_search");
            }
            _ => panic!("Expected Skills Install command"),
        }
    }

    #[test]
    fn test_cli_parse_webhooks_list() {
        let cli = Cli::try_parse_from(["openrustclaw", "webhooks", "list"]).unwrap();
        matches!(
            cli.command,
            Commands::Webhooks {
                action: WebhooksAction::List
            }
        );
    }

    #[test]
    fn test_cli_parse_webhooks_create() {
        let cli = Cli::try_parse_from(["openrustclaw", "webhooks", "create", "github"]).unwrap();
        match cli.command {
            Commands::Webhooks {
                action: WebhooksAction::Create { path },
            } => {
                assert_eq!(path, "github");
            }
            _ => panic!("Expected Webhooks Create command"),
        }
    }

    #[test]
    fn test_cli_parse_mcp_server_default() {
        let cli = Cli::try_parse_from(["openrustclaw", "mcp-server"]).unwrap();
        match cli.command {
            Commands::McpServer { transport, config } => {
                assert_eq!(transport, "stdio");
                assert_eq!(config, "config/default.toml");
            }
            _ => panic!("Expected McpServer command"),
        }
    }

    #[test]
    fn test_cli_parse_mcp_server_with_config() {
        let cli = Cli::try_parse_from([
            "openrustclaw",
            "mcp-server",
            "--transport",
            "stdio",
            "--config",
            "config/dev.toml",
        ])
        .unwrap();
        match cli.command {
            Commands::McpServer { transport, config } => {
                assert_eq!(transport, "stdio");
                assert_eq!(config, "config/dev.toml");
            }
            _ => panic!("Expected McpServer command"),
        }
    }

    #[test]
    fn test_cli_parse_invalid_command() {
        let cli = Cli::try_parse_from(["openrustclaw", "nonexistent"]);
        assert!(cli.is_err());
    }

    #[test]
    fn test_cli_parse_no_args() {
        let cli = Cli::try_parse_from(["openrustclaw"]);
        assert!(cli.is_err());
    }
}

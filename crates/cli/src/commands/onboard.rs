//! Interactive Onboarding Wizard

use anyhow::{Result, anyhow};
use chrono::Utc;
use console::style;
use dialoguer::{Confirm, Input, MultiSelect, Password, Select, theme::ColorfulTheme};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use tokio::fs;

use super::{channels, control, doctor, runtime, self_hosted, services};
use super::{chat, models};

/// Onboarding wizard state
#[derive(Default)]
pub struct OnboardingState {
    pub gateway_configured: bool,
    pub channels_configured: Vec<String>,
    pub model_configured: bool,
    pub preferred_provider: Option<String>,
    pub execution_mode: Option<String>,
    pub deployment_mode: Option<String>,
    pub deployment_path: Option<String>,
    pub remote_connectivity_profile: Option<RemoteConnectivityProfile>,
    pub daemon_installed: bool,
    pub skills_installed: Vec<String>,
    pub profile: Option<String>,
    pub workspace_action: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OnboardingWorkspaceStatus {
    pub workspace_root: String,
    pub env_present: bool,
    pub control_registry_present: bool,
    pub channels_registry_present: bool,
    pub product_mode_present: bool,
    pub product_mode_label: Option<String>,
    pub setup_state_present: bool,
    pub setup_status: Option<String>,
    pub setup_next_action: Option<String>,
    pub user_service_present: bool,
}

/// Interactive onboarding wizard
pub struct OnboardingWizard {
    theme: ColorfulTheme,
    state: OnboardingState,
}

#[derive(Clone, Copy)]
enum OnboardingProfile {
    Standard,
    Advanced,
    Custom,
}

impl OnboardingProfile {
    fn label(&self) -> &'static str {
        match self {
            OnboardingProfile::Standard => "Standard",
            OnboardingProfile::Advanced => "Advanced",
            OnboardingProfile::Custom => "Custom",
        }
    }

    fn from_label(label: &str) -> Self {
        match label {
            "Advanced" => OnboardingProfile::Advanced,
            "Custom" => OnboardingProfile::Custom,
            _ => OnboardingProfile::Standard,
        }
    }
}

/// Enum representing all onboarding steps
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OnboardingStep {
    Gateway,
    Channel,
    Model,
    ControlPlane,
    Skill,
    Daemon,
}

impl OnboardingStep {
    fn id(&self) -> &'static str {
        match self {
            OnboardingStep::Gateway => "gateway",
            OnboardingStep::Channel => "channel",
            OnboardingStep::Model => "model",
            OnboardingStep::ControlPlane => "control_plane",
            OnboardingStep::Skill => "skill",
            OnboardingStep::Daemon => "daemon",
        }
    }

    fn name(&self) -> &'static str {
        match self {
            OnboardingStep::Gateway => "Gateway Setup",
            OnboardingStep::Channel => "Channel Setup",
            OnboardingStep::Model => "AI Model Setup",
            OnboardingStep::ControlPlane => "Claw Runtime Mode",
            OnboardingStep::Skill => "Skills Setup",
            OnboardingStep::Daemon => "System Service",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            OnboardingStep::Gateway => "Configure the WebSocket gateway server",
            OnboardingStep::Channel => "Connect messaging platforms",
            OnboardingStep::Model => "Configure your LLM provider",
            OnboardingStep::ControlPlane => "Choose solo vs multi-claw execution",
            OnboardingStep::Skill => "Install starter skills",
            OnboardingStep::Daemon => "Install as system service (optional)",
        }
    }

    async fn run(&self, wizard: &mut OnboardingWizard) -> Result<bool> {
        match self {
            OnboardingStep::Gateway => run_gateway_setup(wizard).await,
            OnboardingStep::Channel => run_channel_setup(wizard).await,
            OnboardingStep::Model => run_model_setup(wizard).await,
            OnboardingStep::ControlPlane => run_control_plane_setup(wizard).await,
            OnboardingStep::Skill => run_skill_setup(wizard).await,
            OnboardingStep::Daemon => run_daemon_install(wizard).await,
        }
    }
}

const SETUP_STATE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStateManifest {
    #[serde(default = "default_setup_state_version")]
    pub version: u32,
    pub setup: SetupState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteConnectivityProfile {
    pub mode: String,
    pub primary_path: String,
    #[serde(default)]
    pub fallback_paths: Vec<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupState {
    pub started_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub completed_at: Option<String>,
    pub status: String,
    pub workspace_action: String,
    #[serde(default)]
    pub deployment_mode: Option<String>,
    #[serde(default)]
    pub deployment_path: Option<String>,
    #[serde(default)]
    pub remote_connectivity_profile: Option<RemoteConnectivityProfile>,
    #[serde(default)]
    pub setup_path: Option<String>,
    #[serde(default)]
    pub selected_steps: Vec<String>,
    #[serde(default)]
    pub completed_steps: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub next_action: Option<String>,
    #[serde(default)]
    pub current_step: Option<String>,
    #[serde(default)]
    pub bootstrap_outcomes: Vec<SetupBootstrapOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetupBootstrapOutcome {
    pub category: String,
    pub target: String,
    pub status: String,
    pub detail: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
struct SetupRepairPlan {
    steps: Vec<OnboardingStep>,
    reasons: Vec<String>,
}

fn default_setup_state_version() -> u32 {
    SETUP_STATE_VERSION
}

impl OnboardingWizard {
    /// Create a new onboarding wizard
    pub fn new() -> Self {
        Self {
            theme: ColorfulTheme::default(),
            state: OnboardingState::default(),
        }
    }

    /// Run the onboarding wizard
    pub async fn run(&mut self) -> Result<()> {
        self.print_welcome();
        let workspace_root = std::env::current_dir()?;

        let workspace_status = workspace_status(workspace_root.as_path());
        let existing_setup_state = load_setup_state(&workspace_root)?;
        let existing_setup_resumable = existing_setup_state
            .as_ref()
            .map(setup_state_is_resumable)
            .unwrap_or(false);

        if workspace_status.env_present
            || workspace_status.control_registry_present
            || workspace_status.channels_registry_present
            || workspace_status.product_mode_present
            || workspace_status.setup_state_present
        {
            let mut choices = Vec::new();
            let mut resume_choice = None;
            if existing_setup_resumable {
                let setup = &existing_setup_state
                    .as_ref()
                    .expect("checked resumable state")
                    .setup;
                let detail = setup
                    .current_step
                    .clone()
                    .or_else(|| setup.next_action.clone())
                    .unwrap_or_else(|| "continue previous setup".to_string());
                resume_choice = Some(choices.len());
                choices.push(format!("Resume previous setup ({detail})"));
            }
            let repair_choice = choices.len();
            choices.push("Repair setup blockers or workspace drift".to_string());
            let modify_choice = choices.len();
            choices.push("Modify existing workspace state".to_string());
            let health_choice = choices.len();
            choices.push("Keep existing state and only run a health check".to_string());
            let reset_choice = choices.len();
            choices.push("Reset onboarding-managed state with backup".to_string());
            let selection = Select::with_theme(&self.theme)
                .with_prompt("Existing OpenRustClaw workspace state was detected")
                .items(&choices)
                .default(resume_choice.unwrap_or(modify_choice))
                .interact()?;

            if Some(selection) == resume_choice {
                self.print_workspace_status(&workspace_status);
                self.load_from_setup_state(existing_setup_state.as_ref().expect("resume state"))?;
                let steps = selected_steps_from_setup_state(
                    &existing_setup_state.as_ref().expect("resume state").setup,
                );
                self.run_selected_steps(&workspace_root, steps).await?;
                let healthy = self.run_post_onboarding_health_check().await?;
                self.print_completion();
                self.maybe_launch_assistant(healthy).await?;
                return Ok(());
            }

            if selection == repair_choice {
                self.print_workspace_status(&workspace_status);
                if let Some(setup_state) = existing_setup_state.as_ref() {
                    self.load_from_setup_state(setup_state)?;
                } else {
                    self.load_from_workspace_profile(&workspace_root)?;
                }
                self.state.workspace_action = Some("repair_existing".to_string());
                let repair_plan =
                    build_setup_repair_plan(&workspace_root, existing_setup_state.as_ref()).await?;
                if repair_plan.steps.is_empty() {
                    println!(
                        "No targeted repair steps were identified. Running a health check only."
                    );
                    let healthy = self.run_post_onboarding_health_check().await?;
                    self.print_completion();
                    self.maybe_launch_assistant(healthy).await?;
                    return Ok(());
                }
                print_repair_plan(&repair_plan);
                let proceed = Confirm::with_theme(&self.theme)
                    .with_prompt("Run this targeted repair plan now?")
                    .default(true)
                    .interact()?;
                if proceed {
                    prepare_setup_state_for_repair(&workspace_root, &repair_plan.steps)?;
                    self.run_selected_steps(&workspace_root, repair_plan.steps)
                        .await?;
                }
                let healthy = self.run_post_onboarding_health_check().await?;
                self.print_completion();
                self.maybe_launch_assistant(healthy).await?;
                return Ok(());
            }

            match selection {
                value if value == health_choice => {
                    self.print_workspace_status(&workspace_status);
                    let healthy = self.run_post_onboarding_health_check().await?;
                    self.print_completion();
                    self.maybe_launch_assistant(healthy).await?;
                    return Ok(());
                }
                value if value == reset_choice => {
                    let backup_path = backup_and_reset_workspace_state().await?;
                    println!(
                        "✓ Existing onboarding-managed state backed up to {}",
                        backup_path.display()
                    );
                    self.state.workspace_action = Some("reset_with_backup".to_string());
                }
                _ => {
                    self.print_workspace_status(&workspace_status);
                    self.state.workspace_action = Some("modify_existing".to_string());
                }
            }
        }

        if self.state.workspace_action.is_none() {
            self.state.workspace_action = Some("new_workspace".to_string());
        }

        let deployment_mode = select_deployment_mode(&self.theme, &workspace_root)?;
        let descriptor = self_hosted::descriptor_for(deployment_mode.mode())?;
        self_hosted::configure_mode(
            &workspace_root,
            descriptor.mode,
            Some(descriptor.onboarding_path),
            Some("selected during onboarding"),
        )?;
        self.state.deployment_mode = Some(descriptor.mode.to_string());
        self.state.deployment_path = Some(descriptor.onboarding_path.to_string());

        println!(
            "\n{}",
            style(format!(
                "Selected deployment path: {} — {}",
                descriptor.label, descriptor.operator_model
            ))
            .bold()
            .cyan()
        );
        println!(
            "  This self-hosted open-source path recommends `{}` as the initial runtime shape.",
            descriptor.recommended_runtime_mode
        );

        let (profile, steps) = select_setup_path(
            &self.theme,
            descriptor.mode,
            self.state.deployment_mode.as_deref(),
        )?;
        self.state.profile = Some(profile.label().to_string());
        save_setup_state(
            &workspace_root,
            &SetupStateManifest {
                version: SETUP_STATE_VERSION,
                setup: SetupState {
                    started_at: Utc::now().to_rfc3339(),
                    updated_at: Utc::now().to_rfc3339(),
                    completed_at: None,
                    status: "in_progress".to_string(),
                    workspace_action: self
                        .state
                        .workspace_action
                        .clone()
                        .unwrap_or_else(|| "new_workspace".to_string()),
                    deployment_mode: self.state.deployment_mode.clone(),
                    deployment_path: self.state.deployment_path.clone(),
                    remote_connectivity_profile: self.state.remote_connectivity_profile.clone(),
                    setup_path: self.state.profile.clone(),
                    selected_steps: step_ids(&steps),
                    completed_steps: Vec::new(),
                    blockers: Vec::new(),
                    next_action: steps
                        .first()
                        .map(|step| format!("Complete {}", step.name())),
                    current_step: steps.first().map(|step| step.id().to_string()),
                    bootstrap_outcomes: Vec::new(),
                },
            },
        )?;

        self.run_selected_steps(&workspace_root, steps).await?;

        let healthy = self.run_post_onboarding_health_check().await?;
        self.print_completion();
        self.maybe_launch_assistant(healthy).await?;
        Ok(())
    }

    fn print_welcome(&self) {
        println!(
            r#"
🦀 Welcome to OpenRustClaw!

This wizard will guide you through setting up your AI agent.
We'll configure:
  - Gateway (WebSocket server)
  - Channels (Telegram, Discord, Slack, etc.)
  - LLM Provider (Claude, GPT, etc.)
  - Skills (tools and capabilities)
  - System service (optional)

Let's get started!
"#
        );
    }

    fn load_from_setup_state(&mut self, setup_state: &SetupStateManifest) -> Result<()> {
        self.state.deployment_mode = setup_state.setup.deployment_mode.clone();
        self.state.deployment_path = setup_state.setup.deployment_path.clone();
        self.state.remote_connectivity_profile =
            setup_state.setup.remote_connectivity_profile.clone();
        self.state.profile = setup_state.setup.setup_path.clone();
        self.state.workspace_action = Some(setup_state.setup.workspace_action.clone());
        Ok(())
    }

    fn load_from_workspace_profile(&mut self, workspace_root: &Path) -> Result<()> {
        if let Some(manifest) = self_hosted::load_manifest(workspace_root)? {
            self.state.deployment_mode = Some(manifest.profile.mode);
            self.state.deployment_path = Some(manifest.profile.onboarding_path);
        }
        Ok(())
    }

    async fn run_selected_steps(
        &mut self,
        workspace_root: &Path,
        steps: Vec<OnboardingStep>,
    ) -> Result<()> {
        for step in steps {
            set_setup_state_current_step(workspace_root, &step)?;
            println!("\n{}", style(format!("📋 {}", step.name())).bold().cyan());
            println!("{}", style(step.description()).dim());

            match step.run(self).await {
                Ok(true) => {
                    mark_setup_state_step_completed(workspace_root, &step)?;
                }
                Ok(false) => {
                    mark_setup_state_step_blocked(
                        workspace_root,
                        &step,
                        format!("{} was skipped or not completed", step.name()),
                    )?;
                    println!("⚠️  Step skipped or failed, but continuing...");
                }
                Err(e) => {
                    mark_setup_state_step_blocked(
                        workspace_root,
                        &step,
                        format!("{} failed: {}", step.name(), e),
                    )?;
                    println!("⚠️  Step error: {}", e);
                    println!("    Continuing...");
                }
            }
        }

        Ok(())
    }

    fn print_completion(&self) {
        println!("\n{}", style("✅ Onboarding complete!").bold().green());
        println!("\nConfiguration summary:");
        println!(
            "  Deployment Mode: {}",
            self.state
                .deployment_mode
                .as_deref()
                .unwrap_or("solo (default)")
        );
        println!(
            "  Deployment Path: {}",
            self.state
                .deployment_path
                .as_deref()
                .unwrap_or("solo_starter")
        );
        println!(
            "  Path: {}",
            self.state.profile.as_deref().unwrap_or("Standard")
        );
        println!(
            "  Gateway: {}",
            if self.state.gateway_configured {
                "✓ Configured"
            } else {
                "✗ Not configured"
            }
        );
        println!(
            "  Channels: {}",
            if self.state.channels_configured.is_empty() {
                "None".to_string()
            } else {
                self.state.channels_configured.join(", ")
            }
        );
        println!(
            "  Model: {}",
            if self.state.model_configured {
                "✓ Configured"
            } else {
                "✗ Not configured"
            }
        );
        println!(
            "  Runtime Mode: {}",
            self.state
                .execution_mode
                .as_deref()
                .unwrap_or("solo_claw (default)")
        );
        println!(
            "  Skills: {}",
            if self.state.skills_installed.is_empty() {
                "None".to_string()
            } else {
                format!("{} installed", self.state.skills_installed.len())
            }
        );
        println!(
            "  System Service: {}",
            if self.state.daemon_installed {
                "✓ Installed"
            } else {
                "✗ Not installed"
            }
        );
        if let Ok(workspace_root) = std::env::current_dir()
            && let Ok(Some(setup_state)) = load_setup_state(&workspace_root)
        {
            let handoff = setup_handoff_status(&setup_state.setup);
            println!(
                "\nSetup handoff: {}",
                handoff.replace('_', " ").to_ascii_uppercase()
            );
            println!("  {}", setup_handoff_detail(&setup_state.setup));
            if let Some(next_action) = setup_state.setup.next_action.as_deref() {
                println!("  Next action: {next_action}");
            }
            let pending = pending_step_names(&setup_state.setup);
            if !pending.is_empty() {
                println!("  Pending steps: {}", pending.join(", "));
            }
            if !setup_state.setup.blockers.is_empty() {
                println!("  Blockers:");
                for blocker in &setup_state.setup.blockers {
                    println!("    - {blocker}");
                }
            }
            let outcomes = non_ready_bootstrap_outcomes(&setup_state.setup);
            if !outcomes.is_empty() {
                println!("  Bootstrap attention:");
                for outcome in outcomes {
                    println!(
                        "    - {} {}: {}",
                        outcome.category, outcome.target, outcome.detail
                    );
                }
            }
        }
        println!("\nNext steps:");
        println!("  openrustclaw start    # Start the gateway");
        println!("  openrustclaw assistant # Start the persisted assistant session");
        println!("  openrustclaw doctor   # Verify everything works");
        println!("  Open http://127.0.0.1:18789/control/ui after startup for the dashboard");
    }

    fn print_workspace_status(&self, status: &OnboardingWorkspaceStatus) {
        println!("\nExisting workspace state:");
        println!(
            "  .env: {}",
            if status.env_present {
                "present"
            } else {
                "absent"
            }
        );
        println!(
            "  .claw/control: {}",
            if status.control_registry_present {
                "present"
            } else {
                "absent"
            }
        );
        println!(
            "  .claw/channels: {}",
            if status.channels_registry_present {
                "present"
            } else {
                "absent"
            }
        );
        println!(
            "  product mode: {}",
            status
                .product_mode_label
                .as_deref()
                .unwrap_or(if status.product_mode_present {
                    "present"
                } else {
                    "absent"
                })
        );
        println!(
            "  setup state: {}",
            status
                .setup_status
                .as_deref()
                .unwrap_or(if status.setup_state_present {
                    "present"
                } else {
                    "absent"
                })
        );
        if let Some(next_action) = status.setup_next_action.as_deref() {
            println!("  next action: {next_action}");
        }
        println!(
            "  user service: {}",
            if status.user_service_present {
                "present"
            } else {
                "absent"
            }
        );
    }

    async fn run_post_onboarding_health_check(&self) -> Result<bool> {
        println!(
            "\n{}",
            style("Running post-onboarding health check...").cyan()
        );
        let report = doctor::collect_report(false, true, None).await?;
        let readiness = doctor::first_start_readiness(&report);
        println!(
            "  Health check: {} passed, {} warnings, {} failed",
            report.passed, report.warnings, report.failed
        );
        if readiness.ready {
            println!("  ✓ The workspace is ready for first start.");
        } else {
            println!("  ⚠ First start is blocked until these items are fixed:");
            for item in &readiness.blocking_items {
                println!("    - {}", item);
            }
            println!("  ⚠ Review `openrustclaw doctor --deep` before first start.");
        }
        let workspace_root = std::env::current_dir()?;
        finalize_setup_state(&workspace_root, &readiness)?;
        Ok(readiness.ready)
    }

    fn assistant_provider(&self) -> Option<String> {
        if let Some(provider) = self.state.preferred_provider.clone() {
            return Some(provider);
        }

        let workspace_root = std::env::current_dir().ok()?;
        let config = runtime::load_effective_config("config/default.toml", &workspace_root).ok()?;
        let provider = config.providers.default_provider.trim();
        if provider.is_empty() {
            None
        } else {
            Some(provider.to_string())
        }
    }

    async fn maybe_launch_assistant(&self, healthy: bool) -> Result<()> {
        let provider = self.assistant_provider();
        if !should_offer_assistant_launch(
            healthy,
            std::io::stdin().is_terminal(),
            provider.as_deref(),
        ) {
            return Ok(());
        }
        let provider = provider.expect("launch gate requires a resolved provider");

        let launch = Confirm::with_theme(&self.theme)
            .with_prompt(format!(
                "Launch the persisted assistant session now with `{provider}`?"
            ))
            .default(true)
            .interact()?;

        if launch {
            println!();
            chat::run(&provider, None).await?;
        }

        Ok(())
    }
}

pub fn should_offer_assistant_launch(
    healthy: bool,
    stdin_is_terminal: bool,
    provider: Option<&str>,
) -> bool {
    healthy && stdin_is_terminal && provider.is_some()
}

// ============================================================================
// Step 1: Gateway Setup
// ============================================================================

async fn run_gateway_setup(wizard: &mut OnboardingWizard) -> Result<bool> {
    let gateway_modes = vec![
        "Local gateway on this machine",
        "Remote gateway/client guidance only",
    ];
    let mode = Select::with_theme(&wizard.theme)
        .with_prompt("Gateway mode")
        .items(&gateway_modes)
        .default(0)
        .interact()?;

    let connectivity_profile = if mode == 1 {
        println!("Remote gateway/client mode is not a separate shipped runtime yet.");
        println!(
            "Current remote-connectivity direction: prefer a node-first path, fall back to an SSH tunnel if needed, and only use a reverse proxy as a last resort you fully control."
        );

        let remote_profiles = vec![
            "Node-first with SSH tunnel fallback and reverse proxy last resort",
            "Node-first with SSH tunnel fallback only",
            "SSH tunnel fallback only for now",
            "Reverse proxy only as a last resort",
        ];
        let selection = Select::with_theme(&wizard.theme)
            .with_prompt("Remote connectivity profile")
            .items(&remote_profiles)
            .default(0)
            .interact()?;
        remote_connectivity_profile_for_selection(selection)
    } else {
        local_connectivity_profile()
    };
    wizard.state.remote_connectivity_profile = Some(connectivity_profile.clone());

    let host: String = Input::with_theme(&wizard.theme)
        .with_prompt("Gateway host")
        .default("127.0.0.1".to_string())
        .interact_text()?;

    let port: u16 = Input::with_theme(&wizard.theme)
        .with_prompt("Gateway port")
        .default(18789)
        .interact_text()?;

    // Generate JWT secret
    let jwt_secret = generate_jwt_secret();

    // Save to environment file
    save_gateway_config(&host, port, &jwt_secret).await?;

    wizard.state.gateway_configured = true;
    let workspace_root = std::env::current_dir()?;
    let (remote_status, remote_detail) = remote_connectivity_bootstrap_outcome(&connectivity_profile);
    record_bootstrap_outcome(
        &workspace_root,
        "remote_connectivity",
        "gateway_access",
        remote_status,
        remote_detail,
    )?;
    record_bootstrap_outcome(
        &workspace_root,
        "runtime",
        "gateway",
        "ready",
        format!("Gateway configured for {host}:{port}."),
    )?;
    println!("✓ Gateway configured on {}:{}", host, port);
    Ok(true)
}

fn local_connectivity_profile() -> RemoteConnectivityProfile {
    RemoteConnectivityProfile {
        mode: "local_only".to_string(),
        primary_path: "local_runtime".to_string(),
        fallback_paths: Vec::new(),
        detail: "Local runtime selected. No remote connectivity fallback is active for this workspace.".to_string(),
    }
}

fn remote_connectivity_profile_for_selection(selection: usize) -> RemoteConnectivityProfile {
    match selection {
        1 => RemoteConnectivityProfile {
            mode: "remote_access".to_string(),
            primary_path: "node_first".to_string(),
            fallback_paths: vec!["ssh_tunnel".to_string()],
            detail: "Remote access profile saved: prefer a node-first path and use an SSH tunnel if the node path is unavailable.".to_string(),
        },
        2 => RemoteConnectivityProfile {
            mode: "remote_access".to_string(),
            primary_path: "ssh_tunnel".to_string(),
            fallback_paths: Vec::new(),
            detail: "Remote access profile saved: use SSH tunnel as the temporary compatibility path until a node-first path is available.".to_string(),
        },
        3 => RemoteConnectivityProfile {
            mode: "remote_access".to_string(),
            primary_path: "reverse_proxy".to_string(),
            fallback_paths: Vec::new(),
            detail: "Remote access profile saved: reverse proxy is the chosen last-resort path and should only be used when you control the proxy boundary end to end.".to_string(),
        },
        _ => RemoteConnectivityProfile {
            mode: "remote_access".to_string(),
            primary_path: "node_first".to_string(),
            fallback_paths: vec![
                "ssh_tunnel".to_string(),
                "reverse_proxy".to_string(),
            ],
            detail: "Remote access profile saved: prefer a node-first path, use SSH tunnel as the first fallback, and keep reverse proxy as the last-resort fallback.".to_string(),
        },
    }
}

fn remote_connectivity_bootstrap_outcome(
    profile: &RemoteConnectivityProfile,
) -> (&'static str, String) {
    if profile.mode == "local_only" {
        (
            "ready",
            "Local runtime remains the active control path for this workspace.".to_string(),
        )
    } else {
        (
            "warning",
            format!(
                "{} Finish the host-level remote bootstrap manually, then verify the resulting path with `openrustclaw doctor` and the setup handoff surface.",
                profile.detail
            ),
        )
    }
}

// ============================================================================
// Step 2: Channel Setup
// ============================================================================

async fn run_channel_setup(wizard: &mut OnboardingWizard) -> Result<bool> {
    let channels = vec![
        "Telegram",
        "Discord",
        "Slack",
        "WhatsApp (advanced)",
        "Skip for now",
    ];

    let selection = Select::with_theme(&wizard.theme)
        .with_prompt("Which channel would you like to set up?")
        .items(&channels)
        .default(0)
        .interact()?;

    let platform = match selection {
        0 => Some(setup_telegram(wizard).await?),
        1 => Some(setup_discord(wizard).await?),
        2 => Some(setup_slack(wizard).await?),
        3 => {
            let workspace_root = std::env::current_dir()?;
            record_bootstrap_outcome(
                &workspace_root,
                "channel",
                "whatsapp",
                "warning",
                "WhatsApp bootstrap still needs manual setup. Review docs/whatsapp-setup.md before claiming channel readiness.",
            )?;
            return Err(anyhow!(
                "WhatsApp bootstrap is not automated yet. Review docs/whatsapp-setup.md and rerun onboarding after completing the manual steps."
            ));
        }
        _ => return Ok(true),
    };

    if let Some(platform) = platform {
        let workspace_root = std::env::current_dir()?;
        let assessment = validate_channel_bootstrap(&workspace_root, platform).await?;
        record_bootstrap_outcome(
            &workspace_root,
            "channel",
            platform,
            assessment.status,
            assessment.detail.clone(),
        )?;
        print_bootstrap_assessment(platform, &assessment);
        if assessment.blocking {
            return Err(anyhow!(assessment.detail));
        }
    }

    Ok(true)
}

async fn setup_telegram(wizard: &mut OnboardingWizard) -> Result<&'static str> {
    println!("\nTo set up Telegram:");
    println!("1. Message @BotFather on Telegram");
    println!("2. Create a new bot with /newbot");
    println!("3. Copy the bot token");

    let token: String = Password::with_theme(&wizard.theme)
        .with_prompt("Bot token")
        .interact()?;

    // Validate and save
    if !token.is_empty() {
        save_channel_config("telegram", &token).await?;
        wizard
            .state
            .channels_configured
            .push("telegram".to_string());
        println!("✓ Telegram configured");
    } else {
        println!("⚠️  No token provided, skipping Telegram setup");
        return Err(anyhow!(
            "Telegram setup was skipped because no token was provided."
        ));
    }

    Ok("telegram")
}

async fn setup_discord(wizard: &mut OnboardingWizard) -> Result<&'static str> {
    println!("\nTo set up Discord:");
    println!("1. Go to https://discord.com/developers/applications");
    println!("2. Create a New Application");
    println!("3. Go to Bot section and click 'Add Bot'");
    println!("4. Copy the bot token");

    let token: String = Password::with_theme(&wizard.theme)
        .with_prompt("Bot token")
        .interact()?;

    if !token.is_empty() {
        save_channel_config("discord", &token).await?;
        wizard.state.channels_configured.push("discord".to_string());
        println!("✓ Discord configured");
    } else {
        println!("⚠️  No token provided, skipping Discord setup");
        return Err(anyhow!(
            "Discord setup was skipped because no bot token was provided."
        ));
    }

    Ok("discord")
}

async fn setup_slack(wizard: &mut OnboardingWizard) -> Result<&'static str> {
    println!("\nTo set up Slack:");
    println!("1. Go to https://api.slack.com/apps");
    println!("2. Create a New App from scratch");
    println!("3. Go to OAuth & Permissions");
    println!("4. Install to workspace and copy the Bot User OAuth Token");

    let token: String = Password::with_theme(&wizard.theme)
        .with_prompt("Bot User OAuth Token")
        .interact()?;

    if !token.is_empty() {
        save_channel_config("slack", &token).await?;
        wizard.state.channels_configured.push("slack".to_string());
        println!("✓ Slack configured");
    } else {
        println!("⚠️  No token provided, skipping Slack setup");
        return Err(anyhow!(
            "Slack setup was skipped because no bot token was provided."
        ));
    }

    Ok("slack")
}

// ============================================================================
// Step 3: Model Setup
// ============================================================================

async fn run_model_setup(wizard: &mut OnboardingWizard) -> Result<bool> {
    let providers = vec![
        "Anthropic (Claude) - Recommended",
        "OpenAI (GPT-4)",
        "OpenRouter (Multiple models)",
        "Ollama (Local models)",
    ];

    let selection = Select::with_theme(&wizard.theme)
        .with_prompt("Choose your LLM provider")
        .items(&providers)
        .default(0)
        .interact()?;

    let (provider_name, api_key_prompt) = match selection {
        0 => ("anthropic", "Anthropic API key (starts with sk-ant-...)"),
        1 => ("openai", "OpenAI API key (starts with sk-...)"),
        2 => ("openrouter", "OpenRouter API key"),
        3 => {
            println!("Make sure Ollama is running locally (http://localhost:11434)");
            let workspace_root = std::env::current_dir()?;
            runtime::switch_provider(
                "config/default.toml",
                &workspace_root,
                "ollama",
                None,
                None,
                None,
            )?;
            wizard.state.model_configured = true;
            wizard.state.preferred_provider = Some("ollama".to_string());
            let provider_assessment =
                validate_provider_bootstrap(&workspace_root, "ollama").await?;
            record_bootstrap_outcome(
                &workspace_root,
                "provider",
                "ollama",
                provider_assessment.status,
                provider_assessment.detail.clone(),
            )?;
            print_bootstrap_assessment("ollama", &provider_assessment);
            let runtime_assessment =
                runtime_lane_assessment(&workspace_root, wizard.state.deployment_mode.as_deref())
                    .await?;
            record_bootstrap_outcome(
                &workspace_root,
                "runtime",
                "control_plane_lane",
                runtime_assessment.status,
                runtime_assessment.detail.clone(),
            )?;
            print_bootstrap_assessment("control-plane lane", &runtime_assessment);
            if provider_assessment.blocking {
                return Err(anyhow!(provider_assessment.detail));
            }
            println!("✓ Model configured (Ollama - local)");
            return Ok(true);
        }
        _ => return Ok(true),
    };

    let api_key = Password::with_theme(&wizard.theme)
        .with_prompt(api_key_prompt)
        .interact()?;

    if !api_key.is_empty() {
        save_provider_config(provider_name, &api_key).await?;
        let workspace_root = std::env::current_dir()?;
        runtime::switch_provider(
            "config/default.toml",
            &workspace_root,
            provider_name,
            None,
            None,
            None,
        )?;
        wizard.state.model_configured = true;
        wizard.state.preferred_provider = Some(provider_name.to_string());
        let provider_assessment =
            validate_provider_bootstrap(&workspace_root, provider_name).await?;
        record_bootstrap_outcome(
            &workspace_root,
            "provider",
            provider_name,
            provider_assessment.status,
            provider_assessment.detail.clone(),
        )?;
        print_bootstrap_assessment(provider_name, &provider_assessment);
        let runtime_assessment =
            runtime_lane_assessment(&workspace_root, wizard.state.deployment_mode.as_deref())
                .await?;
        record_bootstrap_outcome(
            &workspace_root,
            "runtime",
            "control_plane_lane",
            runtime_assessment.status,
            runtime_assessment.detail.clone(),
        )?;
        print_bootstrap_assessment("control-plane lane", &runtime_assessment);
        if provider_assessment.blocking {
            return Err(anyhow!(provider_assessment.detail));
        }
        println!("✓ Model configured ({provider_name})");
        println!(
            "  Control-plane actions will prefer the dedicated fallback lane in config/default.toml"
        );
        println!();
        models::scan().await?;
    } else {
        println!("⚠️  No API key provided, skipping model setup");
    }

    Ok(true)
}

// ============================================================================
// Step 4: Control-Plane Setup
// ============================================================================

async fn run_control_plane_setup(wizard: &mut OnboardingWizard) -> Result<bool> {
    control::init(None)?;

    let modes = vec![
        "Solo Claw - one Claw handles all work",
        "Task Assigned - specific tasks bind to specific Claws",
        "Category Assigned - task categories map to Claws",
        "Orchestrated - quarterback Claw delegates to worker Claws",
    ];
    let selection = Select::with_theme(&wizard.theme)
        .with_prompt("Choose how work should be assigned")
        .items(&modes)
        .default(default_runtime_mode_index(
            wizard.state.deployment_mode.as_deref(),
        ))
        .interact()?;

    let (mode, orchestrator) = match selection {
        1 => ("task_assigned", None),
        2 => ("category_assigned", None),
        3 => ("orchestrated", Some("orchestrator")),
        _ => ("solo_claw", None),
    };

    let allow_shared_context = if mode == "solo_claw" {
        false
    } else {
        Confirm::with_theme(&wizard.theme)
            .with_prompt("Allow shared context between Claws when explicitly configured?")
            .default(false)
            .interact()?
    };

    control::configure_mode(
        None,
        mode,
        Some("main"),
        orchestrator,
        allow_shared_context,
        Some("strict"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    wizard.state.execution_mode = Some(mode.to_string());
    let workspace_root = std::env::current_dir()?;
    let assessment = validate_execution_mode_bootstrap(
        &workspace_root,
        mode,
        wizard.state.deployment_mode.as_deref(),
    )?;
    record_bootstrap_outcome(
        &workspace_root,
        "runtime",
        "execution_mode",
        assessment.status,
        assessment.detail.clone(),
    )?;
    print_bootstrap_assessment("runtime mode", &assessment);
    println!("✓ Control-plane registry initialized at .claw/control/");
    println!("✓ Runtime mode configured as {mode}");
    println!("  Inspect: openrustclaw control describe");
    if assessment.blocking {
        return Err(anyhow!(assessment.detail));
    }
    Ok(true)
}

// ============================================================================
// Step 5: Skill Setup
// ============================================================================

async fn run_skill_setup(wizard: &mut OnboardingWizard) -> Result<bool> {
    let skills = vec![
        "memory (3-tier memory system) - Recommended",
        "browser (web automation)",
        "scheduler (job scheduling)",
    ];

    let selections = MultiSelect::with_theme(&wizard.theme)
        .with_prompt("Select skills to install (Space to select, Enter to confirm)")
        .items(&skills)
        .defaults(&[true, false, false])
        .interact()?;

    if selections.is_empty() {
        println!("No skills selected, skipping...");
        return Ok(true);
    }

    // Show progress
    let pb = ProgressBar::new(selections.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")?
            .progress_chars("#>-"),
    );

    for &idx in &selections {
        let skill_name = skills[idx].split(' ').next().unwrap_or("");
        pb.set_message(format!("Installing {}...", skill_name));

        // Simulate installation (in real implementation, call skills::install)
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        wizard.state.skills_installed.push(skills[idx].to_string());
        pb.inc(1);
    }

    pb.finish_with_message("Done!");
    println!("✓ {} skills installed", wizard.state.skills_installed.len());
    Ok(true)
}

// ============================================================================
// Step 6: Daemon Install
// ============================================================================

async fn run_daemon_install(wizard: &mut OnboardingWizard) -> Result<bool> {
    let workspace_root = std::env::current_dir()?;
    let status = runtime::runtime_service_install_status("config/default.toml", &workspace_root)?;
    if !status.supported {
        println!("No supported user service manager detected, skipping service setup.");
        return Ok(true);
    }

    let install = Confirm::with_theme(&wizard.theme)
        .with_prompt(format!(
            "Install OpenRustClaw as a {} service?",
            status.service_manager
        ))
        .default(true)
        .interact()?;

    if install {
        install_daemon().await?;
        wizard.state.daemon_installed = true;
        println!("✓ User service installed");
        if let Some(start_command) = status.start_command.as_deref() {
            println!("  Start: {start_command}");
        }
        if let Some(stop_command) = status.stop_command.as_deref() {
            println!("  Stop:  {stop_command}");
        }
        if let Some(enable_command) = status.enable_command.as_deref() {
            println!("  Enable: {enable_command}");
        }
    }

    Ok(true)
}

// ============================================================================
// Helper Functions
// ============================================================================

fn generate_jwt_secret() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.r#gen::<u8>()).collect();
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes)
}

pub fn setup_state_path(workspace_root: &Path) -> PathBuf {
    control::control_root_for(workspace_root).join("setup-state.json")
}

pub fn load_setup_state(workspace_root: &Path) -> Result<Option<SetupStateManifest>> {
    let path = setup_state_path(workspace_root);
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path)?;
    let state = serde_json::from_str(&raw)?;
    Ok(Some(state))
}

pub fn save_setup_state(workspace_root: &Path, manifest: &SetupStateManifest) -> Result<()> {
    let path = setup_state_path(workspace_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(manifest)?)?;
    Ok(())
}

fn setup_state_is_resumable(manifest: &SetupStateManifest) -> bool {
    matches!(manifest.setup.status.as_str(), "in_progress" | "blocked")
}

fn step_ids(steps: &[OnboardingStep]) -> Vec<String> {
    steps.iter().map(|step| step.id().to_string()).collect()
}

fn step_from_id(id: &str) -> Option<OnboardingStep> {
    match id {
        "gateway" => Some(OnboardingStep::Gateway),
        "channel" => Some(OnboardingStep::Channel),
        "model" => Some(OnboardingStep::Model),
        "control_plane" => Some(OnboardingStep::ControlPlane),
        "skill" => Some(OnboardingStep::Skill),
        "daemon" => Some(OnboardingStep::Daemon),
        _ => None,
    }
}

fn selected_steps_from_setup_state(setup: &SetupState) -> Vec<OnboardingStep> {
    let selected = setup
        .selected_steps
        .iter()
        .filter(|id| !setup.completed_steps.contains(*id))
        .filter_map(|id| step_from_id(id))
        .collect::<Vec<_>>();
    if selected.is_empty() {
        steps_for_profile(OnboardingProfile::from_label(
            setup.setup_path.as_deref().unwrap_or("Standard"),
        ))
    } else {
        selected
    }
}

pub fn pending_step_names(setup: &SetupState) -> Vec<String> {
    setup
        .selected_steps
        .iter()
        .filter(|id| !setup.completed_steps.contains(*id))
        .filter_map(|id| step_from_id(id))
        .map(|step| step.name().to_string())
        .collect()
}

pub fn non_ready_bootstrap_outcomes(setup: &SetupState) -> Vec<SetupBootstrapOutcome> {
    setup
        .bootstrap_outcomes
        .iter()
        .filter(|outcome| outcome.status != "ready")
        .cloned()
        .collect()
}

pub fn setup_handoff_status(setup: &SetupState) -> &'static str {
    if setup.status == "blocked"
        || setup
            .bootstrap_outcomes
            .iter()
            .any(|outcome| outcome.status == "blocked")
    {
        "blocked"
    } else if setup
        .bootstrap_outcomes
        .iter()
        .any(|outcome| outcome.status == "warning")
    {
        "degraded"
    } else if matches!(setup.status.as_str(), "ready" | "completed") {
        "ready"
    } else if setup.status == "in_progress" {
        "in_progress"
    } else {
        "unknown"
    }
}

pub fn setup_handoff_detail(setup: &SetupState) -> String {
    match setup_handoff_status(setup) {
        "blocked" => setup
            .blockers
            .first()
            .cloned()
            .or_else(|| setup.next_action.clone())
            .unwrap_or_else(|| "Setup is blocked and needs operator attention.".to_string()),
        "degraded" => non_ready_bootstrap_outcomes(setup)
            .first()
            .map(|outcome| {
                format!(
                    "{} {} needs review: {}",
                    outcome.category, outcome.target, outcome.detail
                )
            })
            .or_else(|| setup.next_action.clone())
            .unwrap_or_else(|| {
                "Setup is usable but still has warning-level bootstrap issues.".to_string()
            }),
        "ready" => "The workspace is ready for first start and assistant handoff.".to_string(),
        "in_progress" => setup
            .next_action
            .clone()
            .unwrap_or_else(|| "Setup is still in progress.".to_string()),
        _ => "Setup state exists but does not yet map to a known handoff status.".to_string(),
    }
}

fn with_setup_state_mut<F>(workspace_root: &Path, mutator: F) -> Result<()>
where
    F: FnOnce(&mut SetupState),
{
    let mut manifest = load_setup_state(workspace_root)?.unwrap_or(SetupStateManifest {
        version: SETUP_STATE_VERSION,
        setup: SetupState {
            started_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            completed_at: None,
            status: "in_progress".to_string(),
            workspace_action: "new_workspace".to_string(),
            deployment_mode: None,
            deployment_path: None,
            remote_connectivity_profile: None,
            setup_path: None,
            selected_steps: Vec::new(),
            completed_steps: Vec::new(),
            blockers: Vec::new(),
            next_action: None,
            current_step: None,
            bootstrap_outcomes: Vec::new(),
        },
    });
    mutator(&mut manifest.setup);
    manifest.setup.updated_at = Utc::now().to_rfc3339();
    save_setup_state(workspace_root, &manifest)
}

fn next_pending_step_name(setup: &SetupState) -> Option<String> {
    setup
        .selected_steps
        .iter()
        .find(|id| !setup.completed_steps.contains(*id))
        .and_then(|id| step_from_id(id))
        .map(|step| step.name().to_string())
}

fn set_setup_state_current_step(workspace_root: &Path, step: &OnboardingStep) -> Result<()> {
    with_setup_state_mut(workspace_root, |setup| {
        setup.status = "in_progress".to_string();
        setup.current_step = Some(step.id().to_string());
        setup.next_action = Some(format!("Complete {}", step.name()));
    })
}

fn mark_setup_state_step_completed(workspace_root: &Path, step: &OnboardingStep) -> Result<()> {
    with_setup_state_mut(workspace_root, |setup| {
        let step_id = step.id().to_string();
        if !setup.completed_steps.contains(&step_id) {
            setup.completed_steps.push(step_id);
        }
        setup
            .blockers
            .retain(|item| !item.contains(step.name()) && !item.contains(step.id()));
        setup.current_step = None;
        setup.next_action = next_pending_step_name(setup).map(|name| format!("Complete {name}"));
    })
}

fn mark_setup_state_step_blocked(
    workspace_root: &Path,
    step: &OnboardingStep,
    message: String,
) -> Result<()> {
    with_setup_state_mut(workspace_root, |setup| {
        setup.status = "blocked".to_string();
        setup.current_step = Some(step.id().to_string());
        if !setup.blockers.contains(&message) {
            setup.blockers.push(message.clone());
        }
        setup.next_action = Some(format!("Review {} and rerun onboarding", step.name()));
    })
}

fn finalize_setup_state(
    workspace_root: &Path,
    readiness: &doctor::FirstStartReadiness,
) -> Result<()> {
    with_setup_state_mut(workspace_root, |setup| {
        setup.completed_at = Some(Utc::now().to_rfc3339());
        setup.current_step = None;
        if readiness.ready {
            setup.status = "ready".to_string();
            setup.blockers.clear();
            setup.next_action =
                Some("Start the gateway or launch the persisted assistant session.".to_string());
        } else {
            setup.status = "blocked".to_string();
            setup.blockers = readiness.blocking_items.clone();
            setup.next_action = Some(
                "Run `openrustclaw doctor --deep`, fix blockers, then rerun onboarding."
                    .to_string(),
            );
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BootstrapAssessment {
    status: &'static str,
    detail: String,
    blocking: bool,
}

fn print_bootstrap_assessment(target: &str, assessment: &BootstrapAssessment) {
    let label = match assessment.status {
        "ready" => "✓",
        "warning" => "⚠",
        _ => "✗",
    };
    println!("  {label} {} bootstrap: {}", target, assessment.detail);
}

fn record_bootstrap_outcome(
    workspace_root: &Path,
    category: &str,
    target: &str,
    status: &str,
    detail: impl Into<String>,
) -> Result<()> {
    let detail = detail.into();
    with_setup_state_mut(workspace_root, |setup| {
        let outcome = SetupBootstrapOutcome {
            category: category.to_string(),
            target: target.to_string(),
            status: status.to_string(),
            detail,
            updated_at: Utc::now().to_rfc3339(),
        };
        if let Some(existing) = setup
            .bootstrap_outcomes
            .iter_mut()
            .find(|existing| existing.category == category && existing.target == target)
        {
            *existing = outcome;
        } else {
            setup.bootstrap_outcomes.push(outcome);
        }
    })
}

#[cfg(test)]
fn bootstrap_outcome_for<'a>(
    setup: &'a SetupState,
    category: &str,
    target: &str,
) -> Option<&'a SetupBootstrapOutcome> {
    setup
        .bootstrap_outcomes
        .iter()
        .find(|outcome| outcome.category == category && outcome.target == target)
}

async fn validate_provider_bootstrap(
    workspace_root: &Path,
    provider_name: &str,
) -> Result<BootstrapAssessment> {
    let report =
        runtime::runtime_health_status("config/default.toml", workspace_root, true).await?;
    let Some(entry) = report
        .providers
        .iter()
        .find(|entry| entry.provider == provider_name)
    else {
        return Ok(BootstrapAssessment {
            status: "blocked",
            detail: format!(
                "Runtime health did not return a `{provider_name}` provider entry after setup."
            ),
            blocking: true,
        });
    };

    if entry.healthy {
        Ok(BootstrapAssessment {
            status: "ready",
            detail: format!(
                "Provider `{provider_name}` is reachable with model `{}`.",
                entry.model
            ),
            blocking: false,
        })
    } else {
        Ok(BootstrapAssessment {
            status: "blocked",
            detail: format!(
                "Provider `{provider_name}` is configured but not ready: {}",
                entry
                    .issue
                    .clone()
                    .unwrap_or_else(|| "health probe failed".to_string())
            ),
            blocking: true,
        })
    }
}

async fn runtime_lane_assessment(
    workspace_root: &Path,
    deployment_mode: Option<&str>,
) -> Result<BootstrapAssessment> {
    let report =
        runtime::runtime_health_status("config/default.toml", workspace_root, true).await?;
    if report.degraded_control_plane_mode {
        return Ok(BootstrapAssessment {
            status: "warning",
            detail: report
                .failover_recommendations
                .first()
                .cloned()
                .or_else(|| report.operator_warnings.first().cloned())
                .unwrap_or_else(|| {
                    "No healthy control-plane fallback is available yet for the current runtime lane.".to_string()
                }),
            blocking: false,
        });
    }

    let mode_label = deployment_mode.unwrap_or(self_hosted::MODE_SOLO);
    Ok(BootstrapAssessment {
        status: "ready",
        detail: format!(
            "Runtime provider lane is healthy for the current `{mode_label}` deployment path."
        ),
        blocking: false,
    })
}

fn validate_execution_mode_bootstrap(
    workspace_root: &Path,
    execution_mode: &str,
    deployment_mode: Option<&str>,
) -> Result<BootstrapAssessment> {
    let registry = control::load_registry(control::control_root_for(workspace_root))?;
    let Some(runtime_spec) = registry.runtime else {
        return Ok(BootstrapAssessment {
            status: "blocked",
            detail: "Control-plane runtime registry was not written during setup.".to_string(),
            blocking: true,
        });
    };

    let deployment_mode = deployment_mode.unwrap_or(self_hosted::MODE_SOLO);
    let descriptor = self_hosted::descriptor_for(deployment_mode)?;
    if runtime_spec.mode == descriptor.recommended_runtime_mode {
        Ok(BootstrapAssessment {
            status: "ready",
            detail: format!(
                "Execution mode `{}` matches the recommended runtime shape for the {} deployment path.",
                runtime_spec.mode, descriptor.label
            ),
            blocking: false,
        })
    } else if runtime_spec.mode == execution_mode {
        Ok(BootstrapAssessment {
            status: "warning",
            detail: format!(
                "Execution mode `{}` was saved successfully, but the {} deployment path recommends `{}` as the initial runtime shape.",
                runtime_spec.mode, descriptor.label, descriptor.recommended_runtime_mode
            ),
            blocking: false,
        })
    } else {
        Ok(BootstrapAssessment {
            status: "blocked",
            detail: format!(
                "Control-plane runtime registry saved `{}` instead of the selected `{execution_mode}` mode.",
                runtime_spec.mode
            ),
            blocking: true,
        })
    }
}

async fn validate_channel_bootstrap(
    workspace_root: &Path,
    platform: &str,
) -> Result<BootstrapAssessment> {
    let report =
        services::channel_probes_status("config/default.toml", workspace_root, true).await?;
    let Some(entry) = report
        .entries
        .iter()
        .find(|entry| entry.platform == platform)
    else {
        return Ok(BootstrapAssessment {
            status: "blocked",
            detail: format!("No `{platform}` channel probe result was produced after setup."),
            blocking: true,
        });
    };

    match entry.status {
        services::ChannelProbeStatus::Ready => Ok(BootstrapAssessment {
            status: "ready",
            detail: entry.detail.clone(),
            blocking: false,
        }),
        services::ChannelProbeStatus::Warning => Ok(BootstrapAssessment {
            status: "warning",
            detail: entry.detail.clone(),
            blocking: true,
        }),
        services::ChannelProbeStatus::Failed => Ok(BootstrapAssessment {
            status: "blocked",
            detail: entry.detail.clone(),
            blocking: true,
        }),
    }
}

fn print_repair_plan(plan: &SetupRepairPlan) {
    println!("\n{}", style("Proposed repair plan:").bold().cyan());
    for step in &plan.steps {
        println!("  - {}", step.name());
    }
    if !plan.reasons.is_empty() {
        println!("  Reasons:");
        for reason in &plan.reasons {
            println!("    - {}", reason);
        }
    }
}

async fn build_setup_repair_plan(
    _workspace_root: &Path,
    setup_state: Option<&SetupStateManifest>,
) -> Result<SetupRepairPlan> {
    let report = doctor::collect_report(false, true, None).await?;
    Ok(derive_setup_repair_plan(
        setup_state.map(|manifest| &manifest.setup),
        &report,
    ))
}

fn derive_setup_repair_plan(
    setup_state: Option<&SetupState>,
    report: &doctor::DiagnosticReport,
) -> SetupRepairPlan {
    let mut steps = Vec::new();
    let mut reasons = Vec::new();

    if let Some(setup) = setup_state {
        let unfinished_steps = selected_steps_from_setup_state(setup);
        for step in &unfinished_steps {
            add_repair_step(&mut steps, *step);
        }
        if !unfinished_steps.is_empty() {
            reasons.push("Durable setup state still has unfinished steps.".to_string());
        }

        for outcome in &setup.bootstrap_outcomes {
            if outcome.status == "ready" {
                continue;
            }
            if let Some(step) = repair_step_for_bootstrap_outcome(outcome) {
                add_repair_step(&mut steps, step);
                reasons.push(format!(
                    "{} bootstrap is {}: {}",
                    outcome.target, outcome.status, outcome.detail
                ));
            }
        }
    }

    for check in &report.checks {
        if !diagnostic_check_requires_repair(check) {
            continue;
        }
        if let Some(step) = repair_step_for_diagnostic_check(check) {
            add_repair_step(&mut steps, step);
            reasons.push(format!(
                "{} requires repair: {}",
                check.label,
                check
                    .message
                    .clone()
                    .unwrap_or_else(|| "diagnostic reported a blocker".to_string())
            ));
        }
    }

    SetupRepairPlan { steps, reasons }
}

fn diagnostic_check_requires_repair(check: &doctor::DiagnosticCheck) -> bool {
    match check.id.as_str() {
        _ if check.status == doctor::DiagnosticStatus::Failed => true,
        "api_keys" | "control_registry" | "onboarding_state" => {
            check.status != doctor::DiagnosticStatus::Ok
        }
        "channel_readiness" => {
            if check.status == doctor::DiagnosticStatus::Ok {
                false
            } else {
                let message = check.message.as_deref().unwrap_or_default();
                !message.contains("No shipped channels are enabled")
            }
        }
        _ => false,
    }
}

fn repair_step_for_bootstrap_outcome(outcome: &SetupBootstrapOutcome) -> Option<OnboardingStep> {
    match (outcome.category.as_str(), outcome.target.as_str()) {
        ("runtime", "gateway") => Some(OnboardingStep::Gateway),
        ("provider", _) => Some(OnboardingStep::Model),
        ("runtime", "control_plane_lane") => Some(OnboardingStep::Model),
        ("runtime", "execution_mode") => Some(OnboardingStep::ControlPlane),
        ("channel", _) => Some(OnboardingStep::Channel),
        _ => None,
    }
}

fn repair_step_for_diagnostic_check(check: &doctor::DiagnosticCheck) -> Option<OnboardingStep> {
    match check.id.as_str() {
        "api_keys" => Some(OnboardingStep::Model),
        "channel_readiness" => Some(OnboardingStep::Channel),
        "control_registry" => Some(OnboardingStep::ControlPlane),
        "onboarding_state" => Some(OnboardingStep::Model),
        _ => None,
    }
}

fn add_repair_step(steps: &mut Vec<OnboardingStep>, step: OnboardingStep) {
    if !steps.contains(&step) {
        steps.push(step);
    }
}

fn prepare_setup_state_for_repair(workspace_root: &Path, steps: &[OnboardingStep]) -> Result<()> {
    let selected_step_ids = step_ids(steps);
    let product_mode = self_hosted::load_manifest(workspace_root)?;
    with_setup_state_mut(workspace_root, move |setup| {
        setup.workspace_action = "repair_existing".to_string();
        setup.status = "in_progress".to_string();
        setup.completed_at = None;
        setup.selected_steps = selected_step_ids.clone();
        setup
            .completed_steps
            .retain(|id| !selected_step_ids.contains(id));
        setup.blockers.clear();
        if setup.deployment_mode.is_none() {
            setup.deployment_mode = product_mode
                .as_ref()
                .map(|manifest| manifest.profile.mode.clone());
        }
        if setup.deployment_path.is_none() {
            setup.deployment_path = product_mode
                .as_ref()
                .map(|manifest| manifest.profile.onboarding_path.clone());
        }
        setup.current_step = steps.first().map(|step| step.id().to_string());
        setup.next_action = steps
            .first()
            .map(|step| format!("Complete {}", step.name()));
    })
}

async fn save_gateway_config(host: &str, port: u16, jwt_secret: &str) -> Result<()> {
    let env_content = format!(
        r#"# OpenRustClaw Configuration - Generated by onboarding wizard
OPENRUSTCLAW_GATEWAY__HOST={host}
OPENRUSTCLAW_GATEWAY__PORT={port}
OPENRUSTCLAW_SECURITY__JWT_SECRET={jwt_secret}
"#
    );

    fs::write(".env", env_content).await?;
    println!("  Configuration saved to .env");
    Ok(())
}

async fn save_channel_config(channel: &str, token: &str) -> Result<()> {
    let env_var = match channel {
        "telegram" => "OPENRUSTCLAW_CHANNELS__TELEGRAM__TOKEN",
        "discord" => "OPENRUSTCLAW_CHANNELS__DISCORD__TOKEN",
        "slack" => "OPENRUSTCLAW_CHANNELS__SLACK__TOKEN",
        _ => return Ok(()),
    };

    let env_line = format!("{}={}\n", env_var, token);
    let enable_line = format!(
        "OPENRUSTCLAW_CHANNELS__{}__ENABLED=true\n",
        channel.to_uppercase()
    );

    // Append to .env file
    let existing = fs::read_to_string(".env").await.unwrap_or_default();
    let mut lines: Vec<&str> = existing.lines().collect();

    // Remove existing lines for this channel
    lines.retain(|line| !line.starts_with(&format!("{}=", env_var)));
    lines.retain(|line| !line.starts_with(&enable_line[..enable_line.find('=').unwrap_or(0)]));

    let new_content = format!("{}{}{}", lines.join("\n"), env_line, enable_line);
    fs::write(".env", new_content.trim()).await?;

    println!("  Configuration saved to .env");
    Ok(())
}

async fn save_provider_config(provider: &str, api_key: &str) -> Result<()> {
    let env_var = match provider {
        "anthropic" => "ANTHROPIC_API_KEY",
        "openai" => "OPENAI_API_KEY",
        "openrouter" => "OPENROUTER_API_KEY",
        _ => return Ok(()),
    };

    let env_line = format!("{}={}\n", env_var, api_key);

    // Append to .env file
    let existing = fs::read_to_string(".env").await.unwrap_or_default();
    let mut lines: Vec<&str> = existing.lines().collect();

    // Remove existing API key for this provider
    lines.retain(|line| !line.starts_with(&format!("{}=", env_var)));

    let new_content = format!("{}{}", lines.join("\n"), env_line);
    fs::write(".env", new_content.trim()).await?;

    println!("  API key saved to .env");
    Ok(())
}

async fn install_daemon() -> Result<()> {
    let workspace_root = std::env::current_dir()?;
    let status = runtime::install_runtime_user_service("config/default.toml", &workspace_root)?;
    if let Some(service_path) = status.service_path {
        println!("  Service file written to {}", service_path);
    }
    if let Some(reload_command) = status.daemon_reload_command {
        println!("  Reload: {}", reload_command);
    }

    Ok(())
}

pub fn workspace_status(workspace_root: &Path) -> OnboardingWorkspaceStatus {
    let control_root = control::control_root_for(workspace_root);
    let channels_root = channels::channels_root_for(workspace_root);
    let product_mode = self_hosted::load_manifest(workspace_root).ok().flatten();
    let product_mode_label = product_mode
        .as_ref()
        .and_then(|manifest| self_hosted::descriptor_for(&manifest.profile.mode).ok())
        .map(|descriptor| descriptor.label.to_string());
    let service_present =
        runtime::runtime_service_install_status("config/default.toml", workspace_root)
            .map(|status| status.installed)
            .unwrap_or(false);
    let setup_state = load_setup_state(workspace_root).ok().flatten();

    OnboardingWorkspaceStatus {
        workspace_root: workspace_root.display().to_string(),
        env_present: workspace_root.join(".env").exists(),
        control_registry_present: control_root.exists(),
        channels_registry_present: channels_root.exists(),
        product_mode_present: product_mode.is_some(),
        product_mode_label,
        setup_state_present: setup_state.is_some(),
        setup_status: setup_state.as_ref().map(|state| state.setup.status.clone()),
        setup_next_action: setup_state.and_then(|state| state.setup.next_action),
        user_service_present: service_present,
    }
}

#[derive(Clone, Copy)]
enum DeploymentModeChoice {
    Solo,
    Team,
    Company,
    Enterprise,
}

impl DeploymentModeChoice {
    fn mode(self) -> &'static str {
        match self {
            DeploymentModeChoice::Solo => self_hosted::MODE_SOLO,
            DeploymentModeChoice::Team => self_hosted::MODE_TEAM,
            DeploymentModeChoice::Company => self_hosted::MODE_COMPANY,
            DeploymentModeChoice::Enterprise => self_hosted::MODE_ENTERPRISE,
        }
    }

    fn items() -> [DeploymentModeChoice; 4] {
        [
            DeploymentModeChoice::Solo,
            DeploymentModeChoice::Team,
            DeploymentModeChoice::Company,
            DeploymentModeChoice::Enterprise,
        ]
    }

    fn label(self) -> String {
        let descriptor =
            self_hosted::descriptor_for(self.mode()).expect("supported deployment mode");
        format!("{} - {}", descriptor.label, descriptor.operator_model)
    }

    fn index(self) -> usize {
        match self {
            DeploymentModeChoice::Solo => 0,
            DeploymentModeChoice::Team => 1,
            DeploymentModeChoice::Company => 2,
            DeploymentModeChoice::Enterprise => 3,
        }
    }

    fn from_mode(mode: &str) -> Self {
        match mode {
            self_hosted::MODE_TEAM => DeploymentModeChoice::Team,
            self_hosted::MODE_COMPANY => DeploymentModeChoice::Company,
            self_hosted::MODE_ENTERPRISE => DeploymentModeChoice::Enterprise,
            _ => DeploymentModeChoice::Solo,
        }
    }
}

impl OnboardingProfile {
    fn index(self) -> usize {
        match self {
            OnboardingProfile::Standard => 0,
            OnboardingProfile::Advanced => 1,
            OnboardingProfile::Custom => 2,
        }
    }
}

fn default_onboarding_profile_for_mode(mode: &str) -> OnboardingProfile {
    match mode {
        self_hosted::MODE_COMPANY | self_hosted::MODE_ENTERPRISE => OnboardingProfile::Advanced,
        _ => OnboardingProfile::Standard,
    }
}

fn default_runtime_mode_index(mode: Option<&str>) -> usize {
    match mode.unwrap_or(self_hosted::MODE_SOLO) {
        self_hosted::MODE_TEAM => 1,
        self_hosted::MODE_COMPANY | self_hosted::MODE_ENTERPRISE => 3,
        _ => 0,
    }
}

fn select_deployment_mode(
    theme: &ColorfulTheme,
    workspace_root: &Path,
) -> Result<DeploymentModeChoice> {
    let default_mode = self_hosted::load_manifest(workspace_root)?
        .map(|manifest| DeploymentModeChoice::from_mode(&manifest.profile.mode))
        .unwrap_or(DeploymentModeChoice::Solo);
    let items = DeploymentModeChoice::items();
    let labels = items
        .iter()
        .map(|choice| choice.label())
        .collect::<Vec<_>>();
    let selection = Select::with_theme(theme)
        .with_prompt("Choose the self-hosted deployment path")
        .items(&labels)
        .default(default_mode.index())
        .interact()?;
    Ok(items[selection])
}

fn steps_for_profile(profile: OnboardingProfile) -> Vec<OnboardingStep> {
    match profile {
        OnboardingProfile::Standard => vec![
            OnboardingStep::Gateway,
            OnboardingStep::Channel,
            OnboardingStep::Model,
            OnboardingStep::ControlPlane,
        ],
        OnboardingProfile::Advanced => vec![
            OnboardingStep::Gateway,
            OnboardingStep::Channel,
            OnboardingStep::Model,
            OnboardingStep::ControlPlane,
            OnboardingStep::Skill,
            OnboardingStep::Daemon,
        ],
        OnboardingProfile::Custom => Vec::new(),
    }
}

fn select_setup_path(
    theme: &ColorfulTheme,
    mode: &str,
    prior_mode: Option<&str>,
) -> Result<(OnboardingProfile, Vec<OnboardingStep>)> {
    let default_profile = default_onboarding_profile_for_mode(prior_mode.unwrap_or(mode));
    let profile = match Select::with_theme(theme)
        .with_prompt(format!(
            "Choose how much of the {} setup to do now",
            self_hosted::descriptor_for(mode)?.label
        ))
        .items(&[
            "Standard - gateway, one channel, provider, and control plane",
            "Advanced - everything in Standard plus skills and system service",
            "Custom - choose the exact setup steps to run now",
        ])
        .default(default_profile.index())
        .interact()?
    {
        1 => OnboardingProfile::Advanced,
        2 => OnboardingProfile::Custom,
        _ => OnboardingProfile::Standard,
    };

    if !matches!(profile, OnboardingProfile::Custom) {
        return Ok((profile, steps_for_profile(profile)));
    }

    let all_steps = [
        OnboardingStep::Gateway,
        OnboardingStep::Channel,
        OnboardingStep::Model,
        OnboardingStep::ControlPlane,
        OnboardingStep::Skill,
        OnboardingStep::Daemon,
    ];
    let labels = all_steps
        .iter()
        .map(|step| format!("{} - {}", step.name(), step.description()))
        .collect::<Vec<_>>();
    let defaults = [true, true, true, true, false, false];
    let selections = MultiSelect::with_theme(theme)
        .with_prompt("Choose the setup steps to run now")
        .items(&labels)
        .defaults(&defaults)
        .interact()?;

    let mut steps = selections
        .iter()
        .filter_map(|idx| all_steps.get(*idx).copied())
        .collect::<Vec<_>>();
    if steps.is_empty() {
        steps = steps_for_profile(OnboardingProfile::Standard);
    }
    Ok((profile, steps))
}

async fn backup_and_reset_workspace_state() -> Result<PathBuf> {
    let workspace_root = std::env::current_dir()?;
    let backup_root = workspace_root
        .join(".claw")
        .join("onboard-backups")
        .join(Utc::now().format("%Y%m%d%H%M%S").to_string());
    fs::create_dir_all(&backup_root).await?;

    let env_path = workspace_root.join(".env");
    if env_path.exists() {
        fs::copy(&env_path, backup_root.join("env.backup")).await?;
        fs::remove_file(&env_path).await?;
    }

    let control_root = control::control_root_for(&workspace_root);
    if control_root.exists() {
        copy_dir_recursive(&control_root, &backup_root.join("control")).await?;
        fs::remove_dir_all(&control_root).await?;
    }

    let channels_root = channels::channels_root_for(&workspace_root);
    if channels_root.exists() {
        copy_dir_recursive(&channels_root, &backup_root.join("channels")).await?;
        fs::remove_dir_all(&channels_root).await?;
    }

    Ok(backup_root)
}

async fn copy_dir_recursive(source: &Path, dest: &Path) -> Result<()> {
    let source = source.to_path_buf();
    let dest = dest.to_path_buf();
    tokio::task::spawn_blocking(move || -> Result<()> {
        fn copy_dir(source: &Path, dest: &Path) -> Result<()> {
            std::fs::create_dir_all(dest)?;
            for entry in std::fs::read_dir(source)? {
                let entry = entry?;
                let entry_path = entry.path();
                let target = dest.join(entry.file_name());
                if entry.file_type()?.is_dir() {
                    copy_dir(&entry_path, &target)?;
                } else {
                    std::fs::copy(&entry_path, &target)?;
                }
            }
            Ok(())
        }

        copy_dir(&source, &dest)
    })
    .await??;
    Ok(())
}

// ============================================================================
// Entry Point
// ============================================================================

/// Run the interactive onboarding wizard
pub async fn run() -> Result<()> {
    let mut wizard = OnboardingWizard::new();
    wizard.run().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onboarding_state_default() {
        let state = OnboardingState::default();
        assert!(!state.gateway_configured);
        assert!(state.channels_configured.is_empty());
        assert!(!state.model_configured);
        assert!(state.preferred_provider.is_none());
        assert!(state.deployment_mode.is_none());
        assert!(state.deployment_path.is_none());
        assert!(!state.daemon_installed);
        assert!(state.skills_installed.is_empty());
        assert!(state.profile.is_none());
        assert!(state.workspace_action.is_none());
    }

    #[test]
    fn test_onboarding_wizard_new() {
        let wizard = OnboardingWizard::new();
        assert!(!wizard.state.gateway_configured);
        assert!(wizard.state.channels_configured.is_empty());
        assert!(!wizard.state.model_configured);
        assert!(wizard.state.preferred_provider.is_none());
        assert!(wizard.state.deployment_mode.is_none());
        assert!(wizard.state.deployment_path.is_none());
        assert!(!wizard.state.daemon_installed);
        assert!(wizard.state.skills_installed.is_empty());
        assert!(wizard.state.profile.is_none());
        assert!(wizard.state.workspace_action.is_none());
    }

    #[test]
    fn test_onboarding_step_names() {
        assert_eq!(OnboardingStep::Gateway.name(), "Gateway Setup");
        assert_eq!(OnboardingStep::Channel.name(), "Channel Setup");
        assert_eq!(OnboardingStep::Model.name(), "AI Model Setup");
        assert_eq!(OnboardingStep::Skill.name(), "Skills Setup");
        assert_eq!(OnboardingStep::Daemon.name(), "System Service");
    }

    #[test]
    fn test_onboarding_step_descriptions() {
        assert!(!OnboardingStep::Gateway.description().is_empty());
        assert!(!OnboardingStep::Channel.description().is_empty());
        assert!(!OnboardingStep::Model.description().is_empty());
        assert!(!OnboardingStep::Skill.description().is_empty());
        assert!(!OnboardingStep::Daemon.description().is_empty());
    }

    #[test]
    fn test_generate_jwt_secret_is_nonempty() {
        let secret = generate_jwt_secret();
        assert!(!secret.is_empty());
    }

    #[test]
    fn test_generate_jwt_secret_is_unique() {
        let secret1 = generate_jwt_secret();
        let secret2 = generate_jwt_secret();
        assert_ne!(secret1, secret2);
    }

    #[test]
    fn test_generate_jwt_secret_is_base64() {
        let secret = generate_jwt_secret();
        // Should be valid base64
        let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &secret);
        assert!(decoded.is_ok());
        // Should be 32 bytes when decoded
        assert_eq!(decoded.unwrap().len(), 32);
    }

    #[test]
    fn test_onboarding_state_channels_can_be_modified() {
        let mut state = OnboardingState::default();
        state.channels_configured.push("telegram".to_string());
        state.channels_configured.push("discord".to_string());
        assert_eq!(state.channels_configured.len(), 2);
        assert!(state.channels_configured.contains(&"telegram".to_string()));
        assert!(state.channels_configured.contains(&"discord".to_string()));
    }

    #[test]
    fn test_onboarding_state_skills_can_be_modified() {
        let mut state = OnboardingState::default();
        state.skills_installed.push("memory".to_string());
        assert_eq!(state.skills_installed.len(), 1);
    }

    #[test]
    fn test_onboarding_state_flags_can_be_set() {
        let mut state = OnboardingState::default();
        state.gateway_configured = true;
        state.model_configured = true;
        state.preferred_provider = Some("ollama".to_string());
        state.daemon_installed = true;
        assert!(state.gateway_configured);
        assert!(state.model_configured);
        assert_eq!(state.preferred_provider.as_deref(), Some("ollama"));
        assert!(state.daemon_installed);
    }

    #[test]
    fn test_assistant_provider_prefers_onboarding_state() {
        let mut wizard = OnboardingWizard::new();
        wizard.state.preferred_provider = Some("openrouter".to_string());

        assert_eq!(wizard.assistant_provider().as_deref(), Some("openrouter"));
    }

    #[test]
    fn test_workspace_status_defaults_to_absent() {
        let dir = tempfile::tempdir().unwrap();
        let status = workspace_status(dir.path());
        assert!(!status.env_present);
        assert!(!status.control_registry_present);
        assert!(!status.channels_registry_present);
        assert!(!status.product_mode_present);
        assert!(status.product_mode_label.is_none());
        assert!(!status.setup_state_present);
        assert!(status.setup_status.is_none());
        assert!(status.setup_next_action.is_none());
    }

    #[test]
    fn test_default_onboarding_profile_for_enterprise_is_advanced() {
        assert_eq!(
            default_onboarding_profile_for_mode(self_hosted::MODE_ENTERPRISE).label(),
            "Advanced"
        );
        assert_eq!(
            default_onboarding_profile_for_mode(self_hosted::MODE_SOLO).label(),
            "Standard"
        );
    }

    #[test]
    fn test_steps_for_standard_and_advanced_paths() {
        assert_eq!(steps_for_profile(OnboardingProfile::Standard).len(), 4);
        assert_eq!(steps_for_profile(OnboardingProfile::Advanced).len(), 6);
        assert!(steps_for_profile(OnboardingProfile::Custom).is_empty());
    }

    #[test]
    fn test_setup_state_round_trip_and_resume_detection() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = SetupStateManifest {
            version: SETUP_STATE_VERSION,
            setup: SetupState {
                started_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
                completed_at: None,
                status: "in_progress".to_string(),
                workspace_action: "new_workspace".to_string(),
                deployment_mode: Some(self_hosted::MODE_SOLO.to_string()),
                deployment_path: Some("solo_starter".to_string()),
                remote_connectivity_profile: Some(RemoteConnectivityProfile {
                    mode: "remote_access".to_string(),
                    primary_path: "node_first".to_string(),
                    fallback_paths: vec!["ssh_tunnel".to_string()],
                    detail: "Remote access profile saved.".to_string(),
                }),
                setup_path: Some("Custom".to_string()),
                selected_steps: vec!["gateway".to_string(), "model".to_string()],
                completed_steps: vec!["gateway".to_string()],
                blockers: Vec::new(),
                next_action: Some("Complete AI Model Setup".to_string()),
                current_step: Some("model".to_string()),
                bootstrap_outcomes: Vec::new(),
            },
        };

        save_setup_state(dir.path(), &manifest).unwrap();
        let loaded = load_setup_state(dir.path()).unwrap().unwrap();
        assert_eq!(loaded.setup.setup_path.as_deref(), Some("Custom"));
        assert_eq!(
            loaded
                .setup
                .remote_connectivity_profile
                .as_ref()
                .map(|profile| profile.primary_path.as_str()),
            Some("node_first")
        );
        assert!(setup_state_is_resumable(&loaded));
        let steps = selected_steps_from_setup_state(&loaded.setup);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].id(), "model");
    }

    #[test]
    fn test_remote_connectivity_profiles_capture_fallback_order() {
        let local = local_connectivity_profile();
        assert_eq!(local.mode, "local_only");
        assert!(local.fallback_paths.is_empty());

        let remote = remote_connectivity_profile_for_selection(0);
        assert_eq!(remote.primary_path, "node_first");
        assert_eq!(
            remote.fallback_paths,
            vec!["ssh_tunnel".to_string(), "reverse_proxy".to_string()]
        );
    }

    #[test]
    fn test_record_bootstrap_outcome_replaces_existing_entry() {
        let dir = tempfile::tempdir().unwrap();
        record_bootstrap_outcome(
            dir.path(),
            "provider",
            "anthropic",
            "warning",
            "initial warning",
        )
        .unwrap();
        record_bootstrap_outcome(
            dir.path(),
            "provider",
            "anthropic",
            "ready",
            "provider reachable",
        )
        .unwrap();

        let loaded = load_setup_state(dir.path()).unwrap().unwrap();
        assert_eq!(loaded.setup.bootstrap_outcomes.len(), 1);
        let outcome = bootstrap_outcome_for(&loaded.setup, "provider", "anthropic").unwrap();
        assert_eq!(outcome.status, "ready");
        assert_eq!(outcome.detail, "provider reachable");
    }

    #[test]
    fn test_validate_execution_mode_bootstrap_warns_on_non_recommended_mode() {
        let dir = tempfile::tempdir().unwrap();
        let control_root = control::control_root_for(dir.path());
        let control_root_str = control_root.display().to_string();
        self_hosted::configure_mode(
            dir.path(),
            self_hosted::MODE_ENTERPRISE,
            Some("enterprise_governed_setup"),
            None,
        )
        .unwrap();
        control::init(Some(&control_root_str)).unwrap();
        control::configure_mode(
            Some(&control_root_str),
            "task_assigned",
            Some("main"),
            None,
            false,
            Some("strict"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let assessment = validate_execution_mode_bootstrap(
            dir.path(),
            "task_assigned",
            Some(self_hosted::MODE_ENTERPRISE),
        )
        .unwrap();
        assert_eq!(assessment.status, "warning");
        assert!(!assessment.blocking);
        assert!(assessment.detail.contains("recommends `orchestrated`"));
    }

    #[test]
    fn test_derive_setup_repair_plan_uses_bootstrap_outcomes_and_diagnostics() {
        let setup = SetupState {
            started_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            completed_at: None,
            status: "blocked".to_string(),
            workspace_action: "repair_existing".to_string(),
            deployment_mode: Some(self_hosted::MODE_TEAM.to_string()),
            deployment_path: Some("shared_team_setup".to_string()),
            remote_connectivity_profile: None,
            setup_path: Some("Advanced".to_string()),
            selected_steps: vec![
                "gateway".to_string(),
                "model".to_string(),
                "channel".to_string(),
            ],
            completed_steps: vec!["gateway".to_string()],
            blockers: vec!["Provider bootstrap failed".to_string()],
            next_action: Some("Review AI Model Setup and rerun onboarding".to_string()),
            current_step: Some("model".to_string()),
            bootstrap_outcomes: vec![
                SetupBootstrapOutcome {
                    category: "provider".to_string(),
                    target: "anthropic".to_string(),
                    status: "blocked".to_string(),
                    detail: "provider not ready".to_string(),
                    updated_at: Utc::now().to_rfc3339(),
                },
                SetupBootstrapOutcome {
                    category: "channel".to_string(),
                    target: "slack".to_string(),
                    status: "warning".to_string(),
                    detail: "channel probe failed".to_string(),
                    updated_at: Utc::now().to_rfc3339(),
                },
            ],
        };
        let report = doctor::DiagnosticReport {
            generated_at: Utc::now(),
            config_path: "config/default.toml".to_string(),
            deep: true,
            checks: vec![
                doctor::DiagnosticCheck {
                    id: "api_keys".to_string(),
                    label: "provider API keys".to_string(),
                    status: doctor::DiagnosticStatus::Failed,
                    message: Some("Anthropic key missing".to_string()),
                },
                doctor::DiagnosticCheck {
                    id: "channel_readiness".to_string(),
                    label: "channel readiness".to_string(),
                    status: doctor::DiagnosticStatus::Failed,
                    message: Some("Slack auth probe failed".to_string()),
                },
            ],
            passed: 0,
            warnings: 0,
            failed: 2,
            healthy: false,
        };

        let plan = derive_setup_repair_plan(Some(&setup), &report);
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].id(), "model");
        assert_eq!(plan.steps[1].id(), "channel");
        assert!(
            plan.reasons
                .iter()
                .any(|reason| reason.contains("unfinished steps"))
        );
        assert!(
            plan.reasons
                .iter()
                .any(|reason| reason.contains("provider not ready"))
        );
        assert!(
            plan.reasons
                .iter()
                .any(|reason| reason.contains("Slack auth probe failed"))
        );
    }

    #[test]
    fn test_prepare_setup_state_for_repair_retargets_selected_steps() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = SetupStateManifest {
            version: SETUP_STATE_VERSION,
            setup: SetupState {
                started_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
                completed_at: Some(Utc::now().to_rfc3339()),
                status: "ready".to_string(),
                workspace_action: "new_workspace".to_string(),
                deployment_mode: Some(self_hosted::MODE_COMPANY.to_string()),
                deployment_path: Some("company_ops_setup".to_string()),
                remote_connectivity_profile: None,
                setup_path: Some("Advanced".to_string()),
                selected_steps: vec![
                    "gateway".to_string(),
                    "model".to_string(),
                    "channel".to_string(),
                ],
                completed_steps: vec![
                    "gateway".to_string(),
                    "model".to_string(),
                    "channel".to_string(),
                ],
                blockers: vec!["old blocker".to_string()],
                next_action: Some("done".to_string()),
                current_step: None,
                bootstrap_outcomes: Vec::new(),
            },
        };
        save_setup_state(dir.path(), &manifest).unwrap();

        prepare_setup_state_for_repair(
            dir.path(),
            &[OnboardingStep::Model, OnboardingStep::Channel],
        )
        .unwrap();

        let loaded = load_setup_state(dir.path()).unwrap().unwrap();
        assert_eq!(loaded.setup.workspace_action, "repair_existing");
        assert_eq!(loaded.setup.status, "in_progress");
        assert_eq!(
            loaded.setup.selected_steps,
            vec!["model".to_string(), "channel".to_string()]
        );
        assert_eq!(loaded.setup.completed_steps, vec!["gateway".to_string()]);
        assert_eq!(loaded.setup.current_step.as_deref(), Some("model"));
        assert_eq!(
            loaded.setup.next_action.as_deref(),
            Some("Complete AI Model Setup")
        );
    }

    #[test]
    fn test_default_runtime_mode_index_matches_deployment_mode() {
        assert_eq!(default_runtime_mode_index(Some(self_hosted::MODE_SOLO)), 0);
        assert_eq!(default_runtime_mode_index(Some(self_hosted::MODE_TEAM)), 1);
        assert_eq!(
            default_runtime_mode_index(Some(self_hosted::MODE_ENTERPRISE)),
            3
        );
    }

    #[test]
    fn test_should_offer_assistant_launch_requires_health_terminal_and_provider() {
        assert!(should_offer_assistant_launch(
            true,
            true,
            Some("openrouter")
        ));
        assert!(!should_offer_assistant_launch(
            false,
            true,
            Some("openrouter")
        ));
        assert!(!should_offer_assistant_launch(
            true,
            false,
            Some("openrouter")
        ));
        assert!(!should_offer_assistant_launch(true, true, None));
    }
}

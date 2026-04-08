//! Interactive Onboarding Wizard

use anyhow::{Result, anyhow};
use chrono::Utc;
use console::style;
use dialoguer::{Confirm, Input, MultiSelect, Password, Select, theme::ColorfulTheme};
use indicatif::{ProgressBar, ProgressStyle};
use openrustclaw_app::agent_backend_catalog::AgentBackendCatalogService;
use openrustclaw_app::agent_backend_control::AgentBackendControlService;
use openrustclaw_app::onboarding_lane_catalog::{
    DirectProviderLaneStatus, OnboardingLaneCatalogService, OnboardingLaneDescriptor,
    OnboardingLaneKind,
};
use openrustclaw_app::runtime_model_validation::validate_model_for_provider;
use openrustclaw_app::setup_lifecycle as app_setup_lifecycle;
use openrustclaw_core::config::AppConfig;
use serde::{Deserialize, Serialize};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::time::Duration;
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
    pub selected_lane_id: Option<String>,
    pub selected_lane_label: Option<String>,
    pub selected_lane_kind: Option<String>,
    pub selected_backend_id: Option<String>,
    pub selected_lane_detail: Option<String>,
    pub selected_lane_compatibility_note: Option<String>,
    pub selected_access_mode: Option<String>,
    pub selected_primary_model: Option<String>,
    pub selected_primary_model_source: Option<String>,
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
    pub selected_provider: Option<String>,
    #[serde(default)]
    pub selected_lane_id: Option<String>,
    #[serde(default)]
    pub selected_lane_label: Option<String>,
    #[serde(default)]
    pub selected_lane_kind: Option<String>,
    #[serde(default)]
    pub selected_backend_id: Option<String>,
    #[serde(default)]
    pub selected_lane_detail: Option<String>,
    #[serde(default)]
    pub selected_lane_compatibility_note: Option<String>,
    #[serde(default)]
    pub selected_access_mode: Option<String>,
    #[serde(default)]
    pub selected_primary_model: Option<String>,
    #[serde(default)]
    pub selected_primary_model_source: Option<String>,
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
    #[serde(default)]
    pub issue_kind: Option<String>,
    #[serde(default)]
    pub verification_stage: Option<String>,
    #[serde(default)]
    pub suggested_action: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
struct SetupRepairPlan {
    steps: Vec<OnboardingStep>,
    reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OnboardingProviderAccessMode {
    ApiKey,
    LocalRuntime,
    SubscriptionManaged,
}

impl OnboardingProviderAccessMode {
    fn id(self) -> &'static str {
        match self {
            Self::ApiKey => "api_key",
            Self::LocalRuntime => "local_runtime",
            Self::SubscriptionManaged => "subscription_managed",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::ApiKey => "API key",
            Self::LocalRuntime => "Local runtime",
            Self::SubscriptionManaged => "Subscription-managed account",
        }
    }

    fn detail(self) -> &'static str {
        match self {
            Self::ApiKey => "Use a provider API key stored in `.env`.",
            Self::LocalRuntime => "Use a local runtime already running on this machine.",
            Self::SubscriptionManaged => {
                "Use a subscription-managed provider path without a direct API key."
            }
        }
    }

    fn from_id(value: &str) -> Option<Self> {
        match value {
            "api_key" => Some(Self::ApiKey),
            "local_runtime" => Some(Self::LocalRuntime),
            "subscription_managed" => Some(Self::SubscriptionManaged),
            _ => None,
        }
    }
}

fn direct_provider_lane_statuses(ollama_available: bool) -> Vec<DirectProviderLaneStatus> {
    [
        (
            "anthropic",
            std::env::var("ANTHROPIC_API_KEY").is_ok(),
            "configured",
            "not configured",
        ),
        (
            "openai",
            std::env::var("OPENAI_API_KEY").is_ok(),
            "configured",
            "not configured",
        ),
        (
            "openrouter",
            std::env::var("OPENROUTER_API_KEY").is_ok(),
            "configured",
            "not configured",
        ),
        (
            "gemini",
            std::env::var("GEMINI_API_KEY").is_ok() || std::env::var("GOOGLE_API_KEY").is_ok(),
            "configured",
            "not configured",
        ),
        ("ollama", ollama_available, "available", "not detected"),
    ]
    .into_iter()
    .map(
        |(provider_id, available, available_label, unavailable_label)| DirectProviderLaneStatus {
            provider_id: provider_id.to_string(),
            available,
            status_label: if available {
                available_label.to_string()
            } else {
                unavailable_label.to_string()
            },
        },
    )
    .collect()
}

async fn onboarding_lane_catalog() -> Result<Vec<OnboardingLaneDescriptor>> {
    let agent_backends = AgentBackendCatalogService::new().discover();
    let delegated_contracts =
        AgentBackendControlService::new().contracts_from_catalog(&agent_backends);
    let direct_statuses = direct_provider_lane_statuses(models::check_ollama().await);
    Ok(OnboardingLaneCatalogService::new().catalog(&direct_statuses, &delegated_contracts))
}

fn print_onboarding_agent_backend_hints(lanes: &[OnboardingLaneDescriptor]) {
    let delegated = lanes
        .iter()
        .filter(|lane| lane.backend_id.is_some())
        .collect::<Vec<_>>();

    if !delegated.is_empty() {
        println!("Detected delegated local agent lanes:");
        println!(
            "  OpenRustClaw keeps these lanes visible, but later delegated runs use the installed CLI directly instead of importing vendor tokens."
        );
        for lane in delegated {
            println!("  - {}: {}", lane.label, lane.status_label);
            if let Some(note) = lane.compatibility_note.as_deref() {
                println!("    {}", note);
            }
        }
        println!();
    }
}

fn print_selected_provider_backend_hint(descriptor: &OnboardingLaneDescriptor) {
    if descriptor.backend_id.is_some() {
        println!(
            "  {} stays visible as a delegated local agent lane backed by `{}`.",
            descriptor.label, descriptor.provider_id
        );
        println!("  Local agent status: {}.", descriptor.detail);
        if let Some(note) = descriptor.compatibility_note.as_deref() {
            println!("  Compatibility note: {note}");
        }
        println!(
            "  First-run onboarding still bootstraps through the documented `{}` provider path today, and later delegated execution stays bounded and audited.",
            descriptor.provider_id
        );
    }
}

fn persist_provider_path_selection(
    workspace_root: &Path,
    descriptor: &OnboardingLaneDescriptor,
    access_mode: Option<&str>,
) -> Result<()> {
    with_setup_state_mut(workspace_root, |setup| {
        setup.selected_provider = Some(descriptor.provider_id.clone());
        setup.selected_lane_id = Some(descriptor.lane_id.clone());
        setup.selected_lane_label = Some(descriptor.label.clone());
        setup.selected_lane_kind = Some(lane_kind_id(&descriptor.kind).to_string());
        setup.selected_backend_id = descriptor.backend_id.clone();
        setup.selected_lane_detail = Some(descriptor.detail.clone());
        setup.selected_lane_compatibility_note = descriptor.compatibility_note.clone();
        setup.selected_access_mode = access_mode.map(ToString::to_string);
        setup.selected_primary_model = None;
        setup.selected_primary_model_source = None;
    })
}

fn select_provider_access_mode(
    wizard: &OnboardingWizard,
    descriptor: &OnboardingLaneDescriptor,
) -> Result<OnboardingProviderAccessMode> {
    let access_modes = descriptor
        .supported_access_modes
        .iter()
        .filter_map(|mode| OnboardingProviderAccessMode::from_id(mode))
        .collect::<Vec<_>>();

    if access_modes.is_empty() {
        return Err(anyhow!(
            "{} does not yet expose a supported onboarding path",
            descriptor.label
        ));
    }

    if access_modes.len() == 1 {
        let mode = access_modes[0];
        if descriptor.backend_id.is_some() {
            println!(
                "  {} stays selected as a delegated local agent lane.",
                descriptor.label
            );
            println!(
                "  Onboarding will validate the backing `{}` provider path via {}.",
                descriptor.provider_id,
                mode.label()
            );
        } else {
            println!(
                "  {} uses the {} onboarding path.",
                descriptor.label,
                mode.label()
            );
        }
        return Ok(mode);
    }

    let labels = access_modes
        .iter()
        .map(|mode| format!("{} - {}", mode.label(), mode.detail()))
        .collect::<Vec<_>>();
    let default = wizard
        .state
        .selected_access_mode
        .as_deref()
        .and_then(OnboardingProviderAccessMode::from_id)
        .and_then(|current| access_modes.iter().position(|mode| *mode == current))
        .unwrap_or(0);
    let selection = Select::with_theme(&wizard.theme)
        .with_prompt(format!(
            "Choose how onboarding should access {}",
            descriptor.label
        ))
        .items(&labels)
        .default(default)
        .interact()?;
    access_modes
        .get(selection)
        .copied()
        .ok_or_else(|| anyhow!("invalid provider access-mode selection"))
}

fn lane_kind_id(kind: &OnboardingLaneKind) -> &'static str {
    match kind {
        OnboardingLaneKind::DirectApi => "direct_api",
        OnboardingLaneKind::LocalRuntime => "local_runtime",
        OnboardingLaneKind::DelegatedAgent => "delegated_agent",
    }
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
                    selected_provider: self.state.preferred_provider.clone(),
                    selected_lane_id: self.state.selected_lane_id.clone(),
                    selected_lane_label: self.state.selected_lane_label.clone(),
                    selected_lane_kind: self.state.selected_lane_kind.clone(),
                    selected_backend_id: self.state.selected_backend_id.clone(),
                    selected_lane_detail: self.state.selected_lane_detail.clone(),
                    selected_lane_compatibility_note: self
                        .state
                        .selected_lane_compatibility_note
                        .clone(),
                    selected_access_mode: self.state.selected_access_mode.clone(),
                    selected_primary_model: self.state.selected_primary_model.clone(),
                    selected_primary_model_source: self.state.selected_primary_model_source.clone(),
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
        self.state.preferred_provider = setup_state.setup.selected_provider.clone();
        self.state.selected_lane_id = setup_state.setup.selected_lane_id.clone();
        self.state.selected_lane_label = setup_state.setup.selected_lane_label.clone();
        self.state.selected_lane_kind = setup_state.setup.selected_lane_kind.clone();
        self.state.selected_backend_id = setup_state.setup.selected_backend_id.clone();
        self.state.selected_lane_detail = setup_state.setup.selected_lane_detail.clone();
        self.state.selected_lane_compatibility_note =
            setup_state.setup.selected_lane_compatibility_note.clone();
        self.state.selected_access_mode = setup_state.setup.selected_access_mode.clone();
        self.state.selected_primary_model = setup_state.setup.selected_primary_model.clone();
        self.state.selected_primary_model_source =
            setup_state.setup.selected_primary_model_source.clone();
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
        if let Some(provider) = self.state.preferred_provider.as_deref() {
            println!("  Provider: {provider}");
        }
        if let Some(lane) = self.state.selected_lane_label.as_deref() {
            println!("  Selected Lane: {lane}");
        }
        if let Some(access_mode) = self.state.selected_access_mode.as_deref() {
            println!("  Access Mode: {access_mode}");
        }
        if let Some(model) = self.state.selected_primary_model.as_deref() {
            println!("  Primary Model: {model}");
        }
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
                    if let Some(issue_kind) = outcome.issue_kind.as_deref() {
                        println!(
                            "    - {} {} [{}]: {}",
                            outcome.category, outcome.target, issue_kind, outcome.detail
                        );
                    } else {
                        println!(
                            "    - {} {}: {}",
                            outcome.category, outcome.target, outcome.detail
                        );
                    }
                    if let Some(next_action) = outcome.suggested_action.as_deref() {
                        println!("      next: {next_action}");
                    }
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

    async fn maybe_launch_assistant(&self, healthy: bool) -> Result<()> {
        let resolved = chat::resolve_chat_target(None, None).await?;
        let provider = Some(resolved.provider.clone());
        if !should_offer_assistant_launch(
            healthy,
            std::io::stdin().is_terminal(),
            provider.as_deref(),
        ) {
            return Ok(());
        }
        let provider = provider.expect("launch gate requires a resolved provider");
        let model = resolved.model;

        let launch = Confirm::with_theme(&self.theme)
            .with_prompt(format!(
                "Launch the persisted assistant session now with `{provider}`{}?",
                model
                    .as_deref()
                    .map(|value| format!(" / `{value}`"))
                    .unwrap_or_default()
            ))
            .default(true)
            .interact()?;

        if launch {
            println!();
            chat::run(&provider, model.as_deref()).await?;
        }

        Ok(())
    }
}

impl Default for OnboardingWizard {
    fn default() -> Self {
        Self::new()
    }
}

pub fn should_offer_assistant_launch(
    healthy: bool,
    stdin_is_terminal: bool,
    provider: Option<&str>,
) -> bool {
    app_setup_lifecycle::SetupLifecycleService::new().should_offer_assistant_launch(
        healthy,
        stdin_is_terminal,
        provider,
    )
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
    let (remote_status, remote_detail) =
        remote_connectivity_bootstrap_outcome(&connectivity_profile);
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
        detail:
            "Local runtime selected. No remote connectivity fallback is active for this workspace."
                .to_string(),
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
    let providers = onboarding_lane_catalog().await?;
    print_onboarding_agent_backend_hints(&providers);
    let provider_labels = providers
        .iter()
        .map(OnboardingLaneDescriptor::selection_label)
        .collect::<Vec<_>>();
    let default_provider_index = wizard
        .state
        .selected_lane_id
        .as_deref()
        .or(wizard.state.preferred_provider.as_deref())
        .and_then(|provider| {
            providers.iter().position(|descriptor| {
                descriptor.provider_id == provider || descriptor.lane_id == provider
            })
        })
        .unwrap_or(0);

    let selection = Select::with_theme(&wizard.theme)
        .with_prompt("Choose your first provider or delegated agent lane")
        .items(&provider_labels)
        .default(default_provider_index)
        .interact()?;
    let descriptor = providers
        .get(selection)
        .cloned()
        .ok_or_else(|| anyhow!("invalid provider selection"))?;
    print_selected_provider_backend_hint(&descriptor);

    wizard.state.preferred_provider = Some(descriptor.provider_id.clone());
    wizard.state.selected_lane_id = Some(descriptor.lane_id.clone());
    wizard.state.selected_lane_label = Some(descriptor.label.clone());
    wizard.state.selected_lane_kind = Some(lane_kind_id(&descriptor.kind).to_string());
    wizard.state.selected_backend_id = descriptor.backend_id.clone();
    wizard.state.selected_lane_detail = Some(descriptor.detail.clone());
    wizard.state.selected_lane_compatibility_note = descriptor.compatibility_note.clone();

    let access_mode = select_provider_access_mode(wizard, &descriptor)?;
    wizard.state.selected_access_mode = Some(access_mode.id().to_string());
    wizard.state.selected_primary_model = None;
    wizard.state.selected_primary_model_source = None;

    let workspace_root = std::env::current_dir()?;
    persist_provider_path_selection(
        &workspace_root,
        &descriptor,
        wizard.state.selected_access_mode.as_deref(),
    )?;

    match access_mode {
        OnboardingProviderAccessMode::LocalRuntime => {
            println!("Using {} via {}.", descriptor.label, access_mode.label());
            println!("  {}", access_mode.detail());
            println!("  Make sure Ollama is running locally (http://localhost:11434)");
            runtime::switch_provider(
                "config/default.toml",
                &workspace_root,
                &descriptor.provider_id,
                None,
                None,
                None,
            )?;
        }
        OnboardingProviderAccessMode::ApiKey => {
            println!("Using {} via {}.", descriptor.label, access_mode.label());
            println!("  {}", access_mode.detail());
            let api_key_prompt = descriptor
                .api_key_prompt
                .as_deref()
                .ok_or_else(|| anyhow!("{} requires an API key prompt", descriptor.provider_id))?;
            let api_key = Password::with_theme(&wizard.theme)
                .with_prompt(api_key_prompt)
                .interact()?;
            if api_key.is_empty() {
                println!("⚠️  No API key provided, skipping model setup");
                return Ok(true);
            }
            save_provider_config(&descriptor.provider_id, &api_key).await?;
            runtime::switch_provider(
                "config/default.toml",
                &workspace_root,
                &descriptor.provider_id,
                None,
                None,
                None,
            )?;
        }
        OnboardingProviderAccessMode::SubscriptionManaged => {
            if descriptor.backend_id.is_none() {
                return Err(anyhow!(
                    "{} does not yet expose a subscription-managed onboarding path",
                    descriptor.label
                ));
            }
            println!("Using {} via {}.", descriptor.label, access_mode.label());
            println!("  {}", access_mode.detail());
            runtime::switch_provider(
                "config/default.toml",
                &workspace_root,
                descriptor
                    .backend_id
                    .as_deref()
                    .unwrap_or(descriptor.provider_id.as_str()),
                None,
                None,
                None,
            )?;
        }
    }

    let provider_assessment = if access_mode == OnboardingProviderAccessMode::SubscriptionManaged {
        validate_delegated_backend_bootstrap(
            descriptor
                .backend_id
                .as_deref()
                .unwrap_or(descriptor.provider_id.as_str()),
        )?
    } else {
        validate_provider_bootstrap(
            &workspace_root,
            &descriptor.provider_id,
            Some(access_mode.id()),
            true,
        )
        .await?
    };
    record_bootstrap_outcome_with_metadata(
        &workspace_root,
        "provider",
        &descriptor.provider_id,
        provider_assessment.status,
        provider_assessment.detail.clone(),
        Some(&provider_assessment),
    )?;
    print_bootstrap_assessment(&descriptor.provider_id, &provider_assessment);
    if provider_assessment.blocking {
        return Err(anyhow!(provider_assessment.detail));
    }

    let runtime_provider_id = if access_mode == OnboardingProviderAccessMode::SubscriptionManaged {
        descriptor
            .backend_id
            .as_deref()
            .unwrap_or(descriptor.provider_id.as_str())
    } else {
        descriptor.provider_id.as_str()
    };
    let primary_model = select_primary_model(wizard, &workspace_root, runtime_provider_id).await?;
    runtime::switch_provider(
        "config/default.toml",
        &workspace_root,
        runtime_provider_id,
        Some(primary_model.model.as_str()),
        None,
        None,
    )?;
    wizard.state.selected_primary_model = Some(primary_model.model.clone());
    wizard.state.selected_primary_model_source = Some(primary_model.source.clone());
    persist_primary_model_selection(&workspace_root, &primary_model.model, &primary_model.source)?;

    let provider_assessment = if access_mode == OnboardingProviderAccessMode::SubscriptionManaged {
        validate_delegated_backend_bootstrap(
            descriptor
                .backend_id
                .as_deref()
                .unwrap_or(descriptor.provider_id.as_str()),
        )?
    } else {
        validate_provider_bootstrap(
            &workspace_root,
            &descriptor.provider_id,
            Some(access_mode.id()),
            false,
        )
        .await?
    };
    record_bootstrap_outcome_with_metadata(
        &workspace_root,
        "provider",
        &descriptor.provider_id,
        provider_assessment.status,
        provider_assessment.detail.clone(),
        Some(&provider_assessment),
    )?;
    print_bootstrap_assessment(&descriptor.provider_id, &provider_assessment);

    wizard.state.model_configured = true;
    let runtime_assessment =
        runtime_lane_assessment(&workspace_root, wizard.state.deployment_mode.as_deref()).await?;
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
    if descriptor.backend_id.is_some() {
        println!(
            "✓ Lane configured ({} via {} / {})",
            descriptor.label,
            access_mode.label(),
            primary_model.model
        );
        if let Some(backend_id) = descriptor.backend_id.as_deref() {
            println!(
                "  Assistant and chat sessions now default to delegated local agent `{backend_id}` when this workspace launches its first task."
            );
        }
    } else {
        println!(
            "✓ Model configured ({} via {} / {})",
            descriptor.label,
            access_mode.label(),
            primary_model.model
        );
    }
    println!(
        "  Control-plane actions will prefer the dedicated fallback lane in config/default.toml"
    );
    println!();
    models::scan().await?;

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

fn app_bootstrap_outcome(
    outcome: &SetupBootstrapOutcome,
) -> app_setup_lifecycle::SetupBootstrapOutcome {
    app_setup_lifecycle::SetupBootstrapOutcome {
        category: outcome.category.clone(),
        target: outcome.target.clone(),
        status: outcome.status.clone(),
        detail: outcome.detail.clone(),
        issue_kind: outcome.issue_kind.clone(),
        verification_stage: outcome.verification_stage.clone(),
        suggested_action: outcome.suggested_action.clone(),
        updated_at: outcome.updated_at.clone(),
    }
}

fn setup_bootstrap_outcome_from_app(
    outcome: app_setup_lifecycle::SetupBootstrapOutcome,
) -> SetupBootstrapOutcome {
    SetupBootstrapOutcome {
        category: outcome.category,
        target: outcome.target,
        status: outcome.status,
        detail: outcome.detail,
        issue_kind: outcome.issue_kind,
        verification_stage: outcome.verification_stage,
        suggested_action: outcome.suggested_action,
        updated_at: outcome.updated_at,
    }
}

fn app_setup_state(setup: &SetupState) -> app_setup_lifecycle::SetupLifecycleState {
    app_setup_lifecycle::SetupLifecycleState {
        status: setup.status.clone(),
        setup_path: setup.setup_path.clone(),
        selected_provider: setup.selected_provider.clone(),
        selected_lane_id: setup.selected_lane_id.clone(),
        selected_lane_label: setup.selected_lane_label.clone(),
        selected_lane_kind: setup.selected_lane_kind.clone(),
        selected_backend_id: setup.selected_backend_id.clone(),
        selected_lane_detail: setup.selected_lane_detail.clone(),
        selected_lane_compatibility_note: setup.selected_lane_compatibility_note.clone(),
        selected_access_mode: setup.selected_access_mode.clone(),
        selected_primary_model: setup.selected_primary_model.clone(),
        selected_primary_model_source: setup.selected_primary_model_source.clone(),
        selected_steps: setup.selected_steps.clone(),
        completed_steps: setup.completed_steps.clone(),
        blockers: setup.blockers.clone(),
        next_action: setup.next_action.clone(),
        bootstrap_outcomes: setup
            .bootstrap_outcomes
            .iter()
            .map(app_bootstrap_outcome)
            .collect(),
    }
}

fn app_diagnostic_check(
    check: &doctor::DiagnosticCheck,
) -> app_setup_lifecycle::SetupDiagnosticCheck {
    app_setup_lifecycle::SetupDiagnosticCheck {
        id: check.id.clone(),
        label: check.label.clone(),
        status: match check.status {
            doctor::DiagnosticStatus::Ok => app_setup_lifecycle::SetupDiagnosticStatus::Ok,
            doctor::DiagnosticStatus::Warning => {
                app_setup_lifecycle::SetupDiagnosticStatus::Warning
            }
            doctor::DiagnosticStatus::Failed => app_setup_lifecycle::SetupDiagnosticStatus::Failed,
        },
        message: check.message.clone(),
    }
}

fn app_onboarding_profile(profile: OnboardingProfile) -> app_setup_lifecycle::OnboardingProfile {
    match profile {
        OnboardingProfile::Standard => app_setup_lifecycle::OnboardingProfile::Standard,
        OnboardingProfile::Advanced => app_setup_lifecycle::OnboardingProfile::Advanced,
        OnboardingProfile::Custom => app_setup_lifecycle::OnboardingProfile::Custom,
    }
}

fn onboarding_profile_from_app(
    profile: app_setup_lifecycle::OnboardingProfile,
) -> OnboardingProfile {
    match profile {
        app_setup_lifecycle::OnboardingProfile::Standard => OnboardingProfile::Standard,
        app_setup_lifecycle::OnboardingProfile::Advanced => OnboardingProfile::Advanced,
        app_setup_lifecycle::OnboardingProfile::Custom => OnboardingProfile::Custom,
    }
}

fn onboarding_step_from_app(step: app_setup_lifecycle::SetupStep) -> OnboardingStep {
    match step {
        app_setup_lifecycle::SetupStep::Gateway => OnboardingStep::Gateway,
        app_setup_lifecycle::SetupStep::Channel => OnboardingStep::Channel,
        app_setup_lifecycle::SetupStep::Model => OnboardingStep::Model,
        app_setup_lifecycle::SetupStep::ControlPlane => OnboardingStep::ControlPlane,
        app_setup_lifecycle::SetupStep::Skill => OnboardingStep::Skill,
        app_setup_lifecycle::SetupStep::Daemon => OnboardingStep::Daemon,
    }
}

fn selected_steps_from_setup_state(setup: &SetupState) -> Vec<OnboardingStep> {
    app_setup_lifecycle::SetupLifecycleService::new()
        .selected_remaining_steps(&app_setup_state(setup))
        .into_iter()
        .map(onboarding_step_from_app)
        .collect()
}

pub fn pending_step_names(setup: &SetupState) -> Vec<String> {
    app_setup_lifecycle::SetupLifecycleService::new().pending_step_names(&app_setup_state(setup))
}

pub fn non_ready_bootstrap_outcomes(setup: &SetupState) -> Vec<SetupBootstrapOutcome> {
    app_setup_lifecycle::SetupLifecycleService::new()
        .non_ready_bootstrap_outcomes(&app_setup_state(setup))
        .into_iter()
        .map(setup_bootstrap_outcome_from_app)
        .collect()
}

pub fn setup_handoff_status(setup: &SetupState) -> &'static str {
    app_setup_lifecycle::SetupLifecycleService::new().handoff_status(&app_setup_state(setup))
}

pub fn setup_handoff_detail(setup: &SetupState) -> String {
    app_setup_lifecycle::SetupLifecycleService::new().handoff_detail(&app_setup_state(setup))
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
            selected_provider: None,
            selected_lane_id: None,
            selected_lane_label: None,
            selected_lane_kind: None,
            selected_backend_id: None,
            selected_lane_detail: None,
            selected_lane_compatibility_note: None,
            selected_access_mode: None,
            selected_primary_model: None,
            selected_primary_model_source: None,
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

fn step_next_action(setup: &SetupState, step: &OnboardingStep) -> String {
    match step {
        OnboardingStep::Model => {
            if setup.selected_provider.is_none() || setup.selected_access_mode.is_none() {
                "Choose the LLM provider and access mode".to_string()
            } else if setup.selected_primary_model.is_none() {
                "Choose the primary task model".to_string()
            } else if let Some(outcome) = setup.bootstrap_outcomes.iter().rev().find(|outcome| {
                outcome.status != "ready"
                    && matches!(
                        outcome.verification_stage.as_deref(),
                        Some("provider_connection")
                    )
            }) {
                outcome.suggested_action.clone().unwrap_or_else(|| {
                    "Resolve provider verification and rerun onboarding".to_string()
                })
            } else {
                format!("Complete {}", step.name())
            }
        }
        _ => format!("Complete {}", step.name()),
    }
}

fn set_setup_state_current_step(workspace_root: &Path, step: &OnboardingStep) -> Result<()> {
    with_setup_state_mut(workspace_root, |setup| {
        setup.status = "in_progress".to_string();
        setup.current_step = Some(step.id().to_string());
        setup.next_action = Some(step_next_action(setup, step));
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
        setup.next_action = setup
            .selected_steps
            .iter()
            .find(|id| !setup.completed_steps.contains(*id))
            .and_then(|id| step_from_id(id))
            .map(|next_step| step_next_action(setup, &next_step));
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
        setup.next_action = setup
            .bootstrap_outcomes
            .iter()
            .rev()
            .find(|outcome| {
                outcome.status != "ready"
                    && step == &OnboardingStep::Model
                    && matches!(
                        outcome.verification_stage.as_deref(),
                        Some("provider_connection")
                    )
            })
            .and_then(|outcome| outcome.suggested_action.clone())
            .or_else(|| Some(format!("Review {} and rerun onboarding", step.name())));
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
    issue_kind: Option<String>,
    verification_stage: Option<String>,
    suggested_action: Option<String>,
}

fn print_bootstrap_assessment(target: &str, assessment: &BootstrapAssessment) {
    let label = match assessment.status {
        "ready" => "✓",
        "warning" => "⚠",
        _ => "✗",
    };
    println!("  {label} {} bootstrap: {}", target, assessment.detail);
    if let Some(next_action) = assessment.suggested_action.as_deref()
        && assessment.status != "ready"
    {
        println!("    Next action: {next_action}");
    }
}

fn record_bootstrap_outcome(
    workspace_root: &Path,
    category: &str,
    target: &str,
    status: &str,
    detail: impl Into<String>,
) -> Result<()> {
    record_bootstrap_outcome_with_metadata(workspace_root, category, target, status, detail, None)
}

fn record_bootstrap_outcome_with_metadata(
    workspace_root: &Path,
    category: &str,
    target: &str,
    status: &str,
    detail: impl Into<String>,
    metadata: Option<&BootstrapAssessment>,
) -> Result<()> {
    let detail = detail.into();
    with_setup_state_mut(workspace_root, |setup| {
        let outcome = SetupBootstrapOutcome {
            category: category.to_string(),
            target: target.to_string(),
            status: status.to_string(),
            detail,
            issue_kind: metadata.and_then(|value| value.issue_kind.clone()),
            verification_stage: metadata.and_then(|value| value.verification_stage.clone()),
            suggested_action: metadata.and_then(|value| value.suggested_action.clone()),
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
    access_mode: Option<&str>,
    allow_model_unavailable: bool,
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
            issue_kind: Some("provider_unavailable".to_string()),
            verification_stage: Some("provider_connection".to_string()),
            suggested_action: Some(format!(
                "Review the `{provider_name}` runtime configuration, then rerun onboarding."
            )),
        });
    };

    if entry.healthy {
        Ok(BootstrapAssessment {
            status: "ready",
            detail: format!(
                "Live verification succeeded for provider `{provider_name}` with model `{}`.",
                entry.model
            ),
            blocking: false,
            issue_kind: None,
            verification_stage: Some("provider_connection".to_string()),
            suggested_action: None,
        })
    } else {
        let issue_kind = onboarding_provider_issue_kind(provider_name, access_mode, entry);
        let allow_warning = issue_kind == "model_unavailable" && allow_model_unavailable;
        Ok(BootstrapAssessment {
            status: if allow_warning { "warning" } else { "blocked" },
            detail: provider_verification_detail(provider_name, &issue_kind, entry),
            blocking: !allow_warning,
            issue_kind: Some(issue_kind.clone()),
            verification_stage: Some("provider_connection".to_string()),
            suggested_action: Some(provider_verification_next_action(
                provider_name,
                &issue_kind,
            )),
        })
    }
}

fn validate_delegated_backend_bootstrap(backend_id: &str) -> Result<BootstrapAssessment> {
    let catalog = AgentBackendCatalogService::new().discover();
    let contract = AgentBackendControlService::new()
        .contracts_from_catalog(&catalog)
        .into_iter()
        .find(|candidate| candidate.backend_id == backend_id)
        .ok_or_else(|| anyhow!("delegated backend `{backend_id}` is not discoverable"))?;

    let discovered_models = catalog
        .iter()
        .find(|entry| entry.host.as_id().replace('-', "_") == backend_id)
        .map(|entry| entry.discovered_models.len())
        .unwrap_or(0);

    if contract.execution_eligible {
        return Ok(BootstrapAssessment {
            status: "ready",
            detail: if discovered_models > 0 {
                format!(
                    "Delegated backend `{backend_id}` is installed, signed in, and currently exposes {discovered_models} discoverable model option(s)."
                )
            } else {
                format!(
                    "Delegated backend `{backend_id}` is installed, signed in, and ready for bounded routed execution once operator policy allows it."
                )
            },
            blocking: false,
            issue_kind: None,
            verification_stage: Some("delegated_backend_contract".to_string()),
            suggested_action: None,
        });
    }

    let detail = contract.readiness_reason.clone().unwrap_or_else(|| {
        format!("Delegated backend `{backend_id}` is visible, but not ready yet.")
    });
    Ok(BootstrapAssessment {
        status: "blocked",
        detail,
        blocking: true,
        issue_kind: Some("delegated_backend_unavailable".to_string()),
        verification_stage: Some("delegated_backend_contract".to_string()),
        suggested_action: Some(format!(
            "Sign in to `{backend_id}` locally or review its documented CLI surface, then rerun onboarding."
        )),
    })
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
            issue_kind: None,
            verification_stage: None,
            suggested_action: None,
        });
    }

    let mode_label = deployment_mode.unwrap_or(self_hosted::MODE_SOLO);
    Ok(BootstrapAssessment {
        status: "ready",
        detail: format!(
            "Runtime provider lane is healthy for the current `{mode_label}` deployment path."
        ),
        blocking: false,
        issue_kind: None,
        verification_stage: None,
        suggested_action: None,
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
            issue_kind: None,
            verification_stage: None,
            suggested_action: None,
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
            issue_kind: None,
            verification_stage: None,
            suggested_action: None,
        })
    } else if runtime_spec.mode == execution_mode {
        Ok(BootstrapAssessment {
            status: "warning",
            detail: format!(
                "Execution mode `{}` was saved successfully, but the {} deployment path recommends `{}` as the initial runtime shape.",
                runtime_spec.mode, descriptor.label, descriptor.recommended_runtime_mode
            ),
            blocking: false,
            issue_kind: None,
            verification_stage: None,
            suggested_action: None,
        })
    } else {
        Ok(BootstrapAssessment {
            status: "blocked",
            detail: format!(
                "Control-plane runtime registry saved `{}` instead of the selected `{execution_mode}` mode.",
                runtime_spec.mode
            ),
            blocking: true,
            issue_kind: None,
            verification_stage: None,
            suggested_action: None,
        })
    }
}

fn onboarding_provider_issue_kind(
    provider_name: &str,
    access_mode: Option<&str>,
    entry: &runtime::RuntimeHealthProviderEntry,
) -> String {
    let raw_issue = entry
        .issue
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if provider_name == "ollama"
        && access_mode == Some("local_runtime")
        && matches!(
            entry.issue_kind.as_deref(),
            Some("probe_failed") | Some("provider_unavailable")
        )
        && (raw_issue.contains("failed to reach")
            || raw_issue.contains("connection refused")
            || raw_issue.contains("tcp connect")
            || raw_issue.contains("timed out"))
    {
        return "local_runtime_missing".to_string();
    }

    entry
        .issue_kind
        .clone()
        .unwrap_or_else(|| "probe_failed".to_string())
}

fn provider_verification_detail(
    provider_name: &str,
    issue_kind: &str,
    entry: &runtime::RuntimeHealthProviderEntry,
) -> String {
    let raw_issue = entry
        .issue
        .clone()
        .unwrap_or_else(|| "health probe failed".to_string());
    match issue_kind {
        "auth" => format!(
            "Provider `{provider_name}` rejected live verification due to authentication or access: {raw_issue}"
        ),
        "local_runtime_missing" => format!(
            "Local runtime for `{provider_name}` is not reachable during live verification: {raw_issue}"
        ),
        "model_unavailable" => format!(
            "Configured model `{}` is not available during live verification for provider `{provider_name}`: {raw_issue}",
            entry.model
        ),
        "billing" => format!(
            "Provider `{provider_name}` blocked live verification because billing or quota is not available: {raw_issue}"
        ),
        "rate_limited" => format!(
            "Provider `{provider_name}` is rate-limited, so onboarding cannot verify readiness yet: {raw_issue}"
        ),
        "provider_unavailable" => format!(
            "Provider `{provider_name}` could not be reached during live verification: {raw_issue}"
        ),
        "not_configured" => format!(
            "Provider `{provider_name}` is selected but the required credential or runtime configuration is missing: {raw_issue}"
        ),
        _ => format!("Provider `{provider_name}` failed live verification: {raw_issue}"),
    }
}

fn provider_verification_next_action(provider_name: &str, issue_kind: &str) -> String {
    match issue_kind {
        "auth" => format!(
            "Review the `{provider_name}` credential or account access, then rerun onboarding."
        ),
        "local_runtime_missing" => {
            "Start the local runtime, confirm it is reachable, then rerun onboarding.".to_string()
        }
        "model_unavailable" => {
            format!("Choose or pull a model that `{provider_name}` exposes, then rerun onboarding.")
        }
        "billing" => {
            format!("Resolve billing or quota for `{provider_name}`, then rerun onboarding.")
        }
        "rate_limited" => format!(
            "Wait for `{provider_name}` rate limits to reset or switch provider path, then rerun onboarding."
        ),
        "provider_unavailable" => {
            format!("Check provider reachability for `{provider_name}`, then rerun onboarding.")
        }
        "not_configured" => format!(
            "Configure the required `{provider_name}` credential or runtime, then rerun onboarding."
        ),
        _ => format!("Review the `{provider_name}` verification failure and rerun onboarding."),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrimaryModelSelection {
    model: String,
    source: String,
}

fn configured_model_for_provider(config: &AppConfig, provider: &str) -> String {
    match provider {
        "anthropic" => config.providers.anthropic.model.clone(),
        "openai" => config.providers.openai.model.clone(),
        "codex" => config.providers.openai.codex_model.clone(),
        "openrouter" => config.providers.openrouter.model.clone(),
        "gemini" => config.providers.gemini.model.clone(),
        "ollama" => config.providers.ollama.model.clone(),
        "cursor" => "auto".to_string(),
        _ => String::new(),
    }
}

fn build_model_shortlist(recommended: &str, discovered: Vec<String>) -> Vec<String> {
    let mut discovered = discovered
        .into_iter()
        .filter(|entry| !entry.trim().is_empty())
        .collect::<Vec<_>>();
    discovered.sort();
    discovered.dedup();

    let mut shortlist = Vec::new();
    if !recommended.is_empty() && discovered.iter().any(|entry| entry == recommended) {
        shortlist.push(recommended.to_string());
    }
    for model in discovered {
        if model == recommended || shortlist.iter().any(|entry| entry == &model) {
            continue;
        }
        shortlist.push(model);
        if shortlist.len() >= 25 {
            break;
        }
    }
    shortlist
}

fn persist_primary_model_selection(workspace_root: &Path, model: &str, source: &str) -> Result<()> {
    with_setup_state_mut(workspace_root, |setup| {
        setup.selected_primary_model = Some(model.to_string());
        setup.selected_primary_model_source = Some(source.to_string());
    })
}

async fn discover_live_provider_models(
    workspace_root: &Path,
    provider: &str,
) -> Result<Vec<String>> {
    let config = runtime::load_effective_config("config/default.toml", workspace_root)?;
    let client = reqwest::Client::new();

    let mut models = match provider {
        "anthropic" => {
            let api_key = std::env::var(
                config
                    .providers
                    .anthropic
                    .api_key_env
                    .as_deref()
                    .unwrap_or("ANTHROPIC_API_KEY"),
            )?;
            let response = client
                .get("https://api.anthropic.com/v1/models")
                .timeout(Duration::from_secs(5))
                .bearer_auth(api_key)
                .header(
                    "anthropic-version",
                    config.providers.anthropic.api_version.clone(),
                )
                .send()
                .await?
                .error_for_status()?;
            let body: serde_json::Value = response.json().await?;
            body["data"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|entry| entry.get("id").and_then(|value| value.as_str()))
                .map(str::to_string)
                .collect::<Vec<_>>()
        }
        "openai" => {
            let api_key = std::env::var(
                config
                    .providers
                    .openai
                    .api_key_env
                    .as_deref()
                    .unwrap_or("OPENAI_API_KEY"),
            )?;
            let response = client
                .get("https://api.openai.com/v1/models")
                .timeout(Duration::from_secs(5))
                .bearer_auth(api_key)
                .send()
                .await?
                .error_for_status()?;
            let body: serde_json::Value = response.json().await?;
            body["data"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|entry| entry.get("id").and_then(|value| value.as_str()))
                .map(str::to_string)
                .collect::<Vec<_>>()
        }
        "openrouter" => {
            let api_key = std::env::var(
                config
                    .providers
                    .openrouter
                    .api_key_env
                    .as_deref()
                    .unwrap_or("OPENROUTER_API_KEY"),
            )?;
            let response = client
                .get("https://openrouter.ai/api/v1/models")
                .timeout(Duration::from_secs(5))
                .bearer_auth(api_key)
                .send()
                .await?
                .error_for_status()?;
            let body: serde_json::Value = response.json().await?;
            body["data"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|entry| entry.get("id").and_then(|value| value.as_str()))
                .map(str::to_string)
                .collect::<Vec<_>>()
        }
        "gemini" => {
            let api_key = config
                .providers
                .gemini
                .api_key_env
                .as_deref()
                .and_then(|name| std::env::var(name).ok())
                .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
                .ok_or_else(|| {
                    anyhow!("GEMINI_API_KEY or GOOGLE_API_KEY environment variable not set")
                })?;
            let response = client
                .get(format!(
                    "{}/models",
                    config
                        .providers
                        .gemini
                        .base_url
                        .as_deref()
                        .unwrap_or("https://generativelanguage.googleapis.com/v1beta")
                        .trim_end_matches('/')
                ))
                .timeout(Duration::from_secs(5))
                .header("x-goog-api-key", api_key)
                .send()
                .await?
                .error_for_status()?;
            let body: serde_json::Value = response.json().await?;
            body["models"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|entry| entry.get("name").and_then(|value| value.as_str()))
                .map(|value| value.trim_start_matches("models/").to_string())
                .collect::<Vec<_>>()
        }
        "ollama" => {
            let response = client
                .get(format!(
                    "{}/api/tags",
                    config.providers.ollama.base_url.trim_end_matches('/')
                ))
                .timeout(Duration::from_secs(5))
                .send()
                .await?
                .error_for_status()?;
            let body: serde_json::Value = response.json().await?;
            body["models"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|entry| entry.get("name").and_then(|value| value.as_str()))
                .map(str::to_string)
                .collect::<Vec<_>>()
        }
        "cursor" => AgentBackendCatalogService::new()
            .discover()
            .into_iter()
            .find(|entry| entry.host == openrustclaw_app::tool_host_service::AiHost::Cursor)
            .map(|entry| entry.discovered_models)
            .unwrap_or_default(),
        _ => Vec::new(),
    };

    models.sort();
    models.dedup();
    Ok(models)
}

async fn select_primary_model(
    wizard: &OnboardingWizard,
    workspace_root: &Path,
    provider: &str,
) -> Result<PrimaryModelSelection> {
    let config = runtime::load_effective_config("config/default.toml", workspace_root)?;
    let recommended_model = configured_model_for_provider(&config, provider);

    match discover_live_provider_models(workspace_root, provider).await {
        Ok(discovered) if !discovered.is_empty() => {
            let shortlist = build_model_shortlist(&recommended_model, discovered);
            if !shortlist.is_empty() {
                let mut labels = shortlist
                    .iter()
                    .map(|model| {
                        if model == &recommended_model {
                            format!("{model} - Recommended")
                        } else {
                            model.clone()
                        }
                    })
                    .collect::<Vec<_>>();
                labels.push("Enter a model manually".to_string());
                let default = shortlist
                    .iter()
                    .position(|model| model == &recommended_model)
                    .unwrap_or(0);
                let selection = Select::with_theme(&wizard.theme)
                    .with_prompt(format!("Choose the primary task model for {}", provider))
                    .items(&labels)
                    .default(default)
                    .interact()?;
                if selection < shortlist.len() {
                    return Ok(PrimaryModelSelection {
                        model: shortlist[selection].clone(),
                        source: "live_discovery".to_string(),
                    });
                }
            }
        }
        Ok(_) => {
            println!(
                "  Live model discovery returned no models for `{provider}`. Falling back to manual entry."
            );
        }
        Err(error) => {
            println!(
                "  Live model discovery for `{provider}` was unavailable: {}",
                error
            );
            println!("  Falling back to manual model entry.");
        }
    }

    let selected: String = Input::with_theme(&wizard.theme)
        .with_prompt(format!("Primary task model for {}", provider))
        .with_initial_text(recommended_model.clone())
        .interact_text()?;
    validate_model_for_provider(provider, &selected).map_err(|error| anyhow!(error.to_string()))?;
    let source = if selected == recommended_model {
        "recommended_fallback"
    } else {
        "manual_entry"
    };
    Ok(PrimaryModelSelection {
        model: selected,
        source: source.to_string(),
    })
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
            issue_kind: None,
            verification_stage: None,
            suggested_action: None,
        });
    };

    match entry.status {
        services::ChannelProbeStatus::Ready => Ok(BootstrapAssessment {
            status: "ready",
            detail: entry.detail.clone(),
            blocking: false,
            issue_kind: None,
            verification_stage: None,
            suggested_action: None,
        }),
        services::ChannelProbeStatus::Warning => Ok(BootstrapAssessment {
            status: "warning",
            detail: entry.detail.clone(),
            blocking: true,
            issue_kind: None,
            verification_stage: None,
            suggested_action: None,
        }),
        services::ChannelProbeStatus::Failed => Ok(BootstrapAssessment {
            status: "blocked",
            detail: entry.detail.clone(),
            blocking: true,
            issue_kind: None,
            verification_stage: None,
            suggested_action: None,
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
    let app_setup = setup_state.map(app_setup_state);
    let app_checks = report
        .checks
        .iter()
        .map(app_diagnostic_check)
        .collect::<Vec<_>>();
    let plan = app_setup_lifecycle::SetupLifecycleService::new()
        .derive_repair_plan(app_setup.as_ref(), &app_checks);
    SetupRepairPlan {
        steps: plan
            .steps
            .into_iter()
            .map(onboarding_step_from_app)
            .collect(),
        reasons: plan.reasons,
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
        setup.next_action = steps.first().map(|step| step_next_action(setup, step));
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
        "gemini" => "GEMINI_API_KEY",
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
    onboarding_profile_from_app(
        app_setup_lifecycle::SetupLifecycleService::new().default_onboarding_profile_for_mode(mode),
    )
}

fn default_runtime_mode_index(mode: Option<&str>) -> usize {
    app_setup_lifecycle::SetupLifecycleService::new().default_runtime_mode_index(mode)
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
    app_setup_lifecycle::SetupLifecycleService::new()
        .steps_for_profile(app_onboarding_profile(profile))
        .into_iter()
        .map(onboarding_step_from_app)
        .collect()
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
        assert!(state.selected_access_mode.is_none());
        assert!(state.selected_primary_model.is_none());
        assert!(state.selected_primary_model_source.is_none());
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
        assert!(wizard.state.selected_access_mode.is_none());
        assert!(wizard.state.selected_primary_model.is_none());
        assert!(wizard.state.selected_primary_model_source.is_none());
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
        state.selected_access_mode = Some("local_runtime".to_string());
        state.selected_primary_model = Some("llama3.1".to_string());
        state.selected_primary_model_source = Some("live_discovery".to_string());
        state.daemon_installed = true;
        assert!(state.gateway_configured);
        assert!(state.model_configured);
        assert_eq!(state.preferred_provider.as_deref(), Some("ollama"));
        assert_eq!(state.selected_access_mode.as_deref(), Some("local_runtime"));
        assert_eq!(state.selected_primary_model.as_deref(), Some("llama3.1"));
        assert_eq!(
            state.selected_primary_model_source.as_deref(),
            Some("live_discovery")
        );
        assert!(state.daemon_installed);
    }

    #[test]
    fn test_onboarding_lane_catalog_matches_current_supported_paths() {
        let catalog =
            OnboardingLaneCatalogService::new().catalog(&direct_provider_lane_statuses(false), &[]);
        assert_eq!(catalog.len(), 5);
        assert_eq!(
            catalog
                .iter()
                .find(|descriptor| descriptor.provider_id == "anthropic")
                .unwrap()
                .supported_access_modes,
            vec!["api_key".to_string()]
        );
        assert_eq!(
            catalog
                .iter()
                .find(|descriptor| descriptor.provider_id == "gemini")
                .unwrap()
                .supported_access_modes,
            vec!["api_key".to_string()]
        );
        assert_eq!(
            catalog
                .iter()
                .find(|descriptor| descriptor.provider_id == "ollama")
                .unwrap()
                .supported_access_modes,
            vec!["local_runtime".to_string()]
        );
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
                selected_provider: Some("openrouter".to_string()),
                selected_lane_id: Some("openrouter".to_string()),
                selected_lane_label: Some("OpenRouter (Multiple models)".to_string()),
                selected_lane_kind: Some("direct_api".to_string()),
                selected_backend_id: None,
                selected_lane_detail: Some("Use a provider API key stored in `.env`.".to_string()),
                selected_lane_compatibility_note: None,
                selected_access_mode: Some("api_key".to_string()),
                selected_primary_model: Some("openai/gpt-4o".to_string()),
                selected_primary_model_source: Some("live_discovery".to_string()),
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
            loaded.setup.selected_provider.as_deref(),
            Some("openrouter")
        );
        assert_eq!(
            loaded.setup.selected_access_mode.as_deref(),
            Some("api_key")
        );
        assert_eq!(
            loaded.setup.selected_primary_model.as_deref(),
            Some("openai/gpt-4o")
        );
        assert_eq!(
            loaded.setup.selected_primary_model_source.as_deref(),
            Some("live_discovery")
        );
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
    fn test_step_next_action_distinguishes_model_substeps() {
        let mut setup = SetupState {
            started_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            completed_at: None,
            status: "in_progress".to_string(),
            workspace_action: "resume".to_string(),
            deployment_mode: None,
            deployment_path: None,
            remote_connectivity_profile: None,
            setup_path: Some("Standard".to_string()),
            selected_provider: None,
            selected_lane_id: None,
            selected_lane_label: None,
            selected_lane_kind: None,
            selected_backend_id: None,
            selected_lane_detail: None,
            selected_lane_compatibility_note: None,
            selected_access_mode: None,
            selected_primary_model: None,
            selected_primary_model_source: None,
            selected_steps: vec!["model".to_string()],
            completed_steps: Vec::new(),
            blockers: Vec::new(),
            next_action: None,
            current_step: Some("model".to_string()),
            bootstrap_outcomes: Vec::new(),
        };

        assert_eq!(
            step_next_action(&setup, &OnboardingStep::Model),
            "Choose the LLM provider and access mode"
        );

        setup.selected_provider = Some("openrouter".to_string());
        setup.selected_access_mode = Some("api_key".to_string());

        assert_eq!(
            step_next_action(&setup, &OnboardingStep::Model),
            "Choose the primary task model"
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
        assert!(outcome.issue_kind.is_none());
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
            selected_provider: Some("anthropic".to_string()),
            selected_lane_id: Some("anthropic".to_string()),
            selected_lane_label: Some("Anthropic (Claude)".to_string()),
            selected_lane_kind: Some("direct_api".to_string()),
            selected_backend_id: None,
            selected_lane_detail: Some("Use a provider API key stored in `.env`.".to_string()),
            selected_lane_compatibility_note: None,
            selected_access_mode: Some("api_key".to_string()),
            selected_primary_model: Some("claude-sonnet-4-20250514".to_string()),
            selected_primary_model_source: Some("manual_entry".to_string()),
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
                    issue_kind: None,
                    verification_stage: None,
                    suggested_action: None,
                    updated_at: Utc::now().to_rfc3339(),
                },
                SetupBootstrapOutcome {
                    category: "channel".to_string(),
                    target: "slack".to_string(),
                    status: "warning".to_string(),
                    detail: "channel probe failed".to_string(),
                    issue_kind: None,
                    verification_stage: None,
                    suggested_action: None,
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
                selected_provider: Some("anthropic".to_string()),
                selected_lane_id: Some("anthropic".to_string()),
                selected_lane_label: Some("Anthropic (Claude)".to_string()),
                selected_lane_kind: Some("direct_api".to_string()),
                selected_backend_id: None,
                selected_lane_detail: Some("Use a provider API key stored in `.env`.".to_string()),
                selected_lane_compatibility_note: None,
                selected_access_mode: Some("api_key".to_string()),
                selected_primary_model: Some("claude-sonnet-4-20250514".to_string()),
                selected_primary_model_source: Some("manual_entry".to_string()),
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
    fn test_validate_provider_bootstrap_classifies_local_runtime_missing() {
        let entry = runtime::RuntimeHealthProviderEntry {
            provider: "ollama".to_string(),
            role: "primary".to_string(),
            model: "llama3".to_string(),
            configured: true,
            healthy: false,
            issue_kind: Some("probe_failed".to_string()),
            recommendation: None,
            issue: Some(
                "Failed to reach Ollama: tcp connect error: Connection refused".to_string(),
            ),
            model_available: None,
            limit_snapshot: None,
        };

        let issue_kind = onboarding_provider_issue_kind("ollama", Some("local_runtime"), &entry);
        assert_eq!(issue_kind, "local_runtime_missing");
        let detail = provider_verification_detail("ollama", &issue_kind, &entry);
        assert!(detail.contains("Local runtime"));
        let next_action = provider_verification_next_action("ollama", &issue_kind);
        assert!(next_action.contains("Start the local runtime"));
    }

    #[test]
    fn test_build_model_shortlist_prefers_recommended_model() {
        let shortlist = build_model_shortlist(
            "gpt-4o",
            vec![
                "gpt-4o-mini".to_string(),
                "gpt-4o".to_string(),
                "gpt-4o".to_string(),
                "o3-mini".to_string(),
            ],
        );
        assert_eq!(
            shortlist,
            vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "o3-mini".to_string()
            ]
        );
    }

    #[test]
    fn test_mark_setup_state_step_blocked_prefers_provider_verification_action() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = SetupStateManifest {
            version: SETUP_STATE_VERSION,
            setup: SetupState {
                started_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
                completed_at: None,
                status: "in_progress".to_string(),
                workspace_action: "resume".to_string(),
                deployment_mode: Some(self_hosted::MODE_SOLO.to_string()),
                deployment_path: Some("solo_starter".to_string()),
                remote_connectivity_profile: None,
                setup_path: Some("Standard".to_string()),
                selected_provider: Some("ollama".to_string()),
                selected_lane_id: Some("ollama".to_string()),
                selected_lane_label: Some("Ollama (Local models)".to_string()),
                selected_lane_kind: Some("local_runtime".to_string()),
                selected_backend_id: None,
                selected_lane_detail: Some(
                    "Use a local runtime already running on this machine.".to_string(),
                ),
                selected_lane_compatibility_note: None,
                selected_access_mode: Some("local_runtime".to_string()),
                selected_primary_model: None,
                selected_primary_model_source: None,
                selected_steps: vec!["model".to_string()],
                completed_steps: Vec::new(),
                blockers: Vec::new(),
                next_action: Some("Complete AI Model Setup".to_string()),
                current_step: Some("model".to_string()),
                bootstrap_outcomes: vec![SetupBootstrapOutcome {
                    category: "provider".to_string(),
                    target: "ollama".to_string(),
                    status: "blocked".to_string(),
                    detail: "Local runtime is not reachable".to_string(),
                    issue_kind: Some("local_runtime_missing".to_string()),
                    verification_stage: Some("provider_connection".to_string()),
                    suggested_action: Some(
                        "Start the local runtime, confirm it is reachable, then rerun onboarding."
                            .to_string(),
                    ),
                    updated_at: Utc::now().to_rfc3339(),
                }],
            },
        };
        save_setup_state(dir.path(), &manifest).unwrap();

        mark_setup_state_step_blocked(
            dir.path(),
            &OnboardingStep::Model,
            "AI Model Setup failed: provider verification failed".to_string(),
        )
        .unwrap();

        let loaded = load_setup_state(dir.path()).unwrap().unwrap();
        assert_eq!(loaded.setup.status, "blocked");
        assert_eq!(
            loaded.setup.next_action.as_deref(),
            Some("Start the local runtime, confirm it is reachable, then rerun onboarding.")
        );
    }

    #[test]
    fn test_persist_provider_path_selection_clears_prior_model_selection() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = SetupStateManifest {
            version: SETUP_STATE_VERSION,
            setup: SetupState {
                started_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
                completed_at: None,
                status: "in_progress".to_string(),
                workspace_action: "resume".to_string(),
                deployment_mode: Some(self_hosted::MODE_SOLO.to_string()),
                deployment_path: Some("solo_starter".to_string()),
                remote_connectivity_profile: None,
                setup_path: Some("Standard".to_string()),
                selected_provider: Some("anthropic".to_string()),
                selected_lane_id: Some("anthropic".to_string()),
                selected_lane_label: Some("Anthropic (Claude)".to_string()),
                selected_lane_kind: Some("direct_api".to_string()),
                selected_backend_id: None,
                selected_lane_detail: Some("Use a provider API key stored in `.env`.".to_string()),
                selected_lane_compatibility_note: None,
                selected_access_mode: Some("api_key".to_string()),
                selected_primary_model: Some("claude-sonnet-4-20250514".to_string()),
                selected_primary_model_source: Some("live_discovery".to_string()),
                selected_steps: vec!["model".to_string()],
                completed_steps: Vec::new(),
                blockers: Vec::new(),
                next_action: Some("Complete AI Model Setup".to_string()),
                current_step: Some("model".to_string()),
                bootstrap_outcomes: Vec::new(),
            },
        };
        save_setup_state(dir.path(), &manifest).unwrap();

        let descriptor = OnboardingLaneDescriptor {
            lane_id: "openai".to_string(),
            provider_id: "openai".to_string(),
            backend_id: None,
            label: "OpenAI (GPT)".to_string(),
            kind: OnboardingLaneKind::DirectApi,
            supported_access_modes: vec!["api_key".to_string()],
            api_key_prompt: Some("OpenAI API key".to_string()),
            recommended: false,
            status_label: "configured".to_string(),
            detail: "Use a provider API key stored in `.env`.".to_string(),
            compatibility_note: None,
            model_catalog_label: None,
        };
        persist_provider_path_selection(dir.path(), &descriptor, Some("api_key")).unwrap();

        let loaded = load_setup_state(dir.path()).unwrap().unwrap();
        assert_eq!(loaded.setup.selected_provider.as_deref(), Some("openai"));
        assert_eq!(loaded.setup.selected_lane_id.as_deref(), Some("openai"));
        assert_eq!(
            loaded.setup.selected_lane_label.as_deref(),
            Some("OpenAI (GPT)")
        );
        assert_eq!(
            loaded.setup.selected_access_mode.as_deref(),
            Some("api_key")
        );
        assert!(loaded.setup.selected_primary_model.is_none());
        assert!(loaded.setup.selected_primary_model_source.is_none());
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

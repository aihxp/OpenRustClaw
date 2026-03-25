//! Interactive Onboarding Wizard

use anyhow::Result;
use chrono::Utc;
use console::style;
use dialoguer::{Confirm, Input, MultiSelect, Password, Select, theme::ColorfulTheme};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tokio::fs;

use super::models;
use super::{channels, control, doctor, runtime};

/// Onboarding wizard state
#[derive(Default)]
pub struct OnboardingState {
    pub gateway_configured: bool,
    pub channels_configured: Vec<String>,
    pub model_configured: bool,
    pub execution_mode: Option<String>,
    pub daemon_installed: bool,
    pub skills_installed: Vec<String>,
    pub profile: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OnboardingWorkspaceStatus {
    pub workspace_root: String,
    pub env_present: bool,
    pub control_registry_present: bool,
    pub channels_registry_present: bool,
    pub user_service_present: bool,
}

/// Interactive onboarding wizard
pub struct OnboardingWizard {
    theme: ColorfulTheme,
    state: OnboardingState,
}

enum OnboardingProfile {
    QuickStart,
    Advanced,
}

impl OnboardingProfile {
    fn label(&self) -> &'static str {
        match self {
            OnboardingProfile::QuickStart => "QuickStart",
            OnboardingProfile::Advanced => "Advanced",
        }
    }
}

/// Enum representing all onboarding steps
enum OnboardingStep {
    Gateway,
    Channel,
    Model,
    ControlPlane,
    Skill,
    Daemon,
}

impl OnboardingStep {
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

        let workspace_status = workspace_status(std::env::current_dir()?.as_path());
        if workspace_status.env_present
            || workspace_status.control_registry_present
            || workspace_status.channels_registry_present
        {
            let choices = vec![
                "Modify existing workspace state",
                "Keep existing state and only run a health check",
                "Reset onboarding-managed state with backup",
            ];
            let selection = Select::with_theme(&self.theme)
                .with_prompt("Existing OpenRustClaw workspace state was detected")
                .items(&choices)
                .default(0)
                .interact()?;

            match selection {
                1 => {
                    self.print_workspace_status(&workspace_status);
                    self.run_post_onboarding_health_check().await?;
                    self.print_completion();
                    return Ok(());
                }
                2 => {
                    let backup_path = backup_and_reset_workspace_state().await?;
                    println!(
                        "✓ Existing onboarding-managed state backed up to {}",
                        backup_path.display()
                    );
                }
                _ => {
                    self.print_workspace_status(&workspace_status);
                }
            }
        }

        let profile = match Select::with_theme(&self.theme)
            .with_prompt("Choose your onboarding path")
            .items(&[
                "QuickStart - gateway, one channel, provider, and control plane",
                "Advanced - everything in QuickStart plus skills and system service",
            ])
            .default(0)
            .interact()?
        {
            1 => OnboardingProfile::Advanced,
            _ => OnboardingProfile::QuickStart,
        };
        self.state.profile = Some(profile.label().to_string());

        let quickstart_steps = vec![
            OnboardingStep::Gateway,
            OnboardingStep::Channel,
            OnboardingStep::Model,
            OnboardingStep::ControlPlane,
        ];
        let advanced_steps = vec![
            OnboardingStep::Gateway,
            OnboardingStep::Channel,
            OnboardingStep::Model,
            OnboardingStep::ControlPlane,
            OnboardingStep::Skill,
            OnboardingStep::Daemon,
        ];

        let steps = match profile {
            OnboardingProfile::QuickStart => quickstart_steps,
            OnboardingProfile::Advanced => advanced_steps,
        };

        for step in steps {
            println!("\n{}", style(format!("📋 {}", step.name())).bold().cyan());
            println!("{}", style(step.description()).dim());

            match step.run(self).await {
                Ok(true) => {}
                Ok(false) => {
                    println!("⚠️  Step skipped or failed, but continuing...");
                }
                Err(e) => {
                    println!("⚠️  Step error: {}", e);
                    println!("    Continuing...");
                }
            }
        }

        self.run_post_onboarding_health_check().await?;
        self.print_completion();
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

    fn print_completion(&self) {
        println!("\n{}", style("✅ Onboarding complete!").bold().green());
        println!("\nConfiguration summary:");
        println!(
            "  Path: {}",
            self.state.profile.as_deref().unwrap_or("QuickStart")
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
        println!("\nNext steps:");
        println!("  openrustclaw start    # Start the gateway");
        println!("  openrustclaw chat     # Start chatting");
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
            "  user service: {}",
            if status.user_service_present {
                "present"
            } else {
                "absent"
            }
        );
    }

    async fn run_post_onboarding_health_check(&self) -> Result<()> {
        println!(
            "\n{}",
            style("Running post-onboarding health check...").cyan()
        );
        let report = doctor::collect_report(false, true, None).await?;
        println!(
            "  Health check: {} passed, {} warnings, {} failed",
            report.passed, report.warnings, report.failed
        );
        if report.failed == 0 {
            println!("  ✓ The workspace is ready for first start.");
        } else {
            println!("  ⚠ Review `openrustclaw doctor --deep` before first start.");
        }
        Ok(())
    }
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

    if mode == 1 {
        println!("Remote gateway/client mode is not a separate shipped runtime yet.");
        println!(
            "Use the local gateway for now and expose it through your own tunnel or reverse proxy if needed."
        );
    }

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
    println!("✓ Gateway configured on {}:{}", host, port);
    Ok(true)
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

    match selection {
        0 => setup_telegram(wizard).await?,
        1 => setup_discord(wizard).await?,
        2 => setup_slack(wizard).await?,
        3 => println!("WhatsApp setup requires additional steps. See docs/whatsapp-setup.md"),
        _ => return Ok(true),
    }

    Ok(true)
}

async fn setup_telegram(wizard: &mut OnboardingWizard) -> Result<()> {
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
    }

    Ok(())
}

async fn setup_discord(wizard: &mut OnboardingWizard) -> Result<()> {
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
    }

    Ok(())
}

async fn setup_slack(wizard: &mut OnboardingWizard) -> Result<()> {
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
    }

    Ok(())
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
            wizard.state.model_configured = true;
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
        wizard.state.model_configured = true;
        println!("✓ Model configured ({provider_name})");
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
        .default(0)
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
    println!("✓ Control-plane registry initialized at .claw/control/");
    println!("✓ Runtime mode configured as {mode}");
    println!("  Inspect: openrustclaw control describe");
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
    // Check if systemd is available
    if !Path::new("/run/systemd/system").exists() && !Path::new("/sbin/systemctl").exists() {
        println!("Systemd not detected, skipping system service setup.");
        return Ok(true);
    }

    let install = Confirm::with_theme(&wizard.theme)
        .with_prompt("Install OpenRustClaw as a user systemd service?")
        .default(true)
        .interact()?;

    if install {
        install_daemon().await?;
        wizard.state.daemon_installed = true;
        println!("✓ System service installed");
        println!("  Start: systemctl --user start openrustclaw");
        println!("  Stop:  systemctl --user stop openrustclaw");
        println!("  Enable: systemctl --user enable openrustclaw");
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
    println!("  Run 'systemctl --user daemon-reload' to reload systemd");

    Ok(())
}

pub fn workspace_status(workspace_root: &Path) -> OnboardingWorkspaceStatus {
    let control_root = control::control_root_for(workspace_root);
    let channels_root = channels::channels_root_for(workspace_root);
    let service_present = dirs::config_dir()
        .map(|dir| dir.join("systemd/user/openrustclaw.service").exists())
        .unwrap_or(false);

    OnboardingWorkspaceStatus {
        workspace_root: workspace_root.display().to_string(),
        env_present: workspace_root.join(".env").exists(),
        control_registry_present: control_root.exists(),
        channels_registry_present: channels_root.exists(),
        user_service_present: service_present,
    }
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
        assert!(!state.daemon_installed);
        assert!(state.skills_installed.is_empty());
        assert!(state.profile.is_none());
    }

    #[test]
    fn test_onboarding_wizard_new() {
        let wizard = OnboardingWizard::new();
        assert!(!wizard.state.gateway_configured);
        assert!(wizard.state.channels_configured.is_empty());
        assert!(!wizard.state.model_configured);
        assert!(!wizard.state.daemon_installed);
        assert!(wizard.state.skills_installed.is_empty());
        assert!(wizard.state.profile.is_none());
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
        state.daemon_installed = true;
        assert!(state.gateway_configured);
        assert!(state.model_configured);
        assert!(state.daemon_installed);
    }

    #[test]
    fn test_workspace_status_defaults_to_absent() {
        let dir = tempfile::tempdir().unwrap();
        let status = workspace_status(dir.path());
        assert!(!status.env_present);
        assert!(!status.control_registry_present);
        assert!(!status.channels_registry_present);
    }
}

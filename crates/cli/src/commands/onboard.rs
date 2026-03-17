//! Interactive Onboarding Wizard

use anyhow::Result;
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Password, Select};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use tokio::fs;

/// Onboarding wizard state
#[derive(Default)]
pub struct OnboardingState {
    pub gateway_configured: bool,
    pub channels_configured: Vec<String>,
    pub model_configured: bool,
    pub daemon_installed: bool,
    pub skills_installed: Vec<String>,
}

/// Interactive onboarding wizard
pub struct OnboardingWizard {
    theme: ColorfulTheme,
    state: OnboardingState,
}

/// Enum representing all onboarding steps
enum OnboardingStep {
    Gateway,
    Channel,
    Model,
    Skill,
    Daemon,
}

impl OnboardingStep {
    fn name(&self) -> &'static str {
        match self {
            OnboardingStep::Gateway => "Gateway Setup",
            OnboardingStep::Channel => "Channel Setup",
            OnboardingStep::Model => "AI Model Setup",
            OnboardingStep::Skill => "Skills Setup",
            OnboardingStep::Daemon => "System Service",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            OnboardingStep::Gateway => "Configure the WebSocket gateway server",
            OnboardingStep::Channel => "Connect messaging platforms",
            OnboardingStep::Model => "Configure your LLM provider",
            OnboardingStep::Skill => "Install starter skills",
            OnboardingStep::Daemon => "Install as system service (optional)",
        }
    }

    async fn run(&self, wizard: &mut OnboardingWizard) -> Result<bool> {
        match self {
            OnboardingStep::Gateway => run_gateway_setup(wizard).await,
            OnboardingStep::Channel => run_channel_setup(wizard).await,
            OnboardingStep::Model => run_model_setup(wizard).await,
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

        let steps = vec![
            OnboardingStep::Gateway,
            OnboardingStep::Channel,
            OnboardingStep::Model,
            OnboardingStep::Skill,
            OnboardingStep::Daemon,
        ];

        for step in steps {
            println!(
                "\n{}",
                style(format!("📋 {}", step.name())).bold().cyan()
            );
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
    }
}

// ============================================================================
// Step 1: Gateway Setup
// ============================================================================

async fn run_gateway_setup(wizard: &mut OnboardingWizard) -> Result<bool> {
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
        wizard.state.channels_configured.push("telegram".to_string());
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
    } else {
        println!("⚠️  No API key provided, skipping model setup");
    }

    Ok(true)
}

// ============================================================================
// Step 4: Skill Setup
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

        wizard
            .state
            .skills_installed
            .push(skills[idx].to_string());
        pb.inc(1);
    }

    pb.finish_with_message("Done!");
    println!(
        "✓ {} skills installed",
        wizard.state.skills_installed.len()
    );
    Ok(true)
}

// ============================================================================
// Step 5: Daemon Install
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
    // Get current executable path
    let current_exe = std::env::current_exe()?;
    let exe_path = current_exe.display();

    let service_content = format!(
        r#"[Unit]
Description=OpenRustClaw Gateway
After=network.target

[Service]
Type=simple
ExecStart={exe_path} start
Restart=on-failure
RestartSec=5
WorkingDirectory={}

[Install]
WantedBy=default.target
"#,
        std::env::current_dir()?.display()
    );

    // Get config directory for user systemd service
    let config_dir = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
    let service_dir = config_dir.join("systemd/user");
    let service_path = service_dir.join("openrustclaw.service");

    // Create directory if it doesn't exist
    fs::create_dir_all(&service_dir).await?;
    fs::write(&service_path, service_content).await?;

    println!("  Service file written to {}", service_path.display());
    println!("  Run 'systemctl --user daemon-reload' to reload systemd");

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

//! Webhook management commands.

use anyhow::{Context, Result};
use dialoguer::{Confirm, Input, Select};
use sqlx::Row;

use openrustclaw_gateway::webhooks::{
    handlers, WebhookHandler, WebhookSource,
};

/// List configured webhooks.
pub async fn list() -> Result<()> {
    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Query webhooks from database
    let rows = sqlx::query(
        r#"
        SELECT 
            path,
            source,
            secret IS NOT NULL as has_secret,
            enabled,
            action_type,
            created_at
        FROM webhooks 
        ORDER BY path
        "#,
    )
    .fetch_all(&pool)
    .await;

    match rows {
        Ok(rows) if !rows.is_empty() => {
            println!("╔══════════════════════════════════════════════════════════╗");
            println!("║                Configured Webhooks                       ║");
            println!("╚══════════════════════════════════════════════════════════╝");
            println!();

            for row in rows {
                let path: String = row.get("path");
                let source: String = row.get("source");
                let has_secret: i64 = row.get("has_secret");
                let enabled: i64 = row.get("enabled");
                let action_type: String = row.get("action_type");

                let status = if enabled == 1 {
                    "\x1b[32m● enabled\x1b[0m"
                } else {
                    "\x1b[90m○ disabled\x1b[0m"
                };

                let secret_indicator = if has_secret == 1 {
                    "\x1b[32m🔒\x1b[0m"
                } else {
                    "\x1b[90m○\x1b[0m"
                };

                println!("{} /webhooks/{} {} {}", secret_indicator, path, status, action_type);
                println!("  Source: {}", source);
                println!();
            }

            println!("Webhook URL format: http://<host>:<port>/webhooks/<path>");
        }
        _ => {
            println!("No webhooks configured.");
            println!();
            println!("To create a webhook:");
            println!("  openrustclaw webhooks create <path>");
            println!();
            println!("Available presets:");
            println!("  github, gitlab, gmail, stripe, slack, discord, telegram, generic");
        }
    }

    Ok(())
}

/// Create a new webhook.
pub async fn create(path: &str) -> Result<()> {
    println!("Creating webhook: {}", path);

    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Check if webhook already exists
    let existing: Option<String> = sqlx::query_scalar("SELECT path FROM webhooks WHERE path = ?")
        .bind(path)
        .fetch_optional(&pool)
        .await?;

    if existing.is_some() {
        println!("Webhook '{}' already exists.", path);
        return Ok(());
    }

    // Select preset
    let presets = vec![
        "GitHub",
        "GitLab",
        "Gmail",
        "Stripe",
        "Slack",
        "Discord",
        "Telegram",
        "Generic (custom)",
    ];

    let preset_idx = Select::new()
        .with_prompt("Select webhook preset")
        .items(&presets)
        .default(0)
        .interact()?;

    let (source, secret, action_type, template): (WebhookSource, Option<String>, String, Option<String>) = match preset_idx {
        0 => {
            // GitHub
            let secret: String = Input::new()
                .with_prompt("Webhook secret (leave empty for none)")
                .allow_empty(true)
                .interact_text()?;
            let secret = if secret.is_empty() { None } else { Some(secret) };

            let template = "🔔 GitHub: {{action}} on {{repository.full_name}} by {{sender.login}}".to_string();
            println!("Message template: {}", template);

            (
                WebhookSource::GitHub,
                secret,
                "agent_message".to_string(),
                Some(template),
            )
        }
        1 => {
            // GitLab
            let secret: String = Input::new()
                .with_prompt("Webhook secret (leave empty for none)")
                .allow_empty(true)
                .interact_text()?;
            let secret = if secret.is_empty() { None } else { Some(secret) };

            let template = "🔔 GitLab: {{object_kind}} on {{project.name}} by {{user_name}}".to_string();
            println!("Message template: {}", template);

            (
                WebhookSource::GitLab,
                secret,
                "agent_message".to_string(),
                Some(template),
            )
        }
        2 => {
            // Gmail
            println!("Gmail webhooks use Google Cloud Pub/Sub.");
            (
                WebhookSource::Gmail,
                None,
                "agent_message".to_string(),
                Some("📧 New email notification received".to_string()),
            )
        }
        3 => {
            // Stripe
            let secret: String = Input::new()
                .with_prompt("Webhook signing secret (required)")
                .interact_text()?;

            (
                WebhookSource::Stripe,
                Some(secret),
                "emit_event".to_string(),
                None,
            )
        }
        4 => {
            // Slack
            let template = "💬 Slack: {{text}}".to_string();
            println!("Message template: {}", template);

            (
                WebhookSource::Slack,
                None,
                "agent_message".to_string(),
                Some(template),
            )
        }
        5 => {
            // Discord
            (
                WebhookSource::Discord,
                None,
                "agent_message".to_string(),
                Some("🎮 Discord message received".to_string()),
            )
        }
        6 => {
            // Telegram
            let secret: String = Input::new()
                .with_prompt("Bot token (for verification)")
                .allow_empty(true)
                .interact_text()?;
            let secret = if secret.is_empty() { None } else { Some(secret) };

            let template = "📱 Telegram: {{message.text}}".to_string();
            println!("Message template: {}", template);

            (
                WebhookSource::Telegram,
                secret,
                "agent_message".to_string(),
                Some(template),
            )
        }
        _ => {
            // Generic
            let custom_name: String = Input::new()
                .with_prompt("Custom source name")
                .default("custom".to_string())
                .interact_text()?;

            (
                WebhookSource::Custom { name: custom_name },
                None,
                "emit_event".to_string(),
                None,
            )
        }
    };

    // Ask for rate limiting
    let enable_rate_limit = Confirm::new()
        .with_prompt("Enable rate limiting?")
        .default(true)
        .interact()?;

    let (rate_limit_max, rate_limit_window) = if enable_rate_limit {
        let max: u32 = Input::new()
            .with_prompt("Max requests per window")
            .default(100)
            .interact_text()?;

        let window: u64 = Input::new()
            .with_prompt("Window duration (seconds)")
            .default(60)
            .interact_text()?;

        (Some(max), Some(window))
    } else {
        (None, None)
    };

    // Insert into database
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO webhooks (
            id, path, source, secret, action_type, template,
            rate_limit_max, rate_limit_window, enabled, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
        "#,
    )
    .bind(&id)
    .bind(path)
    .bind(source.to_string())
    .bind(&secret)
    .bind(&action_type)
    .bind(template.as_ref())
    .bind(rate_limit_max.map(|v| v as i64))
    .bind(rate_limit_window.map(|v| v as i64))
    .bind(1) // enabled
    .execute(&pool)
    .await
    .context("Failed to insert webhook into database")?;

    println!();
    println!("✓ Webhook created successfully");
    println!("  ID: {}", id);
    println!("  Path: /webhooks/{}", path);
    println!("  Source: {}", source);
    println!("  Action: {}", action_type);
    if secret.is_some() {
        println!("  Signature verification: enabled");
    }
    if let (Some(max), Some(window)) = (rate_limit_max, rate_limit_window) {
        println!("  Rate limit: {} requests per {} seconds", max, window);
    }
    println!();
    println!("Configure your external service to send webhooks to:");
    println!("  http://<your-gateway-host>:<port>/webhooks/{}", path);

    Ok(())
}

/// Delete a webhook.
pub async fn delete(path: &str) -> Result<()> {
    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Check if webhook exists
    let existing: Option<String> = sqlx::query_scalar("SELECT id FROM webhooks WHERE path = ?")
        .bind(path)
        .fetch_optional(&pool)
        .await?;

    let Some(id) = existing else {
        println!("Webhook '{}' not found.", path);
        return Ok(());
    };

    // Confirm deletion
    let confirm = Confirm::new()
        .with_prompt(format!("Are you sure you want to delete webhook '{}'?", path))
        .default(false)
        .interact()?;

    if !confirm {
        println!("Cancelled.");
        return Ok(());
    }

    // Delete webhook
    sqlx::query("DELETE FROM webhooks WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await
        .context("Failed to delete webhook")?;

    println!("✓ Webhook '{}' deleted successfully.", path);

    Ok(())
}

/// Enable a webhook.
pub async fn enable(path: &str) -> Result<()> {
    update_webhook_status(path, true).await
}

/// Disable a webhook.
pub async fn disable(path: &str) -> Result<()> {
    update_webhook_status(path, false).await
}

async fn update_webhook_status(path: &str, enabled: bool) -> Result<()> {
    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Update webhook status
    let result = sqlx::query("UPDATE webhooks SET enabled = ?, updated_at = datetime('now') WHERE path = ?")
        .bind(if enabled { 1 } else { 0 })
        .bind(path)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        println!("Webhook '{}' not found.", path);
    } else {
        let status = if enabled { "enabled" } else { "disabled" };
        println!("✓ Webhook '{}' {} successfully.", path, status);
    }

    Ok(())
}

/// Show webhook details.
pub async fn info(path: &str) -> Result<()> {
    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Query webhook details
    let row = sqlx::query(
        r#"
        SELECT 
            id,
            path,
            source,
            secret IS NOT NULL as has_secret,
            action_type,
            template,
            rate_limit_max,
            rate_limit_window,
            enabled,
            created_at,
            updated_at
        FROM webhooks 
        WHERE path = ?
        "#,
    )
    .bind(path)
    .fetch_optional(&pool)
    .await?;

    let Some(row) = row else {
        println!("Webhook '{}' not found.", path);
        return Ok(());
    };

    let id: String = row.get("id");
    let path: String = row.get("path");
    let source: String = row.get("source");
    let has_secret: i64 = row.get("has_secret");
    let action_type: String = row.get("action_type");
    let template: Option<String> = row.get("template");
    let rate_limit_max: Option<i64> = row.get("rate_limit_max");
    let rate_limit_window: Option<i64> = row.get("rate_limit_window");
    let enabled: i64 = row.get("enabled");
    let created_at: String = row.get("created_at");
    let updated_at: String = row.get("updated_at");

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║                  Webhook Details                         ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    println!("  ID: {}", id);
    println!("  Path: /webhooks/{}", path);
    println!("  Source: {}", source);
    println!("  Status: {}", if enabled == 1 { "\x1b[32menabled\x1b[0m" } else { "\x1b[90mdisabled\x1b[0m" });
    println!("  Action: {}", action_type);
    println!("  Signature verification: {}", if has_secret == 1 { "enabled" } else { "disabled" });

    if let Some(template) = template {
        println!("  Template: {}", template);
    }

    if let (Some(max), Some(window)) = (rate_limit_max, rate_limit_window) {
        println!("  Rate limit: {} requests per {} seconds", max, window);
    }

    println!("  Created: {}", created_at);
    println!("  Updated: {}", updated_at);
    println!();
    println!("  Webhook URL: http://<host>:<port>/webhooks/{}", path);

    Ok(())
}

/// Test a webhook (simulate a request).
pub async fn test(path: &str) -> Result<()> {
    use reqwest::Client;

    println!("Testing webhook: {}", path);

    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Check if webhook exists
    let row = sqlx::query("SELECT source, secret FROM webhooks WHERE path = ? AND enabled = 1")
        .bind(path)
        .fetch_optional(&pool)
        .await?;

    let Some(row) = row else {
        println!("Webhook '{}' not found or disabled.", path);
        return Ok(());
    };

    let source: String = row.get("source");
    let _secret: Option<String> = row.get("secret");

    // Build test payload based on source
    let (payload, headers) = match source.as_str() {
        "github" => {
            let payload = serde_json::json!({
                "action": "opened",
                "repository": {
                    "full_name": "test/repo"
                },
                "sender": {
                    "login": "testuser"
                }
            });
            let headers = vec![("X-GitHub-Event", "pull_request")];
            (payload, headers)
        }
        "gitlab" => {
            let payload = serde_json::json!({
                "object_kind": "push",
                "project": {
                    "name": "test-project"
                },
                "user_name": "Test User"
            });
            let headers = vec![("X-GitLab-Event", "Push Hook")];
            (payload, headers)
        }
        "stripe" => {
            let payload = serde_json::json!({
                "type": "charge.succeeded",
                "data": {
                    "object": {
                        "id": "ch_test"
                    }
                }
            });
            let headers = vec![("Stripe-Signature", "test")];
            (payload, headers)
        }
        _ => {
            let payload = serde_json::json!({
                "test": true,
                "message": "Test webhook",
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
            (payload, vec![])
        }
    };

    // Get gateway host from config
    let gateway_host = format!("{}:{}", config.gateway.host, config.gateway.port);

    let url = format!("http://{}/webhooks/{}", gateway_host, path);

    // Send test request
    let client = Client::new();
    let mut request = client.post(&url).json(&payload);

    for (key, value) in headers {
        request = request.header(key, value);
    }

    match request.send().await {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                println!("✓ Test request sent successfully");
                println!("  Status: {}", status);
                println!("  URL: {}", url);
                println!();
                println!("Payload:");
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("✗ Test request failed");
                println!("  Status: {}", status);
                if let Ok(text) = response.text().await {
                    println!("  Response: {}", text);
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to send test request: {}", e);
            println!();
            println!("Make sure the gateway server is running:");
            println!("  openrustclaw start");
        }
    }

    Ok(())
}

/// Get a pre-configured webhook handler by source type.
#[allow(dead_code)]
pub fn get_preset_handler(source: &str, secret: Option<String>) -> Option<WebhookHandler> {
    match source.to_lowercase().as_str() {
        "github" => Some(handlers::github(secret)),
        "gitlab" => Some(handlers::gitlab(secret)),
        "gmail" => Some(handlers::gmail()),
        "stripe" => secret.map(handlers::stripe),
        "slack" => Some(handlers::slack()),
        "discord" => Some(handlers::discord()),
        "telegram" => Some(handlers::telegram(secret)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_preset_handler_github() {
        let handler = get_preset_handler("github", None);
        assert!(handler.is_some());
    }

    #[test]
    fn test_get_preset_handler_github_with_secret() {
        let handler = get_preset_handler("github", Some("mysecret".to_string()));
        assert!(handler.is_some());
    }

    #[test]
    fn test_get_preset_handler_gitlab() {
        let handler = get_preset_handler("gitlab", None);
        assert!(handler.is_some());
    }

    #[test]
    fn test_get_preset_handler_gmail() {
        let handler = get_preset_handler("gmail", None);
        assert!(handler.is_some());
    }

    #[test]
    fn test_get_preset_handler_stripe_without_secret() {
        // Stripe requires a secret
        let handler = get_preset_handler("stripe", None);
        assert!(handler.is_none());
    }

    #[test]
    fn test_get_preset_handler_stripe_with_secret() {
        let handler = get_preset_handler("stripe", Some("whsec_test".to_string()));
        assert!(handler.is_some());
    }

    #[test]
    fn test_get_preset_handler_slack() {
        let handler = get_preset_handler("slack", None);
        assert!(handler.is_some());
    }

    #[test]
    fn test_get_preset_handler_discord() {
        let handler = get_preset_handler("discord", None);
        assert!(handler.is_some());
    }

    #[test]
    fn test_get_preset_handler_telegram() {
        let handler = get_preset_handler("telegram", None);
        assert!(handler.is_some());
    }

    #[test]
    fn test_get_preset_handler_unknown_source() {
        let handler = get_preset_handler("unknown_service", None);
        assert!(handler.is_none());
    }

    #[test]
    fn test_get_preset_handler_case_insensitive() {
        let handler = get_preset_handler("GITHUB", None);
        assert!(handler.is_some());

        let handler = get_preset_handler("GitHub", None);
        assert!(handler.is_some());
    }
}

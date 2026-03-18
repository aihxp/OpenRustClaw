//! Skill management commands.

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use sqlx::Row;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use openrustclaw_security::SkillVerifier;
use openrustclaw_scheduler::DurableEventBus;
use openrustclaw_skills::{ClawHubRegistry, SearchFilters, SortBy, normalize_capability_names};

fn parse_hex_bytes(input: &str) -> Result<Vec<u8>> {
    let trimmed = input.trim();
    if trimmed.len() % 2 != 0 {
        anyhow::bail!("hex value must contain an even number of characters");
    }

    let mut bytes = Vec::with_capacity(trimmed.len() / 2);
    let chars: Vec<char> = trimmed.chars().collect();
    for pair in chars.chunks(2) {
        let hi = pair[0]
            .to_digit(16)
            .ok_or_else(|| anyhow::anyhow!("invalid hex character '{}'", pair[0]))?;
        let lo = pair[1]
            .to_digit(16)
            .ok_or_else(|| anyhow::anyhow!("invalid hex character '{}'", pair[1]))?;
        bytes.push(((hi << 4) | lo) as u8);
    }

    Ok(bytes)
}

fn load_configured_skill_verifier(
    config: &openrustclaw_core::config::AppConfig,
    required: bool,
) -> Result<SkillVerifier> {
    let Some(key_hex) = config.security.skill_verifying_key.as_deref() else {
        anyhow::bail!(
            "No skill verifying key configured. Set [security].skill_verifying_key to the Ed25519 public key hex before verifying external skills."
        );
    };

    let key_bytes = parse_hex_bytes(key_hex).context("Invalid skill verifying key hex")?;
    let key_bytes: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("Skill verifying key must be exactly 32 bytes"))?;

    SkillVerifier::new(Some(&key_bytes), required)
        .context("Failed to construct skill verifier from configured key")
}

async fn publish_plugin_event(
    pool: &sqlx::SqlitePool,
    event_name: &str,
    payload: serde_json::Value,
) {
    let event_bus = DurableEventBus::new(pool.clone(), 64);
    if let Err(error) = event_bus
        .publish_named(event_name, "plugin_runtime_event", None, &payload, None)
        .await
    {
        tracing::warn!(error = %error, event_name, "Failed to publish plugin runtime event");
    }
}

fn resolve_workspace_skill_file(name: &str) -> Option<PathBuf> {
    let candidates = [
        Path::new("skills").join(name).join("SKILL.md"),
        Path::new("skills").join(format!("{name}.md")),
    ];

    candidates.into_iter().find(|path| path.exists())
}

fn serialize_capabilities(capabilities: &[String]) -> Result<String> {
    let normalized = normalize_capability_names(capabilities)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("Failed to validate skill capabilities")?;
    serde_json::to_string(&normalized).context("Failed to serialize skill capabilities")
}

fn sensitive_capabilities(capabilities: &[String]) -> Result<Vec<String>> {
    let normalized = normalize_capability_names(capabilities)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("Failed to validate skill capabilities")?;
    let sensitive: HashSet<&str> = ["file_write", "network_access", "shell_exec", "database_access"]
        .into_iter()
        .collect();
    Ok(normalized
        .into_iter()
        .filter(|capability| sensitive.contains(capability.as_str()))
        .collect())
}

fn enforce_external_skill_policy(
    config: &openrustclaw_core::config::AppConfig,
    capabilities: &[String],
    signature: Option<&str>,
    skill_name: &str,
) -> Result<()> {
    let sensitive = sensitive_capabilities(capabilities)?;

    if config.security.skill_signature_required && signature.is_none() {
        anyhow::bail!(
            "Skill '{}' is unsigned and [security].skill_signature_required is enabled",
            skill_name
        );
    }

    if !sensitive.is_empty() && signature.is_none() {
        anyhow::bail!(
            "Skill '{}' requests sensitive capabilities ({}) and must be signed before install",
            skill_name,
            sensitive.join(", ")
        );
    }

    Ok(())
}

async fn upsert_skill_record(
    pool: &sqlx::SqlitePool,
    name: &str,
    description: Option<&str>,
    source: &str,
    version: Option<&str>,
    signature: Option<&str>,
    verified: bool,
    capabilities_json: Option<&str>,
    schema: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO skills (
            id, name, description, source, version, signature,
            verified, enabled, capabilities, schema, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?, ?, datetime('now'), datetime('now'))
        ON CONFLICT(name) DO UPDATE SET
            description = excluded.description,
            source = excluded.source,
            version = excluded.version,
            signature = excluded.signature,
            verified = excluded.verified,
            enabled = 1,
            capabilities = excluded.capabilities,
            schema = excluded.schema,
            updated_at = datetime('now')
        "#,
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(name)
    .bind(description)
    .bind(source)
    .bind(version)
    .bind(signature)
    .bind(if verified { 1 } else { 0 })
    .bind(capabilities_json)
    .bind(schema)
    .execute(pool)
    .await
    .context("Failed to upsert skill record")?;

    Ok(())
}

async fn persist_verified_state(pool: &sqlx::SqlitePool, id: &str, verified: bool) -> Result<()> {
    sqlx::query("UPDATE skills SET verified = ? WHERE id = ?")
        .bind(if verified { 1 } else { 0 })
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to update skill verification state")?;
    Ok(())
}

/// List installed skills from database.
pub async fn list() -> Result<()> {
    // Load configuration to get DB path
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Query skills from database
    let rows = sqlx::query(
        r#"
        SELECT 
            name, 
            description, 
            source, 
            version, 
            verified, 
            enabled,
            created_at
        FROM skills 
        ORDER BY name
        "#,
    )
    .fetch_all(&pool)
    .await
    .context("Failed to query skills from database")?;

    if rows.is_empty() {
        println!("No skills installed.");
        println!();
        println!("To install a skill:");
        println!("  openrustclaw skills install <skill-name>");
        println!();
        println!("To search for skills:");
        println!("  openrustclaw skills search <query>");
        return Ok(());
    }

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║                 Installed Skills                         ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    for row in rows {
        let name: String = row.get("name");
        let description: Option<String> = row.get("description");
        let source: String = row.get("source");
        let version: Option<String> = row.get("version");
        let verified: i64 = row.get("verified");
        let enabled: i64 = row.get("enabled");

        let status = if enabled == 1 {
            "\x1b[32m● enabled\x1b[0m"
        } else {
            "\x1b[90m○ disabled\x1b[0m"
        };

        let verified_icon = if verified == 1 {
            "\x1b[32m✓\x1b[0m"
        } else {
            "\x1b[90m○\x1b[0m"
        };

        println!("{} {} {}", verified_icon, name, status);

        if let Some(desc) = description {
            println!("  {}", desc);
        }

        println!("  Source: {}", source);

        if let Some(ver) = version {
            println!("  Version: {}", ver);
        }

        println!();
    }

    Ok(())
}

/// Search for skills in the ClawHub registry.
pub async fn search(query: &str, category: Option<&str>, sort: &str) -> Result<()> {
    println!("🔍 Searching for: {}", query);

    // Parse sort option
    let sort_by = match sort.to_lowercase().as_str() {
        "downloads" => SortBy::Downloads,
        "rating" => SortBy::Rating,
        "recent" => SortBy::Recent,
        _ => SortBy::Relevance,
    };

    let filters = SearchFilters {
        category: category.map(|s| s.to_string()),
        sort_by,
        min_rating: None,
        verified_only: false,
    };

    // Get registry endpoint from config or use default
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();
    let endpoint = config
        .skills
        .and_then(|s| s.registry_url)
        .unwrap_or_else(|| "https://clawhub.openrustclaw.dev".to_string());

    // Create registry client
    let registry = ClawHubRegistry::new(&endpoint)
        .await
        .context("Failed to connect to ClawHub registry")?;

    // Show progress
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Searching registry...");

    // Search
    let results = registry.search(query, filters).await;

    pb.finish_and_clear();

    match results {
        Ok(skills) => {
            if skills.is_empty() {
                println!("No skills found matching '{}'", query);
                return Ok(());
            }

            println!("╔══════════════════════════════════════════════════════════╗");
            println!("║              Search Results                              ║");
            println!("╚══════════════════════════════════════════════════════════╝");
            println!();
            println!("Found {} skill(s) matching '{}'", skills.len(), query);
            println!();

            for skill in skills {
                let rating = if skill.rating_count > 0 {
                    format!("★ {:.1} ({})", skill.rating, skill.rating_count)
                } else {
                    "★ No ratings".to_string()
                };

                println!("📦 {} \x1b[32mv{}\x1b[0m", skill.name, skill.version);
                println!("   {}", skill.description);
                println!(
                    "   Author: {} | Downloads: {} | {}",
                    skill.author, skill.downloads, rating
                );

                if !skill.categories.is_empty() {
                    println!("   Categories: {}", skill.categories.join(", "));
                }

                println!();
                println!("   Install: openrustclaw skills install {}", skill.name);
                println!();
            }
        }
        Err(e) => {
            println!("⚠ Failed to search registry: {}", e);
            println!();
            println!("You can still install skills from local files:");
            println!("  openrustclaw skills install <skill-name>");
        }
    }

    Ok(())
}

/// Install a skill from marketplace.
pub async fn install(name: &str) -> Result<()> {
    println!("Installing skill: {}", name);

    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Check if skill already exists
    let existing: Option<String> = sqlx::query_scalar("SELECT name FROM skills WHERE name = ?")
        .bind(name)
        .fetch_optional(&pool)
        .await?;

    if existing.is_some() {
        println!("Skill '{}' is already installed.", name);
        println!("Use `openrustclaw skills update {}` to update it.", name);
        println!(
            "Use `openrustclaw skills verify {}` to verify its signature.",
            name
        );
        return Ok(());
    }

    let skills_dir = std::path::Path::new("skills");
    let expected_skill_path = Path::new("skills").join(name).join("SKILL.md");
    if !skills_dir.exists() {
        tokio::fs::create_dir_all(skills_dir).await?;
    }

    if let Some(skill_file) = resolve_workspace_skill_file(name) {
        // Load and parse the skill
        let content = tokio::fs::read_to_string(&skill_file).await?;

        // Parse SKILL.md format
        let metadata = parse_skill_metadata(&content)?;
        let capabilities_json = serialize_capabilities(&metadata.capabilities)?;

        upsert_skill_record(
            &pool,
            &metadata.name,
            metadata.description.as_deref(),
            "workspace",
            metadata.version.as_deref(),
            metadata.signature.as_deref(),
            true,
            Some(&capabilities_json),
            metadata.schema.as_deref(),
        )
        .await?;

        publish_plugin_event(
            &pool,
            "plugin.skill_installed",
            serde_json::json!({
                "name": metadata.name,
                "source": "workspace",
                "capabilities": metadata.capabilities,
            }),
        )
        .await;

        println!("✓ Skill '{}' installed successfully", name);
        println!("  Path: {}", skill_file.display());
        println!("  Source: workspace");
        if !metadata.capabilities.is_empty() {
            println!("  Capabilities: {}", metadata.capabilities.join(", "));
        }
    } else {
        // Try to install from ClawHub registry
        println!("Skill not found locally. Checking ClawHub registry...");

        let endpoint = config
            .skills
            .clone()
            .and_then(|s| s.registry_url)
            .unwrap_or_else(|| "https://clawhub.openrustclaw.dev".to_string());

        match ClawHubRegistry::new(&endpoint).await {
            Ok(registry) => {
                let registry_metadata = registry.get_skill(name).await;
                let registry_metadata = match registry_metadata {
                    Ok(metadata) => metadata,
                    Err(e) => {
                        println!("✗ Failed to fetch skill metadata from ClawHub: {}", e);
                        println!();
                        println!(
                            "To create a new skill locally, add a SKILL.md file to the ./skills directory."
                        );
                        println!("Expected file: {}", Path::new("skills").join(name).join("SKILL.md").display());
                        return Ok(());
                    }
                };

                enforce_external_skill_policy(
                    &config,
                    &registry_metadata.capabilities,
                    registry_metadata.signature.as_deref(),
                    name,
                )?;

                let pb = ProgressBar::new_spinner();
                pb.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner:.green} {msg}")
                        .unwrap(),
                );
                pb.set_message("Downloading from ClawHub...");

                match registry.install(name, None).await {
                    Ok(result) => {
                        pb.finish_and_clear();
                        match result {
                            openrustclaw_skills::InstallResult::AlreadyInstalled => {
                                println!("Skill '{}' is already installed.", name);
                            }
                            openrustclaw_skills::InstallResult::Installed {
                                name,
                                version,
                                path,
                            } => {
                                let capabilities_json =
                                    serialize_capabilities(&registry_metadata.capabilities)?;
                                let version_string = version.to_string();
                                upsert_skill_record(
                                    &pool,
                                    &name,
                                    Some(&registry_metadata.description),
                                    "marketplace",
                                    Some(version_string.as_str()),
                                    registry_metadata.signature.as_deref(),
                                    false,
                                    Some(&capabilities_json),
                                    None,
                                )
                                .await?;

                                publish_plugin_event(
                                    &pool,
                                    "plugin.skill_installed",
                                    serde_json::json!({
                                        "name": name,
                                        "source": "marketplace",
                                        "version": version_string,
                                    }),
                                )
                                .await;

                                println!("✓ Skill '{}' v{} installed successfully", name, version);
                                println!("  Path: {}", path.display());
                            }
                        }
                    }
                    Err(e) => {
                        pb.finish_and_clear();
                        println!("✗ Failed to install from ClawHub: {}", e);
                        println!();
                        println!(
                            "To create a new skill locally, add a SKILL.md file to the ./skills directory."
                        );
                        println!("Expected file: {}", expected_skill_path.display());
                    }
                }
            }
            Err(e) => {
                println!("✗ Failed to connect to ClawHub: {}", e);
                println!();
                println!(
                    "To create a new skill locally, add a SKILL.md file to the ./skills directory."
                );
                println!("Expected file: {}", expected_skill_path.display());
            }
        }
    }

    Ok(())
}

/// Update an installed skill.
pub async fn update(name: &str) -> Result<()> {
    println!("Updating skill: {}", name);

    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Check if skill exists
    let existing: Option<String> = sqlx::query_scalar("SELECT name FROM skills WHERE name = ?")
        .bind(name)
        .fetch_optional(&pool)
        .await?;

    if existing.is_none() {
        println!("Skill '{}' is not installed.", name);
        println!("Use `openrustclaw skills install {}` to install it.", name);
        return Ok(());
    }

    // Try to update via ClawHub registry
    let endpoint = config
        .skills
        .clone()
        .and_then(|s| s.registry_url)
        .unwrap_or_else(|| "https://clawhub.openrustclaw.dev".to_string());

    match ClawHubRegistry::new(&endpoint).await {
        Ok(registry) => {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap(),
            );
            pb.set_message("Checking for updates...");

            match registry.update(name).await {
                Ok(result) => {
                    pb.finish_and_clear();
                    match result {
                        openrustclaw_skills::UpdateResult::UpToDate => {
                            println!("✓ Skill '{}' is already up to date.", name);
                        }
                        openrustclaw_skills::UpdateResult::Updated { from, to } => {
                            println!("✓ Skill '{}' updated from v{} to v{}", name, from, to);

                            let metadata = registry.get_skill(name).await?;
                            enforce_external_skill_policy(
                                &config,
                                &metadata.capabilities,
                                metadata.signature.as_deref(),
                                name,
                            )?;
                            let capabilities_json =
                                serialize_capabilities(&metadata.capabilities)?;
                            let version_string = to.to_string();
                            upsert_skill_record(
                                &pool,
                                name,
                                Some(&metadata.description),
                                "marketplace",
                                Some(version_string.as_str()),
                                metadata.signature.as_deref(),
                                false,
                                Some(&capabilities_json),
                                None,
                            )
                            .await?;

                            publish_plugin_event(
                                &pool,
                                "plugin.skill_updated",
                                serde_json::json!({
                                    "name": name,
                                    "from": from.to_string(),
                                    "to": version_string,
                                }),
                            )
                            .await;
                        }
                    }
                }
                Err(e) => {
                    pb.finish_and_clear();
                    println!("✗ Failed to update: {}", e);
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to connect to ClawHub: {}", e);
        }
    }

    Ok(())
}

/// Uninstall a skill.
pub async fn uninstall(name: &str) -> Result<()> {
    println!("Uninstalling skill: {}", name);

    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Check if skill exists
    let existing: Option<String> = sqlx::query_scalar("SELECT name FROM skills WHERE name = ?")
        .bind(name)
        .fetch_optional(&pool)
        .await?;

    if existing.is_none() {
        println!("Skill '{}' is not installed.", name);
        return Ok(());
    }

    // Try to uninstall via ClawHub registry first
    let endpoint = config
        .skills
        .clone()
        .and_then(|s| s.registry_url)
        .unwrap_or_else(|| "https://clawhub.openrustclaw.dev".to_string());

    if let Ok(registry) = ClawHubRegistry::new(&endpoint).await
        && let Err(e) = registry.uninstall(name).await
    {
        debug!("Failed to uninstall via registry: {}", e);
    }

    // Remove from database
    sqlx::query("DELETE FROM skills WHERE name = ?")
        .bind(name)
        .execute(&pool)
        .await?;

    publish_plugin_event(
        &pool,
        "plugin.skill_uninstalled",
        serde_json::json!({
            "name": name,
        }),
    )
    .await;

    println!("✓ Skill '{}' uninstalled successfully.", name);

    Ok(())
}

/// Verify skill signatures.
pub async fn verify(name: &str) -> Result<()> {
    println!("Verifying skill: {}", name);

    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Get skill from database
    let row = sqlx::query("SELECT id, signature, source, verified FROM skills WHERE name = ?")
        .bind(name)
        .fetch_optional(&pool)
        .await?;

    let Some(row) = row else {
        anyhow::bail!("Skill '{}' not found", name);
    };

    let id: String = row.get("id");
    let signature: Option<String> = row.get("signature");
    let source: String = row.get("source");
    let verified: i64 = row.get("verified");

    // Workspace and bundled skills are exempt from verification
    if source == "workspace" || source == "bundled" {
        println!(
            "✓ Skill '{}' is from {} - verification exempt",
            name, source
        );

        persist_verified_state(&pool, &id, true).await?;
        publish_plugin_event(
            &pool,
            "plugin.skill_verified",
            serde_json::json!({
                "name": name,
                "source": source,
                "verified": true,
                "verification_mode": "exempt",
            }),
        )
        .await;

        return Ok(());
    }

    if verified == 1 {
        println!(
            "✓ Skill '{}' is already verified from its managed install record",
            name
        );
        return Ok(());
    }

    // Load skill content
    let Some(skill_file) = resolve_workspace_skill_file(name) else {
        anyhow::bail!(
            "Skill '{}' is not a workspace skill and no local SKILL.md was found for manual verification",
            name
        );
    };

    let content = tokio::fs::read(&skill_file).await?;

    // External skills require a configured public key for real verification.
    let verifier = load_configured_skill_verifier(&config, true)?;

    // Check if skill has signature
    let Some(sig_hex) = signature else {
        println!("✗ Skill '{}' has no signature", name);
        persist_verified_state(&pool, &id, false).await?;
        return Ok(());
    };

    // Parse signature
    let signature_bytes = hex::decode(&sig_hex).context("Invalid signature format in database")?;

    // Verify
    match verifier.verify(&content, &signature_bytes) {
        Ok(true) => {
            println!("✓ Skill '{}' signature verified successfully", name);
            persist_verified_state(&pool, &id, true).await?;
            publish_plugin_event(
                &pool,
                "plugin.skill_verified",
                serde_json::json!({
                    "name": name,
                    "source": source,
                    "verified": true,
                    "verification_mode": "signature",
                }),
            )
            .await;
        }
        Ok(false) => {
            println!("✗ Skill '{}' signature verification failed", name);
            persist_verified_state(&pool, &id, false).await?;
            publish_plugin_event(
                &pool,
                "plugin.skill_verified",
                serde_json::json!({
                    "name": name,
                    "source": source,
                    "verified": false,
                    "verification_mode": "signature",
                }),
            )
            .await;
        }
        Err(e) => {
            println!("✗ Error verifying skill '{}': {}", name, e);
            persist_verified_state(&pool, &id, false).await?;
            publish_plugin_event(
                &pool,
                "plugin.skill_verified",
                serde_json::json!({
                    "name": name,
                    "source": source,
                    "verified": false,
                    "verification_mode": "error",
                    "error": e.to_string(),
                }),
            )
            .await;
        }
    }

    Ok(())
}

/// Show popular skills from the registry.
pub async fn popular(limit: usize) -> Result<()> {
    println!("🔥 Popular Skills");
    println!();

    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();
    let endpoint = config
        .skills
        .and_then(|s| s.registry_url)
        .unwrap_or_else(|| "https://clawhub.openrustclaw.dev".to_string());

    match ClawHubRegistry::new(&endpoint).await {
        Ok(registry) => {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap(),
            );
            pb.set_message("Fetching popular skills...");

            match registry.get_popular(limit).await {
                Ok(skills) => {
                    pb.finish_and_clear();

                    if skills.is_empty() {
                        println!("No popular skills found.");
                        return Ok(());
                    }

                    println!("╔══════════════════════════════════════════════════════════╗");
                    println!("║              Popular Skills                              ║");
                    println!("╚══════════════════════════════════════════════════════════╝");
                    println!();

                    for (i, skill) in skills.iter().enumerate() {
                        let rating = if skill.rating_count > 0 {
                            format!("★ {:.1}", skill.rating)
                        } else {
                            "★ -".to_string()
                        };

                        println!(
                            "{}. 📦 {} \x1b[32mv{}\x1b[0m",
                            i + 1,
                            skill.name,
                            skill.version
                        );
                        println!("   {}", skill.description);
                        println!(
                            "   {} downloads | {} | {}",
                            skill.downloads, rating, skill.author
                        );
                        println!();
                    }

                    println!("Install a skill with: openrustclaw skills install <name>");
                }
                Err(e) => {
                    pb.finish_and_clear();
                    println!("⚠ Failed to fetch popular skills: {}", e);
                }
            }
        }
        Err(e) => {
            println!("⚠ Failed to connect to ClawHub: {}", e);
        }
    }

    Ok(())
}

/// Show trending skills from the registry.
pub async fn trending(limit: usize) -> Result<()> {
    println!("📈 Trending Skills");
    println!();

    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();
    let endpoint = config
        .skills
        .and_then(|s| s.registry_url)
        .unwrap_or_else(|| "https://clawhub.openrustclaw.dev".to_string());

    match ClawHubRegistry::new(&endpoint).await {
        Ok(registry) => {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap(),
            );
            pb.set_message("Fetching trending skills...");

            match registry.get_trending(limit).await {
                Ok(skills) => {
                    pb.finish_and_clear();

                    if skills.is_empty() {
                        println!("No trending skills found.");
                        return Ok(());
                    }

                    println!("╔══════════════════════════════════════════════════════════╗");
                    println!("║              Trending Skills                             ║");
                    println!("╚══════════════════════════════════════════════════════════╝");
                    println!();

                    for (i, skill) in skills.iter().enumerate() {
                        let rating = if skill.rating_count > 0 {
                            format!("★ {:.1}", skill.rating)
                        } else {
                            "★ -".to_string()
                        };

                        println!(
                            "{}. 🚀 {} \x1b[32mv{}\x1b[0m",
                            i + 1,
                            skill.name,
                            skill.version
                        );
                        println!("   {}", skill.description);
                        println!(
                            "   {} downloads | {} | {}",
                            skill.downloads, rating, skill.author
                        );
                        println!();
                    }

                    println!("Install a skill with: openrustclaw skills install <name>");
                }
                Err(e) => {
                    pb.finish_and_clear();
                    println!("⚠ Failed to fetch trending skills: {}", e);
                }
            }
        }
        Err(e) => {
            println!("⚠ Failed to connect to ClawHub: {}", e);
        }
    }

    Ok(())
}

/// Parsed skill metadata from SKILL.md format.
struct ParsedSkillMetadata {
    name: String,
    description: Option<String>,
    version: Option<String>,
    capabilities: Vec<String>,
    schema: Option<String>,
    signature: Option<String>,
}

/// Parse SKILL.md metadata.
fn parse_skill_metadata(content: &str) -> Result<ParsedSkillMetadata> {
    // Simple frontmatter parser
    let mut name = "unknown".to_string();
    let mut description = None;
    let mut version = None;
    let mut capabilities = Vec::new();
    let mut schema = None;
    let mut signature = None;

    // Extract name from title (# Title)
    for line in content.lines() {
        if let Some(title) = line.strip_prefix("# ") {
            name = title.trim().to_string();
            break;
        }
    }

    // Extract description from first paragraph after title
    let lines: Vec<_> = content.lines().collect();
    for (i, line_text) in lines.iter().enumerate() {
        if line_text.starts_with("# ") && i + 1 < lines.len() {
            // Find first non-empty line after title
            for other_line in &lines[(i + 1)..] {
                if !other_line.trim().is_empty() && !other_line.starts_with("##") {
                    description = Some(other_line.trim().to_string());
                    break;
                }
            }
            break;
        }
    }

    // Parse version if present in frontmatter or metadata section
    for line in content.lines() {
        if line.starts_with("version:") || line.starts_with("Version:") {
            version = line.split_once(':').map(|(_, s)| s.trim().to_string());
        }
        if line.starts_with("capabilities:") || line.starts_with("Capabilities:") {
            if let Some((_, raw)) = line.split_once(':') {
                capabilities = raw
                    .split(',')
                    .map(|value| value.trim().trim_matches('"'))
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string)
                    .collect();
            }
        }
        if line.starts_with("signature:") || line.starts_with("Signature:") {
            signature = line
                .split_once(':')
                .map(|(_, s)| s.trim().trim_matches('"').to_string())
                .filter(|value| !value.is_empty());
        }
    }

    // Extract JSON schema if present
    if let Some(start) = content.find("```json")
        && let Some(end) = content[start + 7..].find("```")
    {
        schema = Some(content[start + 7..start + 7 + end].trim().to_string());
    }

    Ok(ParsedSkillMetadata {
        name,
        description,
        version,
        capabilities,
        schema,
        signature,
    })
}

// Simple hex decode helper for when the hex crate isn't available
mod hex {
    pub fn decode(s: &str) -> anyhow::Result<Vec<u8>> {
        let mut result = Vec::with_capacity(s.len() / 2);
        let chars: Vec<_> = s.chars().collect();

        for chunk in chars.chunks(2) {
            if chunk.len() != 2 {
                anyhow::bail!("Invalid hex string length");
            }
            let high = chunk[0]
                .to_digit(16)
                .ok_or_else(|| anyhow::anyhow!("Invalid hex char"))?;
            let low = chunk[1]
                .to_digit(16)
                .ok_or_else(|| anyhow::anyhow!("Invalid hex char"))?;
            result.push((high << 4 | low) as u8);
        }

        Ok(result)
    }
}

use tracing::debug;

#[cfg(test)]
mod tests {
    use super::*;

    // --- hex module tests ---

    #[test]
    fn test_hex_decode_valid() {
        let result = hex::decode("48656c6c6f").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_hex_decode_empty() {
        let result = hex::decode("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_hex_decode_all_zeros() {
        let result = hex::decode("000000").unwrap();
        assert_eq!(result, vec![0, 0, 0]);
    }

    #[test]
    fn test_hex_decode_all_ff() {
        let result = hex::decode("ffffff").unwrap();
        assert_eq!(result, vec![0xff, 0xff, 0xff]);
    }

    #[test]
    fn test_hex_decode_invalid_char() {
        let result = hex::decode("zz");
        assert!(result.is_err());
    }

    #[test]
    fn test_hex_decode_odd_length() {
        let result = hex::decode("abc");
        // odd length: last chunk is 1 char, so it should fail
        assert!(result.is_err());
    }

    // --- parse_skill_metadata tests ---

    #[test]
    fn test_parse_skill_metadata_basic() {
        let content = "# My Skill\n\nThis is a great skill.\n";
        let metadata = parse_skill_metadata(content).unwrap();
        assert_eq!(metadata.name, "My Skill");
        assert_eq!(
            metadata.description.as_deref(),
            Some("This is a great skill.")
        );
    }

    #[test]
    fn test_parse_skill_metadata_with_version() {
        let content = "# Test Skill\n\nA test.\n\nversion: 1.2.3\n";
        let metadata = parse_skill_metadata(content).unwrap();
        assert_eq!(metadata.name, "Test Skill");
        assert_eq!(metadata.version.as_deref(), Some("1.2.3"));
    }

    #[test]
    fn test_parse_skill_metadata_with_capabilities() {
        let content = "# Net Skill\n\nA net skill.\n\ncapabilities: network_access, file_read\n";
        let metadata = parse_skill_metadata(content).unwrap();
        assert_eq!(metadata.capabilities, vec!["network_access", "file_read"]);
    }

    #[test]
    fn test_parse_skill_metadata_with_json_schema() {
        let content = "# Schema Skill\n\nHas schema.\n\n```json\n{\"type\": \"object\"}\n```\n";
        let metadata = parse_skill_metadata(content).unwrap();
        assert_eq!(metadata.schema.as_deref(), Some("{\"type\": \"object\"}"));
    }

    #[test]
    fn test_parse_skill_metadata_with_signature() {
        let content = "# Signed Skill\n\nSigned.\n\nsignature: deadbeef\n";
        let metadata = parse_skill_metadata(content).unwrap();
        assert_eq!(metadata.signature.as_deref(), Some("deadbeef"));
    }

    #[test]
    fn test_parse_skill_metadata_no_title() {
        let content = "No title here\n\nJust text.\n";
        let metadata = parse_skill_metadata(content).unwrap();
        assert_eq!(metadata.name, "unknown");
    }

    #[test]
    fn test_parse_skill_metadata_empty_content() {
        let content = "";
        let metadata = parse_skill_metadata(content).unwrap();
        assert_eq!(metadata.name, "unknown");
        assert!(metadata.description.is_none());
        assert!(metadata.version.is_none());
        assert!(metadata.capabilities.is_empty());
    }

    #[test]
    fn test_serialize_capabilities_normalizes_and_deduplicates() {
        let serialized = serialize_capabilities(&[
            "network".to_string(),
            "file-read".to_string(),
            "network_access".to_string(),
        ])
        .unwrap();
        assert_eq!(serialized, r#"["file_read","network_access"]"#);
    }

    #[test]
    fn test_serialize_capabilities_rejects_unknown_values() {
        let error = serialize_capabilities(&["launch_missiles".to_string()]).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("Failed to validate skill capabilities")
                || message.contains("Unknown skill capability")
        );
    }

    #[test]
    fn test_sensitive_capabilities_filters_safe_entries() {
        let sensitive = sensitive_capabilities(&[
            "network_access".to_string(),
            "file_read".to_string(),
            "shell_exec".to_string(),
        ])
        .unwrap();
        assert_eq!(
            sensitive,
            vec!["network_access".to_string(), "shell_exec".to_string()]
        );
    }

    #[test]
    fn test_external_skill_policy_rejects_unsigned_sensitive_skills() {
        let config = openrustclaw_core::config::AppConfig::default();
        let error = enforce_external_skill_policy(
            &config,
            &["shell_exec".to_string()],
            None,
            "dangerous-skill",
        )
        .unwrap_err();
        assert!(error.to_string().contains("must be signed"));
    }

    #[test]
    fn test_external_skill_policy_allows_signed_sensitive_skills() {
        let config = openrustclaw_core::config::AppConfig::default();
        enforce_external_skill_policy(
            &config,
            &["shell_exec".to_string()],
            Some("deadbeef"),
            "signed-skill",
        )
        .unwrap();
    }

    #[tokio::test]
    async fn test_persist_verified_state_updates_row() {
        let db_path = std::env::temp_dir().join(format!("skills-verify-{}.db", uuid::Uuid::new_v4()));
        let pool = openrustclaw_db::init_pool(&format!("sqlite://{}", db_path.display()), 1)
            .await
            .unwrap();
        openrustclaw_db::run_migrations(&pool).await.unwrap();

        sqlx::query(
            r#"
            INSERT INTO skills (
                id, name, description, source, version, signature,
                verified, enabled, capabilities, schema, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
            "#,
        )
        .bind("skill-1")
        .bind("test-skill")
        .bind("desc")
        .bind("managed")
        .bind("1.0.0")
        .bind(Option::<String>::None)
        .bind(1i64)
        .bind(1i64)
        .bind("[]")
        .bind(Option::<String>::None)
        .execute(&pool)
        .await
        .unwrap();

        persist_verified_state(&pool, "skill-1", false).await.unwrap();
        let verified: i64 = sqlx::query_scalar("SELECT verified FROM skills WHERE id = ?")
            .bind("skill-1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(verified, 0);

        let _ = std::fs::remove_file(db_path);
    }
}

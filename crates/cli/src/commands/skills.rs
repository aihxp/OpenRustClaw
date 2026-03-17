//! Skill management commands.

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use sqlx::Row;

use openrustclaw_security::SkillVerifier;
use openrustclaw_skills::{ClawHubRegistry, SearchFilters, SortBy};

/// List installed skills from database.
pub async fn list() -> Result<()> {
    // Load configuration to get DB path
    let config = openrustclaw_core::config::AppConfig::load()
        .unwrap_or_default();
    
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
        "#
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
    let config = openrustclaw_core::config::AppConfig::load()
        .unwrap_or_default();
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
                println!("   Author: {} | Downloads: {} | {}", 
                    skill.author, skill.downloads, rating);
                
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
    let config = openrustclaw_core::config::AppConfig::load()
        .unwrap_or_default();
    
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
        println!("Use `openrustclaw skills verify {}` to verify its signature.", name);
        return Ok(());
    }
    
    // For now, we support installing from the local skills directory
    // In a full implementation, this would download from a marketplace API
    let skills_dir = std::path::Path::new("skills");
    if !skills_dir.exists() {
        tokio::fs::create_dir_all(skills_dir).await?;
    }
    
    let skill_file = skills_dir.join(format!("{}.md", name));
    
    if skill_file.exists() {
        // Load and parse the skill
        let content = tokio::fs::read_to_string(&skill_file).await?;
        
        // Parse SKILL.md format
        let metadata = parse_skill_metadata(&content)?;
        
        // Insert into database
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO skills (
                id, name, description, source, version, 
                verified, enabled, capabilities, schema, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
            "#
        )
        .bind(&id)
        .bind(&metadata.name)
        .bind(&metadata.description)
        .bind("workspace")
        .bind(&metadata.version)
        .bind(0) // not verified
        .bind(1) // enabled
        .bind(&metadata.capabilities)
        .bind(&metadata.schema)
        .execute(&pool)
        .await
        .context("Failed to insert skill into database")?;
        
        println!("✓ Skill '{}' installed successfully", name);
        println!("  ID: {}", id);
        println!("  Source: workspace");
        
        if config.security.skill_signature_required {
            println!();
            println!("⚠ Warning: Signature verification is required but this skill is not signed.");
            println!("  Run `openrustclaw skills verify {}` to verify.", name);
        }
    } else {
        // Try to install from ClawHub registry
        println!("Skill not found locally. Checking ClawHub registry...");
        
        let endpoint = "https://clawhub.openrustclaw.dev".to_string();
        
        match ClawHubRegistry::new(&endpoint).await {
            Ok(registry) => {
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
                            openrustclaw_skills::InstallResult::Installed { name, version, path } => {
                                println!("✓ Skill '{}' v{} installed successfully", name, version);
                                println!("  Path: {}", path.display());
                            }
                        }
                    }
                    Err(e) => {
                        pb.finish_and_clear();
                        println!("✗ Failed to install from ClawHub: {}", e);
                        println!();
                        println!("To create a new skill locally, add a SKILL.md file to the ./skills directory.");
                        println!("Expected file: {}", skill_file.display());
                    }
                }
            }
            Err(e) => {
                println!("✗ Failed to connect to ClawHub: {}", e);
                println!();
                println!("To create a new skill locally, add a SKILL.md file to the ./skills directory.");
                println!("Expected file: {}", skill_file.display());
            }
        }
    }
    
    Ok(())
}

/// Update an installed skill.
pub async fn update(name: &str) -> Result<()> {
    println!("Updating skill: {}", name);
    
    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load()
        .unwrap_or_default();
    
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
    let endpoint = "https://clawhub.openrustclaw.dev".to_string();
    
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
                            
                            // Update database
                            sqlx::query(
                                "UPDATE skills SET version = ?, updated_at = datetime('now') WHERE name = ?"
                            )
                            .bind(to.to_string())
                            .bind(name)
                            .execute(&pool)
                            .await?;
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
    let config = openrustclaw_core::config::AppConfig::load()
        .unwrap_or_default();
    
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
    let endpoint = "https://clawhub.openrustclaw.dev".to_string();
    
    if let Ok(registry) = ClawHubRegistry::new(&endpoint).await {
        if let Err(e) = registry.uninstall(name).await {
            debug!("Failed to uninstall via registry: {}", e);
        }
    }
    
    // Remove from database
    sqlx::query("DELETE FROM skills WHERE name = ?")
        .bind(name)
        .execute(&pool)
        .await?;
    
    println!("✓ Skill '{}' uninstalled successfully.", name);
    
    Ok(())
}

/// Verify skill signatures.
pub async fn verify(name: &str) -> Result<()> {
    println!("Verifying skill: {}", name);
    
    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load()
        .unwrap_or_default();
    
    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;
    
    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;
    
    // Get skill from database
    let row = sqlx::query(
        "SELECT id, signature, source FROM skills WHERE name = ?"
    )
    .bind(name)
    .fetch_optional(&pool)
    .await?;
    
    let Some(row) = row else {
        anyhow::bail!("Skill '{}' not found", name);
    };
    
    let id: String = row.get("id");
    let signature: Option<String> = row.get("signature");
    let source: String = row.get("source");
    
    // Workspace and bundled skills are exempt from verification
    if source == "workspace" || source == "bundled" {
        println!("✓ Skill '{}' is from {} - verification exempt", name, source);
        
        // Update verified status
        sqlx::query("UPDATE skills SET verified = 1 WHERE id = ?")
            .bind(&id)
            .execute(&pool)
            .await?;
        
        return Ok(());
    }
    
    // Load skill content
    let skills_dir = std::path::Path::new("skills");
    let skill_file = skills_dir.join(format!("{}.md", name));
    
    if !skill_file.exists() {
        anyhow::bail!("Skill file not found: {}", skill_file.display());
    }
    
    let content = tokio::fs::read(&skill_file).await?;
    
    // Create verifier
    let verifier = SkillVerifier::disabled(); // In production, load from config
    
    // Check if skill has signature
    let Some(sig_hex) = signature else {
        println!("✗ Skill '{}' has no signature", name);
        return Ok(());
    };
    
    // Parse signature
    let signature_bytes = hex::decode(&sig_hex)
        .context("Invalid signature format in database")?;
    
    // Verify
    match verifier.verify(&content, &signature_bytes) {
        Ok(true) => {
            println!("✓ Skill '{}' signature verified successfully", name);
            
            // Update verified status
            sqlx::query("UPDATE skills SET verified = 1 WHERE id = ?")
                .bind(&id)
                .execute(&pool)
                .await?;
        }
        Ok(false) => {
            println!("✗ Skill '{}' signature verification failed", name);
        }
        Err(e) => {
            println!("✗ Error verifying skill '{}': {}", name, e);
        }
    }
    
    Ok(())
}

/// Show popular skills from the registry.
pub async fn popular(limit: usize) -> Result<()> {
    println!("🔥 Popular Skills");
    println!();
    
    let endpoint = "https://clawhub.openrustclaw.dev".to_string();
    
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
                        
                        println!("{}. 📦 {} \x1b[32mv{}\x1b[0m", i + 1, skill.name, skill.version);
                        println!("   {}", skill.description);
                        println!("   {} downloads | {} | {}", 
                            skill.downloads, rating, skill.author);
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
    
    let endpoint = "https://clawhub.openrustclaw.dev".to_string();
    
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
                        
                        println!("{}. 🚀 {} \x1b[32mv{}\x1b[0m", i + 1, skill.name, skill.version);
                        println!("   {}", skill.description);
                        println!("   {} downloads | {} | {}", 
                            skill.downloads, rating, skill.author);
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
    capabilities: Option<String>,
    schema: Option<String>,
}

/// Parse SKILL.md metadata.
fn parse_skill_metadata(content: &str) -> Result<ParsedSkillMetadata> {
    // Simple frontmatter parser
    let mut name = "unknown".to_string();
    let mut description = None;
    let mut version = None;
    let mut capabilities = None;
    let mut schema = None;
    
    // Extract name from title (# Title)
    for line in content.lines() {
        if let Some(title) = line.strip_prefix("# ") {
            name = title.trim().to_string();
            break;
        }
    }
    
    // Extract description from first paragraph after title
    let lines: Vec<_> = content.lines().collect();
    for i in 0..lines.len() {
        if lines[i].starts_with("# ") && i + 1 < lines.len() {
            // Find first non-empty line after title
            for j in (i + 1)..lines.len() {
                if !lines[j].trim().is_empty() && !lines[j].starts_with("##") {
                    description = Some(lines[j].trim().to_string());
                    break;
                }
            }
            break;
        }
    }
    
    // Parse version if present in frontmatter or metadata section
    for line in content.lines() {
        if line.starts_with("version:") || line.starts_with("Version:") {
            version = line.splitn(2, ':').nth(1).map(|s| s.trim().to_string());
        }
        if line.starts_with("capabilities:") || line.starts_with("Capabilities:") {
            capabilities = line.splitn(2, ':').nth(1).map(|s| s.trim().to_string());
        }
    }
    
    // Extract JSON schema if present
    if let Some(start) = content.find("```json") {
        if let Some(end) = content[start + 7..].find("```") {
            schema = Some(content[start + 7..start + 7 + end].trim().to_string());
        }
    }
    
    Ok(ParsedSkillMetadata {
        name,
        description,
        version,
        capabilities,
        schema,
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
            let high = chunk[0].to_digit(16).ok_or_else(|| anyhow::anyhow!("Invalid hex char"))?;
            let low = chunk[1].to_digit(16).ok_or_else(|| anyhow::anyhow!("Invalid hex char"))?;
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
        assert_eq!(metadata.description.as_deref(), Some("This is a great skill."));
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
        assert_eq!(metadata.capabilities.as_deref(), Some("network_access, file_read"));
    }

    #[test]
    fn test_parse_skill_metadata_with_json_schema() {
        let content = "# Schema Skill\n\nHas schema.\n\n```json\n{\"type\": \"object\"}\n```\n";
        let metadata = parse_skill_metadata(content).unwrap();
        assert_eq!(metadata.schema.as_deref(), Some("{\"type\": \"object\"}"));
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
    }
}

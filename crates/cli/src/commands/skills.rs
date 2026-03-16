//! Skill management commands.

use anyhow::{Context, Result};
use sqlx::Row;

use openrustclaw_security::SkillVerifier;

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
        // Would download from marketplace in full implementation
        println!("Skill '{}' not found in local skills directory.", name);
        println!();
        println!("To create a new skill, add a SKILL.md file to the ./skills directory.");
        println!("Expected file: {}", skill_file.display());
    }
    
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

/// Parsed skill metadata from SKILL.md format.
struct SkillMetadata {
    name: String,
    description: Option<String>,
    version: Option<String>,
    capabilities: Option<String>,
    schema: Option<String>,
}

/// Parse SKILL.md metadata.
fn parse_skill_metadata(content: &str) -> Result<SkillMetadata> {
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
    
    Ok(SkillMetadata {
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

//! Memory management commands.

use anyhow::{Context, Result};
use sqlx::Row;

/// Export memories to markdown.
pub async fn export(output: &str, user_id: Option<&str>) -> Result<()> {
    println!("Exporting memories to: {}", output);

    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Build query
    let mut query = String::from(
        r#"
        SELECT 
            id,
            memory_type,
            content,
            source,
            source_type,
            user_id,
            namespace,
            importance,
            confidence,
            access_count,
            created_at
        FROM memory_entries 
        WHERE 1=1
        "#,
    );

    if user_id.is_some() {
        query.push_str(" AND (user_id = ? OR user_id IS NULL)");
    }

    query.push_str(" ORDER BY created_at DESC");

    // Execute query
    let rows = if let Some(uid) = user_id {
        sqlx::query(&query).bind(uid).fetch_all(&pool).await?
    } else {
        sqlx::query(&query).fetch_all(&pool).await?
    };

    if rows.is_empty() {
        println!("No memories found to export.");
        return Ok(());
    }

    // Build markdown output
    let mut markdown = String::new();

    // Header
    markdown.push_str("# OpenRustClaw Memory Export\n\n");
    markdown.push_str(&format!(
        "Generated: {}\n\n",
        chrono::Utc::now().to_rfc3339()
    ));

    if let Some(uid) = user_id {
        markdown.push_str(&format!("User: {}\n\n", uid));
    }

    // Statistics
    markdown.push_str("## Statistics\n\n");
    markdown.push_str(&format!("Total memories: {}\n\n", rows.len()));

    // Group by type
    let mut by_type: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for row in &rows {
        let memory_type: String = row.get("memory_type");
        *by_type.entry(memory_type).or_insert(0) += 1;
    }

    markdown.push_str("| Type | Count |\n");
    markdown.push_str("|------|-------|\n");
    for (typ, count) in by_type {
        markdown.push_str(&format!("| {} | {} |\n", typ, count));
    }
    markdown.push('\n');

    // Memories
    markdown.push_str("## Memories\n\n");

    let row_count = rows.len();
    for row in &rows {
        let id: String = row.get("id");
        let memory_type: String = row.get("memory_type");
        let content: String = row.get("content");
        let source: Option<String> = row.get("source");
        let source_type: Option<String> = row.get("source_type");
        let user_id: Option<String> = row.get("user_id");
        let namespace: Option<String> = row.get("namespace");
        let importance: f64 = row.get("importance");
        let confidence: f64 = row.get("confidence");
        let access_count: i64 = row.get("access_count");
        let created_at: String = row.get("created_at");

        markdown.push_str(&format!("### {} ({}...)\n\n", memory_type, &id[..8]));
        markdown.push_str(&format!("**Content:**\n{}\n\n", content));

        if let Some(src) = source {
            markdown.push_str(&format!("- **Source:** {}\n", src));
        }
        if let Some(st) = source_type {
            markdown.push_str(&format!("- **Source Type:** {}\n", st));
        }
        if let Some(uid) = user_id {
            markdown.push_str(&format!("- **User:** {}\n", uid));
        }
        if let Some(ns) = namespace {
            markdown.push_str(&format!("- **Namespace:** {}\n", ns));
        }

        markdown.push_str(&format!("- **Importance:** {:.2}\n", importance));
        markdown.push_str(&format!("- **Confidence:** {:.2}\n", confidence));
        markdown.push_str(&format!("- **Access Count:** {}\n", access_count));
        markdown.push_str(&format!("- **Created:** {}\n", created_at));
        markdown.push('\n');
        markdown.push_str("---\n\n");
    }

    // Write to file
    tokio::fs::write(output, markdown)
        .await
        .with_context(|| format!("Failed to write to {}", output))?;

    println!("✓ Exported {} memories to {}", row_count, output);

    Ok(())
}

/// Import memories from OpenClaw MEMORY.md format.
pub async fn import(file: &str, user_id: &str) -> Result<()> {
    println!("Importing memories from: {}", file);
    println!("Target user: {}", user_id);

    // Read file
    let content = tokio::fs::read_to_string(file)
        .await
        .with_context(|| format!("Failed to read file: {}", file))?;

    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Parse MEMORY.md format
    // Expected format:
    // # MEMORY.md
    // ## <Type> - <Key>
    // Content here...

    let mut imported = 0;
    let lines: Vec<_> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        // Look for section headers (## Type - Key)
        if lines[i].starts_with("## ") {
            let header = &lines[i][3..];
            let parts: Vec<_> = header.splitn(2, " - ").collect();

            if parts.len() == 2 {
                let memory_type = parts[0].trim().to_lowercase();
                let _key = parts[1].trim();

                // Collect content until next section
                let mut memory_content = String::new();
                i += 1;

                while i < lines.len() && !lines[i].starts_with("## ") && !lines[i].starts_with("# ")
                {
                    if !lines[i].trim().is_empty() {
                        memory_content.push_str(lines[i]);
                        memory_content.push('\n');
                    }
                    i += 1;
                }

                let memory_content = memory_content.trim();

                if !memory_content.is_empty() {
                    // Calculate content hash
                    let content_hash = sha256_hex(memory_content);

                    // Map memory type
                    let db_type = match memory_type.as_str() {
                        "episodic" | "event" => "episodic",
                        "semantic" | "fact" => "semantic",
                        "procedural" | "howto" | "workflow" => "procedural",
                        _ => "semantic",
                    };

                    // Insert into database
                    let id = uuid::Uuid::new_v4().to_string();
                    let result = sqlx::query(
                        r#"
                        INSERT INTO memory_entries (
                            id, memory_type, content, content_hash, 
                            user_id, namespace, importance, confidence, created_at
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
                        ON CONFLICT(content_hash) DO NOTHING
                        "#,
                    )
                    .bind(&id)
                    .bind(db_type)
                    .bind(memory_content)
                    .bind(&content_hash)
                    .bind(user_id)
                    .bind("imported")
                    .bind(0.8f64)
                    .bind(1.0f64)
                    .execute(&pool)
                    .await?;

                    if result.rows_affected() > 0 {
                        imported += 1;
                    }

                    continue;
                }
            }
        }

        i += 1;
    }

    println!("✓ Imported {} memories", imported);

    Ok(())
}

/// Show memory statistics.
pub async fn stats() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║               Memory Statistics                          ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Total count
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memory_entries")
        .fetch_one(&pool)
        .await?;

    println!("Total Memories: {}", total);
    println!();

    if total == 0 {
        println!("No memories stored yet.");
        return Ok(());
    }

    // By type
    println!("By Type:");
    let type_rows = sqlx::query(
        "SELECT memory_type, COUNT(*) as count FROM memory_entries GROUP BY memory_type",
    )
    .fetch_all(&pool)
    .await?;

    for row in type_rows {
        let memory_type: String = row.get("memory_type");
        let count: i64 = row.get("count");
        let percentage = (count as f64 / total as f64) * 100.0;
        println!("  {}: {} ({:.1}%)", memory_type, count, percentage);
    }
    println!();

    // By namespace
    println!("By Namespace:");
    let ns_rows = sqlx::query(
        "SELECT namespace, COUNT(*) as count FROM memory_entries GROUP BY namespace ORDER BY count DESC LIMIT 5"
    )
    .fetch_all(&pool)
    .await?;

    for row in ns_rows {
        let namespace: String = row.get("namespace");
        let count: i64 = row.get("count");
        println!("  {}: {}", namespace, count);
    }
    println!();

    // Access statistics
    let access_stats: (f64, i64, i64) = sqlx::query_as(
        "SELECT AVG(access_count), MAX(access_count), SUM(access_count) FROM memory_entries",
    )
    .fetch_one(&pool)
    .await?;

    println!("Access Statistics:");
    println!("  Average accesses: {:.2}", access_stats.0);
    println!("  Max accesses: {}", access_stats.1);
    println!("  Total accesses: {}", access_stats.2);
    println!();

    // Oldest and newest
    let oldest: Option<String> =
        sqlx::query_scalar("SELECT created_at FROM memory_entries ORDER BY created_at ASC LIMIT 1")
            .fetch_optional(&pool)
            .await?;

    let newest: Option<String> = sqlx::query_scalar(
        "SELECT created_at FROM memory_entries ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(&pool)
    .await?;

    if let (Some(old), Some(new)) = (oldest, newest) {
        println!("Date Range:");
        println!("  Oldest: {}", old);
        println!("  Newest: {}", new);
    }

    // Core memory
    let core_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM core_memory")
        .fetch_one(&pool)
        .await?;

    println!();
    println!("Core Memory Entries: {}", core_count);

    Ok(())
}

/// Simple SHA-256 hash
fn sha256_hex(input: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Note: In production, use a proper SHA-256 implementation
    // This is a simplified version for the CLI
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_hex_nonempty() {
        let hash = sha256_hex("hello world");
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_sha256_hex_deterministic() {
        let hash1 = sha256_hex("test input");
        let hash2 = sha256_hex("test input");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_sha256_hex_different_inputs_differ() {
        let hash1 = sha256_hex("input one");
        let hash2 = sha256_hex("input two");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_sha256_hex_is_hex_string() {
        let hash = sha256_hex("some data");
        assert_eq!(hash.len(), 16); // 16 hex chars = 8 bytes
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_sha256_hex_empty_input() {
        let hash = sha256_hex("");
        assert!(!hash.is_empty());
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
}

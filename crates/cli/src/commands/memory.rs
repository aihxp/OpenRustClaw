//! Memory management commands.

use anyhow::{Context, Result};
use chrono::Utc;
use openrustclaw_app::memory_views as app_memory_views;
use openrustclaw_core::traits::{CoreMemoryStore, MemoryStore};
use openrustclaw_core::types::{CoreEntry, MemoryEntry, MemoryQuery, MemoryType, SourceType};
use openrustclaw_db::{SqliteCoreMemoryStore, SqliteMemoryStore};
use openrustclaw_memory::WorkspaceArtifactRegistry;
use sqlx::Row;
use std::path::{Path, PathBuf};

fn app_core_item(entry: &CoreEntry) -> app_memory_views::CoreMemoryItem {
    app_memory_views::CoreMemoryItem {
        key: entry.key.clone(),
        value: entry.value.clone(),
    }
}

fn app_recall_item(entry: &MemoryEntry) -> app_memory_views::RecallMemoryItem {
    app_memory_views::RecallMemoryItem {
        id: entry.id.to_string(),
        memory_type: memory_type_label(entry.memory_type).to_string(),
        namespace: entry.namespace.clone(),
        content: entry.content.clone(),
    }
}

fn app_archive_item(
    entry: &openrustclaw_db::models::MemoryArchiveRow,
) -> app_memory_views::ArchiveMemoryItem {
    app_memory_views::ArchiveMemoryItem {
        id: entry.id.clone(),
        summary: entry.summary.clone(),
    }
}

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

/// Import memories from a legacy MEMORY.md format.
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

/// Get one memory entry by id.
pub async fn get(id: &str) -> Result<()> {
    let (store, _core_store, _pool) = open_stores().await?;
    match store.get(id).await? {
        Some(entry) => {
            println!("{}", serde_json::to_string_pretty(&entry)?);
        }
        None => {
            println!("Memory entry not found: {}", id);
        }
    }
    Ok(())
}

/// Show recent memory timeline.
pub async fn timeline(namespace: Option<&str>, limit: usize) -> Result<()> {
    let (store, _core_store, _pool) = open_stores().await?;
    for entry in store.list_recent(namespace, limit).await? {
        let policy = assistant_write_policy_summary(&entry)
            .map(|summary| format!("  [{}]", summary))
            .unwrap_or_default();
        println!(
            "{}  {}  {}  {}{}",
            entry.created_at.to_rfc3339(),
            memory_type_label(entry.memory_type),
            entry.id,
            entry.content.replace('\n', " "),
            policy
        );
    }
    Ok(())
}

/// Search recall memory entries.
pub async fn search(
    query: &str,
    namespace: Option<&str>,
    limit: usize,
    min_confidence: Option<f32>,
    memory_type: Option<&str>,
    source_type: Option<&str>,
) -> Result<()> {
    let (store, _core_store, _pool) = open_stores().await?;
    let memory_types = memory_type
        .map(parse_memory_type)
        .transpose()?
        .into_iter()
        .collect();
    let source_types = source_type
        .map(parse_source_type)
        .transpose()?
        .into_iter()
        .collect();
    let query = MemoryQuery {
        text: query.to_string(),
        memory_types,
        source_types,
        namespace: namespace.map(|value| value.to_string()),
        limit,
        min_confidence: min_confidence.unwrap_or(0.0),
        recency_weight: 0.2,
    };

    for result in store.search(&query).await? {
        let policy = assistant_write_policy_summary(&result.entry)
            .map(|summary| format!("  [{}]", summary))
            .unwrap_or_default();
        println!(
            "{:.3}  {}  {}  {}{}",
            result.score,
            memory_type_label(result.entry.memory_type),
            result.entry.id,
            result.entry.content.replace('\n', " "),
            policy
        );
    }
    Ok(())
}

/// List known memory namespaces.
pub async fn namespaces() -> Result<()> {
    let (store, _core_store, _pool) = open_stores().await?;
    for namespace in store.list_namespaces().await? {
        println!("{}", namespace);
    }
    Ok(())
}

/// Inspect archive summaries.
pub async fn archive(namespace: Option<&str>, limit: usize) -> Result<()> {
    let (store, _core_store, _pool) = open_stores().await?;
    for entry in store.list_archive_entries(namespace, limit).await? {
        println!(
            "{}  {}  {}",
            entry.created_at,
            entry.id,
            entry.summary.replace('\n', " ")
        );
    }
    Ok(())
}

/// Export file-backed memory and persona views.
pub async fn views_export(root: &str, user_id: Option<&str>) -> Result<()> {
    let (store, core_store, _pool) = open_stores().await?;
    let root = PathBuf::from(root);
    let user_id = user_id.unwrap_or("default");
    let views_root = root.join(".claw/memory/views");
    tokio::fs::create_dir_all(views_root.join("core")).await?;
    tokio::fs::create_dir_all(views_root.join("recall")).await?;
    tokio::fs::create_dir_all(views_root.join("archive")).await?;
    tokio::fs::create_dir_all(root.join(".claw/persona")).await?;
    tokio::fs::create_dir_all(root.join(".claw/memory/ledgers")).await?;

    let core_entries = core_store.get_all(user_id).await?;
    tokio::fs::write(
        views_root.join("core").join(format!("{}.md", user_id)),
        render_core_view(user_id, &core_entries),
    )
    .await?;

    let recall_entries = store.list_recent(Some(user_id), 200).await?;
    tokio::fs::write(
        views_root.join("recall").join(format!("{}.md", user_id)),
        render_recall_view(user_id, &recall_entries),
    )
    .await?;

    let archive_entries = store.list_archive_entries(Some(user_id), 200).await?;
    tokio::fs::write(
        views_root.join("archive").join(format!("{}.md", user_id)),
        render_archive_view(user_id, &archive_entries),
    )
    .await?;

    let persona_path = root.join(".claw/persona/profile.md");
    if !persona_path.exists() {
        tokio::fs::write(
            &persona_path,
            format!(
                "# Persona\n\n## User\n{}\n\n## Values\n- helpful\n- precise\n",
                user_id
            ),
        )
        .await?;
    }

    let pool = open_pool().await?;
    let rows = sqlx::query(
        r#"
        SELECT event_name, payload, created_at
        FROM runtime_events
        WHERE event_name IN ('message.received', 'message.sent', 'session.pre_compaction')
        ORDER BY created_at DESC
        LIMIT 50
        "#,
    )
    .fetch_all(&pool)
    .await?;
    let mut ledger = String::from("# Runtime Learnings Ledger\n\n");
    for row in rows {
        let event_name: String = row.get("event_name");
        let payload: String = row.get("payload");
        let created_at: String = row.get("created_at");
        ledger.push_str(&format!(
            "## {} [{}]\n{}\n\n",
            event_name, created_at, payload
        ));
    }
    tokio::fs::write(root.join(".claw/memory/ledgers/runtime.md"), ledger).await?;

    println!(
        "✓ Exported file-backed memory views under {}",
        views_root.display()
    );
    Ok(())
}

/// Import edited file-backed memory and persona views.
pub async fn views_import(root: &str, user_id: &str) -> Result<()> {
    let (store, core_store, _pool) = open_stores().await?;
    let root = PathBuf::from(root);
    let core_path = root
        .join(".claw/memory/views/core")
        .join(format!("{}.md", user_id));
    if core_path.is_file() {
        let content = tokio::fs::read_to_string(&core_path).await?;
        for entry in parse_core_view(&content) {
            core_store.set(user_id, entry).await?;
        }
    }

    let recall_path = root
        .join(".claw/memory/views/recall")
        .join(format!("{}.md", user_id));
    if recall_path.is_file() {
        let content = tokio::fs::read_to_string(&recall_path).await?;
        for entry in parse_recall_view(&content, user_id) {
            store.store(entry).await?;
        }
    }

    println!("✓ Imported memory views for {}", user_id);
    Ok(())
}

/// Scan workspace artifacts.
pub async fn artifacts_scan(root: &str) -> Result<()> {
    let artifacts = WorkspaceArtifactRegistry::scan(Path::new(root))?;
    for artifact in artifacts {
        println!(
            "{}  {:?}  {:?}",
            artifact.path.display(),
            artifact.class,
            artifact.visibility
        );
    }
    Ok(())
}

/// Render merged artifact bundle for a model.
pub async fn artifacts_render(root: &str, model: &str) -> Result<()> {
    let bundle = WorkspaceArtifactRegistry::resolve(Path::new(root), model)?;
    println!("{}", bundle.merged_instructions);
    Ok(())
}

/// Sync preferred artifact files for a model family.
pub async fn artifacts_sync(root: &str, model: &str) -> Result<()> {
    let written = WorkspaceArtifactRegistry::sync_preferred(Path::new(root), model)?;
    for path in written {
        println!("{}", path.display());
    }
    Ok(())
}

async fn open_stores() -> Result<(SqliteMemoryStore, SqliteCoreMemoryStore, sqlx::SqlitePool)> {
    let pool = open_pool().await?;
    Ok((
        SqliteMemoryStore::new(pool.clone()),
        SqliteCoreMemoryStore::new(pool.clone()),
        pool,
    ))
}

async fn open_pool() -> Result<sqlx::SqlitePool> {
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;
    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;
    Ok(pool)
}

fn render_core_view(user_id: &str, entries: &[CoreEntry]) -> String {
    app_memory_views::MemoryViewsService::new().render_core_view(
        user_id,
        &entries.iter().map(app_core_item).collect::<Vec<_>>(),
    )
}

fn render_recall_view(user_id: &str, entries: &[MemoryEntry]) -> String {
    app_memory_views::MemoryViewsService::new().render_recall_view(
        user_id,
        &entries.iter().map(app_recall_item).collect::<Vec<_>>(),
    )
}

fn render_archive_view(
    namespace: &str,
    entries: &[openrustclaw_db::models::MemoryArchiveRow],
) -> String {
    app_memory_views::MemoryViewsService::new().render_archive_view(
        namespace,
        &entries.iter().map(app_archive_item).collect::<Vec<_>>(),
    )
}

fn parse_core_view(content: &str) -> Vec<CoreEntry> {
    app_memory_views::MemoryViewsService::new()
        .parse_core_view(content)
        .into_iter()
        .map(|entry| CoreEntry {
            key: entry.key,
            value: entry.value.clone(),
            importance: 0.8,
            token_count: entry.value.len() / 4,
            updated_at: Utc::now(),
        })
        .collect()
}

fn parse_recall_view(content: &str, user_id: &str) -> Vec<MemoryEntry> {
    app_memory_views::MemoryViewsService::new()
        .parse_recall_view(content)
        .into_iter()
        .map(|entry| {
            build_memory_entry(
                &entry.id,
                &entry.memory_type,
                &entry.namespace,
                user_id,
                &entry.content,
            )
        })
        .collect()
}

fn build_memory_entry(
    id: &str,
    typ: &str,
    namespace: &str,
    user_id: &str,
    body: &str,
) -> MemoryEntry {
    MemoryEntry {
        id: uuid::Uuid::parse_str(id).unwrap_or_else(|_| uuid::Uuid::new_v4()),
        memory_type: match typ {
            "episodic" => MemoryType::Episodic,
            "procedural" => MemoryType::Procedural,
            _ => MemoryType::Semantic,
        },
        content: body.trim().to_string(),
        content_hash: openrustclaw_memory::MemoryPolicies::content_hash(body.trim()),
        source: Some("memory_views_import".to_string()),
        source_type: Some(SourceType::Conversation),
        session_id: None,
        user_id: Some(user_id.to_string()),
        namespace: namespace.to_string(),
        importance: 0.7,
        confidence: 1.0,
        access_count: 0,
        last_accessed: None,
        created_at: Utc::now(),
        expires_at: None,
        metadata: serde_json::json!({
            "source": "memory_views_import",
        }),
    }
}

fn memory_type_label(value: MemoryType) -> &'static str {
    match value {
        MemoryType::Episodic => "episodic",
        MemoryType::Semantic => "semantic",
        MemoryType::Procedural => "procedural",
    }
}

fn assistant_write_policy_summary(entry: &MemoryEntry) -> Option<String> {
    app_memory_views::MemoryViewsService::new().assistant_write_policy_summary(&entry.metadata)
}

fn parse_memory_type(value: &str) -> Result<MemoryType> {
    match app_memory_views::MemoryViewsService::new().parse_memory_type(value) {
        Some("episodic") => Ok(MemoryType::Episodic),
        Some("semantic") => Ok(MemoryType::Semantic),
        Some("procedural") => Ok(MemoryType::Procedural),
        _ => anyhow::bail!(
            "Unknown memory type '{}'. Available: episodic, semantic, procedural",
            value
        ),
    }
}

fn parse_source_type(value: &str) -> Result<SourceType> {
    match app_memory_views::MemoryViewsService::new().parse_source_type(value) {
        Some("document") => Ok(SourceType::Document),
        Some("code") => Ok(SourceType::Code),
        Some("config") => Ok(SourceType::Config),
        Some("conversation") => Ok(SourceType::Conversation),
        Some("runbook") => Ok(SourceType::Runbook),
        Some("tool_schema") => Ok(SourceType::ToolSchema),
        _ => anyhow::bail!(
            "Unknown source type '{}'. Available: document, code, config, conversation, runbook, tool_schema",
            value
        ),
    }
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

    #[test]
    fn test_parse_memory_type_accepts_known_values() {
        assert_eq!(parse_memory_type("episodic").unwrap(), MemoryType::Episodic);
        assert_eq!(parse_memory_type("semantic").unwrap(), MemoryType::Semantic);
        assert_eq!(
            parse_memory_type("procedural").unwrap(),
            MemoryType::Procedural
        );
    }

    #[test]
    fn test_parse_source_type_accepts_known_values() {
        assert_eq!(parse_source_type("code").unwrap(), SourceType::Code);
        assert_eq!(
            parse_source_type("conversation").unwrap(),
            SourceType::Conversation
        );
        assert_eq!(
            parse_source_type("tool-schema").unwrap(),
            SourceType::ToolSchema
        );
    }

    #[test]
    fn test_assistant_write_policy_summary_formats_basis_and_reason() {
        let entry = MemoryEntry {
            id: uuid::Uuid::new_v4(),
            memory_type: MemoryType::Semantic,
            content: "User prefers Rust".to_string(),
            content_hash: "hash".to_string(),
            source: Some("agent_tool".to_string()),
            source_type: Some(SourceType::Conversation),
            session_id: None,
            user_id: Some("user_123".to_string()),
            namespace: "user_123".to_string(),
            importance: 1.0,
            confidence: 1.0,
            access_count: 0,
            last_accessed: None,
            created_at: Utc::now(),
            expires_at: None,
            metadata: serde_json::json!({
                "assistant_write_policy": {
                    "basis": "explicit_user_request",
                    "declared_reason": "The user asked me to remember it."
                }
            }),
        };

        assert_eq!(
            assistant_write_policy_summary(&entry).as_deref(),
            Some("basis=explicit_user_request; reason=The user asked me to remember it.")
        );
    }
}

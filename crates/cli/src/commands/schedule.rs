//! Job scheduling and file-backed task manifest commands.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use openrustclaw_app::schedule_planning as app_schedule_planning;
use openrustclaw_scheduler::tasks::{TaskTrigger, default_export_path, disabled_until_utc};
use openrustclaw_scheduler::{
    DEFAULT_TASKS_DIR, DurableEventBus, LoadedTaskManifest, TaskManifest, TaskSpec,
    load_task_manifest, manifest_job_id, render_task_manifest, tasks_dir_for_root,
};
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;
use walkdir::WalkDir;

async fn open_schedule_pool() -> Result<sqlx::SqlitePool> {
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;
    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;
    Ok(pool)
}

async fn publish_control_event(
    pool: &sqlx::SqlitePool,
    event_name: &str,
    payload: serde_json::Value,
) {
    let event_bus = DurableEventBus::new(pool.clone(), 64);
    if let Err(error) = event_bus
        .publish_named(event_name, "control_runtime_event", None, &payload, None)
        .await
    {
        tracing::warn!(error = %error, event_name, "Failed to publish control runtime event");
    }
}

fn current_workspace_root() -> Result<PathBuf> {
    std::env::current_dir().context("Failed to resolve current workspace root")
}

fn resolve_tasks_root(path: Option<&str>) -> Result<PathBuf> {
    let root = match path {
        Some(path) => PathBuf::from(path),
        None => tasks_dir_for_root(current_workspace_root()?),
    };

    if root.is_absolute() {
        Ok(root)
    } else {
        Ok(current_workspace_root()?.join(root))
    }
}

fn notes_path_for_manifest_path(path: &Path) -> PathBuf {
    let stem = path
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "task".to_string());
    path.parent()
        .unwrap_or_else(|| Path::new("."))
        .join("_artifacts")
        .join(stem)
        .join("notes.md")
}

fn detect_manifest_paths(root: &Path) -> Vec<PathBuf> {
    let mut manifests = Vec::new();
    if !root.exists() {
        return manifests;
    }

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        if matches!(ext, "yaml" | "yml") {
            manifests.push(path);
        }
    }

    manifests.sort();
    manifests
}

fn app_task_trigger(trigger: &TaskTrigger) -> app_schedule_planning::TaskTriggerSpec {
    match trigger {
        TaskTrigger::Interval { every_seconds } => {
            app_schedule_planning::TaskTriggerSpec::Interval {
                every_seconds: *every_seconds,
            }
        }
        TaskTrigger::Absolute { at } => {
            app_schedule_planning::TaskTriggerSpec::Absolute { at: at.clone() }
        }
        TaskTrigger::Event { event_name } => app_schedule_planning::TaskTriggerSpec::Event {
            event_name: event_name.clone(),
        },
        TaskTrigger::Dependency { depends_on } => {
            app_schedule_planning::TaskTriggerSpec::Dependency {
                depends_on: depends_on.clone(),
            }
        }
    }
}

fn task_trigger_from_app(trigger: app_schedule_planning::TaskTriggerSpec) -> TaskTrigger {
    match trigger {
        app_schedule_planning::TaskTriggerSpec::Interval { every_seconds } => {
            TaskTrigger::Interval { every_seconds }
        }
        app_schedule_planning::TaskTriggerSpec::Absolute { at } => TaskTrigger::Absolute { at },
        app_schedule_planning::TaskTriggerSpec::Event { event_name } => {
            TaskTrigger::Event { event_name }
        }
        app_schedule_planning::TaskTriggerSpec::Dependency { depends_on } => {
            TaskTrigger::Dependency { depends_on }
        }
    }
}

fn trigger_from_inline(every_seconds: Option<u64>, at: Option<&str>) -> Result<TaskTrigger> {
    Ok(task_trigger_from_app(
        app_schedule_planning::SchedulePlanningService::new()
            .trigger_from_inline(every_seconds, at)?,
    ))
}

fn trigger_config_and_next_run(
    trigger: &TaskTrigger,
) -> Result<(String, Value, Option<DateTime<Utc>>)> {
    Ok(app_schedule_planning::SchedulePlanningService::new()
        .trigger_config_and_next_run(&app_task_trigger(trigger), Utc::now())?)
}

fn build_metadata(
    payload: Value,
    priority: i64,
    owner: Option<&str>,
    tags: &[String],
    manifest_path: Option<&Path>,
    source_kind: &str,
    metadata: Value,
    delivery_policy: Option<&Value>,
    hook_policy: Option<&Value>,
    routing: Option<&Value>,
) -> Value {
    app_schedule_planning::SchedulePlanningService::new().build_metadata(
        payload,
        priority,
        owner,
        tags,
        manifest_path,
        source_kind,
        metadata,
        delivery_policy,
        hook_policy,
        routing,
    )
}

struct AppliedTaskResult {
    job_id: String,
    action: &'static str,
}

async fn upsert_task_manifest(
    pool: &sqlx::SqlitePool,
    loaded: &LoadedTaskManifest,
    dry_run: bool,
) -> Result<AppliedTaskResult> {
    let manifest_path = loaded
        .path
        .canonicalize()
        .unwrap_or_else(|_| loaded.path.clone());
    let job_id = manifest_job_id(&manifest_path, &loaded.manifest);
    let task = &loaded.manifest.task;
    let existing_hash: Option<String> = sqlx::query_scalar(
        "SELECT manifest_hash FROM task_manifests WHERE job_id = ? OR manifest_path = ?",
    )
    .bind(&job_id)
    .bind(manifest_path.display().to_string())
    .fetch_optional(pool)
    .await?;

    if existing_hash.as_deref() == Some(loaded.hash.as_str()) {
        return Ok(AppliedTaskResult {
            job_id,
            action: "unchanged",
        });
    }

    let action = if existing_hash.is_some() {
        "updated"
    } else {
        "created"
    };
    if dry_run {
        return Ok(AppliedTaskResult { job_id, action });
    }

    let (trigger_type, trigger_config, next_run_at) = trigger_config_and_next_run(&task.trigger)?;
    let notes_path = notes_path_for_manifest_path(&manifest_path);
    if let Some(parent) = notes_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create task artifact directory '{}'",
                parent.display()
            )
        })?;
    }

    let metadata = build_metadata(
        task.payload.clone(),
        task.priority,
        task.owner.as_deref(),
        &task.tags,
        Some(&manifest_path),
        "filesystem",
        task.metadata.clone(),
        task.delivery_policy.as_ref(),
        task.hook_policy.as_ref(),
        task.routing.as_ref(),
    );
    let state = if task.enabled { "active" } else { "paused" };
    let disabled_until = task.disabled_until.clone();
    let idempotency_key = format!("{}:{}", &job_id, Uuid::new_v4());

    sqlx::query(
        r#"
        INSERT INTO scheduled_jobs (
            id, name, description, workflow_id, trigger_type, trigger_config,
            idempotency_key, state, timezone, max_retries, priority, source_kind, owner,
            tags, disabled_until, manifest_path, task_notes_path, next_run_at, run_count,
            consecutive_failures, metadata, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0, ?, datetime('now'))
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            description = excluded.description,
            workflow_id = excluded.workflow_id,
            trigger_type = excluded.trigger_type,
            trigger_config = excluded.trigger_config,
            state = excluded.state,
            timezone = excluded.timezone,
            max_retries = excluded.max_retries,
            priority = excluded.priority,
            source_kind = excluded.source_kind,
            owner = excluded.owner,
            tags = excluded.tags,
            disabled_until = excluded.disabled_until,
            manifest_path = excluded.manifest_path,
            task_notes_path = excluded.task_notes_path,
            next_run_at = excluded.next_run_at,
            metadata = excluded.metadata
        "#,
    )
    .bind(&job_id)
    .bind(&task.name)
    .bind(
        task.description
            .clone()
            .unwrap_or_else(|| format!("Task manifest for workflow {}", task.workflow)),
    )
    .bind(&task.workflow)
    .bind(&trigger_type)
    .bind(trigger_config.to_string())
    .bind(idempotency_key)
    .bind(state)
    .bind(&task.timezone)
    .bind(task.max_retries.unwrap_or(3) as i64)
    .bind(task.priority)
    .bind("filesystem")
    .bind(task.owner.clone())
    .bind(serde_json::to_string(&task.tags)?)
    .bind(disabled_until)
    .bind(manifest_path.display().to_string())
    .bind(notes_path.display().to_string())
    .bind(next_run_at.map(|value| value.to_rfc3339()))
    .bind(metadata.to_string())
    .execute(pool)
    .await
    .context("Failed to upsert scheduled task manifest job")?;

    sqlx::query(
        r#"
        INSERT INTO task_manifests (job_id, manifest_path, manifest_hash, version, origin, imported_at, updated_at)
        VALUES (?, ?, ?, ?, 'filesystem', datetime('now'), datetime('now'))
        ON CONFLICT(job_id) DO UPDATE SET
            manifest_path = excluded.manifest_path,
            manifest_hash = excluded.manifest_hash,
            version = excluded.version,
            origin = excluded.origin,
            updated_at = datetime('now')
        "#,
    )
    .bind(&job_id)
    .bind(manifest_path.display().to_string())
    .bind(&loaded.hash)
    .bind(loaded.manifest.version as i64)
    .execute(pool)
    .await
    .context("Failed to persist task manifest record")?;

    Ok(AppliedTaskResult { job_id, action })
}

async fn deactivate_missing_manifests(
    pool: &sqlx::SqlitePool,
    root: &Path,
    seen_paths: &HashSet<String>,
    dry_run: bool,
) -> Result<Vec<String>> {
    let root_prefix = root.display().to_string();
    let rows = sqlx::query(
        r#"
        SELECT job_id, manifest_path
        FROM task_manifests
        WHERE manifest_path LIKE ?
        "#,
    )
    .bind(format!("{root_prefix}%"))
    .fetch_all(pool)
    .await?;

    let mut paused = Vec::new();
    for row in rows {
        let path: String = row.get("manifest_path");
        if seen_paths.contains(&path) {
            continue;
        }
        let job_id: String = row.get("job_id");
        paused.push(job_id.clone());
        if !dry_run {
            sqlx::query(
                "UPDATE scheduled_jobs SET state = 'paused' WHERE id = ? AND state != 'completed'",
            )
            .bind(&job_id)
            .execute(pool)
            .await?;
        }
    }

    Ok(paused)
}

async fn fetch_job(pool: &sqlx::SqlitePool, id: &str) -> Result<sqlx::sqlite::SqliteRow> {
    sqlx::query(
        r#"
        SELECT id, name, description, workflow_id, trigger_type, trigger_config, state,
               timezone, max_retries, priority, source_kind, owner, tags, disabled_until,
               manifest_path, task_notes_path, next_run_at, last_run_at, run_count,
               consecutive_failures, metadata, created_at
        FROM scheduled_jobs
        WHERE id = ? OR name = ?
        "#,
    )
    .bind(id)
    .bind(id)
    .fetch_optional(pool)
    .await?
    .with_context(|| format!("Job '{}' not found", id))
}

fn row_to_task_spec(row: &sqlx::sqlite::SqliteRow) -> Result<TaskSpec> {
    let trigger_type: String = row.get("trigger_type");
    let trigger_config_raw: String = row.get("trigger_config");
    let trigger_config: Value = serde_json::from_str(&trigger_config_raw).with_context(|| {
        format!(
            "Invalid trigger_config JSON for {}",
            row.get::<String, _>("id")
        )
    })?;
    let metadata_raw: String = row.get("metadata");
    let metadata: Value = serde_json::from_str(&metadata_raw)
        .with_context(|| format!("Invalid metadata JSON for {}", row.get::<String, _>("id")))?;
    let tags_raw: String = row.get("tags");
    let tags: Vec<String> = serde_json::from_str(&tags_raw).unwrap_or_default();

    let trigger = match trigger_type.as_str() {
        "interval" => TaskTrigger::Interval {
            every_seconds: trigger_config
                .get("interval_secs")
                .or_else(|| trigger_config.get("interval_seconds"))
                .and_then(Value::as_u64)
                .unwrap_or(3600),
        },
        "absolute" => TaskTrigger::Absolute {
            at: trigger_config
                .get("run_at")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        },
        "event" => TaskTrigger::Event {
            event_name: trigger_config
                .get("event_name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        },
        "dependency" => TaskTrigger::Dependency {
            depends_on: trigger_config
                .get("depends_on")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|value| value.as_str().map(ToString::to_string))
                .collect(),
        },
        other => anyhow::bail!("Unsupported trigger_type for export: {}", other),
    };

    let mut passthrough_metadata = metadata.clone();
    if let Some(object) = passthrough_metadata.as_object_mut() {
        object.remove("input");
        object.remove("task");
        object.remove("delivery_policy");
        object.remove("hook_policy");
        object.remove("routing");
    }

    Ok(TaskSpec {
        id: Some(row.get("id")),
        name: row.get("name"),
        workflow: row.get("workflow_id"),
        description: row.try_get("description").ok(),
        notes: None,
        priority: row.try_get("priority").unwrap_or(100i64),
        enabled: row.get::<String, _>("state") == "active",
        timezone: row.get("timezone"),
        owner: row.try_get("owner").ok(),
        tags,
        max_retries: row
            .try_get::<i64, _>("max_retries")
            .ok()
            .map(|value| value as u32),
        disabled_until: row.try_get("disabled_until").ok(),
        trigger,
        payload: metadata
            .get("input")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({})),
        metadata: passthrough_metadata,
        delivery_policy: metadata.get("delivery_policy").cloned(),
        hook_policy: metadata.get("hook_policy").cloned(),
        routing: metadata.get("routing").cloned(),
    })
}

/// List scheduled jobs/tasks.
pub async fn list() -> Result<()> {
    let pool = open_schedule_pool().await?;
    let rows = sqlx::query(
        r#"
        SELECT
            id, name, description, workflow_id, state, trigger_type, next_run_at, last_run_at,
            run_count, consecutive_failures, created_at, priority, source_kind, owner,
            tags, manifest_path, disabled_until
        FROM scheduled_jobs
        ORDER BY
            CASE state
                WHEN 'active' THEN 1
                WHEN 'paused' THEN 2
                ELSE 3
            END,
            priority ASC,
            next_run_at
        "#,
    )
    .fetch_all(&pool)
    .await
    .context("Failed to query scheduled jobs")?;

    if rows.is_empty() {
        println!("No scheduled jobs found.");
        println!();
        println!("Task manifests live under ./{DEFAULT_TASKS_DIR}");
        println!("To create a scheduled job:");
        println!("  openrustclaw schedule create --name <name> --workflow <workflow>");
        println!("To sync task manifests:");
        println!("  openrustclaw schedule sync");
        return Ok(());
    }

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║                 Scheduled Tasks                          ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    for row in rows {
        let id: String = row.get("id");
        let state: String = row.get("state");
        let state_icon = match state.as_str() {
            "active" => "\x1b[32m●\x1b[0m",
            "paused" => "\x1b[33m⏸\x1b[0m",
            "completed" => "\x1b[90m✓\x1b[0m",
            "failed" => "\x1b[31m✗\x1b[0m",
            "dead_letter" => "\x1b[31m☠\x1b[0m",
            _ => "\x1b[90m?\x1b[0m",
        };

        println!(
            "{} {} ({}) [priority={}]",
            state_icon,
            row.get::<String, _>("name"),
            &id[..8],
            row.get::<i64, _>("priority")
        );
        if let Some(desc) = row.get::<Option<String>, _>("description") {
            println!("  {}", desc);
        }
        println!("  Workflow: {}", row.get::<String, _>("workflow_id"));
        println!(
            "  Trigger: {}  Source: {}",
            row.get::<String, _>("trigger_type"),
            row.get::<String, _>("source_kind")
        );
        if let Some(owner) = row.get::<Option<String>, _>("owner") {
            println!("  Owner: {}", owner);
        }
        let tags: String = row.get("tags");
        if tags != "[]" {
            println!("  Tags: {}", tags);
        }
        if let Some(path) = row.get::<Option<String>, _>("manifest_path") {
            println!("  Manifest: {}", path);
        }
        if let Some(until) = row.get::<Option<String>, _>("disabled_until") {
            println!("  Disabled until: {}", until);
        }
        println!(
            "  Runs: {} ({} failures)",
            row.get::<i64, _>("run_count"),
            row.get::<i64, _>("consecutive_failures")
        );
        println!(
            "  Next run: {}",
            row.get::<Option<String>, _>("next_run_at")
                .unwrap_or_else(|| "Not scheduled".to_string())
        );
        if let Some(last) = row.get::<Option<String>, _>("last_run_at") {
            println!("  Last run: {}", last);
        }
        println!();
    }

    Ok(())
}

/// Create a scheduled job or import a task manifest.
#[allow(clippy::too_many_arguments)]
pub async fn create(
    name: Option<&str>,
    workflow: Option<&str>,
    description: Option<&str>,
    file: Option<&str>,
    every_seconds: Option<u64>,
    at: Option<&str>,
    payload: Option<&str>,
    priority: i64,
    owner: Option<&str>,
    tags: &[String],
) -> Result<()> {
    let pool = open_schedule_pool().await?;

    if let Some(file) = file {
        let loaded = load_task_manifest(file)?;
        let applied = upsert_task_manifest(&pool, &loaded, false).await?;
        publish_control_event(
            &pool,
            "control.scheduler.task_manifest_applied",
            serde_json::json!({
                "job_id": applied.job_id,
                "manifest_path": loaded.path.display().to_string(),
                "action": applied.action,
            }),
        )
        .await;
        println!(
            "✓ Task manifest '{}' {} as job '{}'",
            loaded.path.display(),
            applied.action,
            applied.job_id
        );
        return Ok(());
    }

    let name = name.context("--name is required when --file is not used")?;
    let workflow = workflow.context("--workflow is required when --file is not used")?;
    let trigger = trigger_from_inline(every_seconds, at)?;
    let (trigger_type, trigger_config, next_run) = trigger_config_and_next_run(&trigger)?;
    let payload_json = match payload {
        Some(raw) => serde_json::from_str::<serde_json::Value>(raw)
            .with_context(|| format!("Invalid --payload JSON: {}", raw))?,
        None => serde_json::json!({}),
    };

    let id = Uuid::new_v4().to_string();
    let idempotency_key = format!("{}:{}", id, Uuid::new_v4());
    let metadata = build_metadata(
        payload_json,
        priority,
        owner,
        tags,
        None,
        "cli",
        serde_json::json!({}),
        None,
        None,
        None,
    );

    sqlx::query(
        r#"
        INSERT INTO scheduled_jobs (
            id, name, description, workflow_id, trigger_type, trigger_config,
            idempotency_key, state, timezone, max_retries, priority, source_kind, owner,
            tags, next_run_at, run_count, metadata, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, 'active', 'UTC', 3, ?, 'cli', ?, ?, ?, 0, ?, datetime('now'))
        "#,
    )
    .bind(&id)
    .bind(name)
    .bind(
        description
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("Auto-created job for workflow {}", workflow)),
    )
    .bind(workflow)
    .bind(&trigger_type)
    .bind(trigger_config.to_string())
    .bind(idempotency_key)
    .bind(priority)
    .bind(owner)
    .bind(serde_json::to_string(tags)?)
    .bind(next_run.map(|value| value.to_rfc3339()))
    .bind(metadata.to_string())
    .execute(&pool)
    .await
    .context("Failed to create scheduled job")?;

    publish_control_event(
        &pool,
        "control.scheduler.job_created",
        serde_json::json!({
            "job_id": id,
            "name": name,
            "workflow_id": workflow,
            "trigger_type": trigger_type,
            "priority": priority,
            "source_kind": "cli",
        }),
    )
    .await;

    println!("✓ Job created successfully");
    println!("  ID: {}", id);
    println!("  Name: {}", name);
    println!("  Workflow: {}", workflow);
    println!("  Priority: {}", priority);
    println!(
        "  Next run: {}",
        next_run
            .map(|value| value.to_rfc3339())
            .unwrap_or_else(|| "Event-triggered".to_string())
    );
    Ok(())
}

/// Sync task manifests into the durable scheduler.
pub async fn sync(path: Option<&str>, dry_run: bool) -> Result<()> {
    let pool = open_schedule_pool().await?;
    let root = resolve_tasks_root(path)?;
    std::fs::create_dir_all(&root).with_context(|| {
        format!(
            "Failed to create task manifest directory '{}'",
            root.display()
        )
    })?;

    let paths = detect_manifest_paths(&root);
    let mut seen = HashSet::new();
    let mut created = 0usize;
    let mut updated = 0usize;
    let mut unchanged = 0usize;

    for path in paths {
        let loaded = load_task_manifest(&path)?;
        let canonical = loaded
            .path
            .canonicalize()
            .unwrap_or_else(|_| loaded.path.clone());
        seen.insert(canonical.display().to_string());
        let applied = upsert_task_manifest(&pool, &loaded, dry_run).await?;
        match applied.action {
            "created" => created += 1,
            "updated" => updated += 1,
            _ => unchanged += 1,
        }
        println!(
            "{} {} -> {}",
            if dry_run { "would-sync" } else { "synced" },
            loaded.path.display(),
            applied.action
        );
    }

    let paused_missing = deactivate_missing_manifests(&pool, &root, &seen, dry_run).await?;
    if !dry_run {
        publish_control_event(
            &pool,
            "control.scheduler.task_manifests_synced",
            serde_json::json!({
                "path": root.display().to_string(),
                "created": created,
                "updated": updated,
                "unchanged": unchanged,
                "paused_missing": paused_missing,
            }),
        )
        .await;
    }

    println!();
    println!("Task manifest root: {}", root.display());
    println!("Created:   {}", created);
    println!("Updated:   {}", updated);
    println!("Unchanged: {}", unchanged);
    println!("Paused missing manifests: {}", paused_missing.len());
    if dry_run {
        println!("Dry run only; no changes were applied.");
    }
    Ok(())
}

/// Initialize the standard task registry folder and starter templates.
pub async fn init(path: Option<&str>) -> Result<()> {
    let root = resolve_tasks_root(path)?;
    let artifacts = root.join("_artifacts");
    std::fs::create_dir_all(&artifacts)
        .with_context(|| format!("Failed to create task registry '{}'", root.display()))?;

    let starter = root.join("reminder-example.yaml");
    if !starter.exists() {
        let example = render_task_manifest(&TaskSpec {
            id: None,
            name: "Daily Reminder".to_string(),
            workflow: "reminder".to_string(),
            description: Some("Example recurring reminder task".to_string()),
            notes: None,
            priority: 100,
            enabled: true,
            timezone: "UTC".to_string(),
            owner: None,
            tags: vec!["example".to_string(), "reminder".to_string()],
            max_retries: Some(3),
            disabled_until: None,
            trigger: TaskTrigger::Interval {
                every_seconds: 3600,
            },
            payload: serde_json::json!({
                "message": "Review today's queued work",
                "delivery_policy": {
                    "mode": "first_success"
                }
            }),
            metadata: serde_json::json!({}),
            delivery_policy: Some(serde_json::json!({
                "mode": "first_success"
            })),
            hook_policy: None,
            routing: None,
        })?;
        std::fs::write(&starter, example)
            .with_context(|| format!("Failed to write starter manifest '{}'", starter.display()))?;
    }

    println!("✓ Initialized task registry at {}", root.display());
    println!("  Starter manifest: {}", starter.display());
    println!(
        "  Sync into SQLite with: openrustclaw schedule sync --path {}",
        root.display()
    );
    Ok(())
}

/// Export a scheduled job as a task manifest.
pub async fn export(id: &str, output: Option<&str>) -> Result<()> {
    let pool = open_schedule_pool().await?;
    let row = fetch_job(&pool, id).await?;
    let spec = row_to_task_spec(&row)?;
    let workspace_root = current_workspace_root()?;
    let output_path = if let Some(path) = output {
        PathBuf::from(path)
    } else if let Some(path) = row.get::<Option<String>, _>("manifest_path") {
        PathBuf::from(path)
    } else {
        default_export_path(&workspace_root, &spec.name)
    };

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create export directory '{}'", parent.display()))?;
    }

    let yaml = render_task_manifest(&spec)?;
    std::fs::write(&output_path, yaml)
        .with_context(|| format!("Failed to write task manifest '{}'", output_path.display()))?;

    let canonical = output_path
        .canonicalize()
        .unwrap_or_else(|_| output_path.clone());
    let raw = std::fs::read_to_string(&output_path)?;
    let loaded = LoadedTaskManifest {
        path: canonical.clone(),
        hash: {
            let loaded = load_task_manifest(&canonical)?;
            loaded.hash
        },
        manifest: TaskManifest {
            version: 1,
            task: spec.clone(),
        },
        raw,
    };

    upsert_task_manifest(&pool, &loaded, false).await?;

    println!("✓ Exported task manifest to {}", output_path.display());
    Ok(())
}

/// Inspect a single task/job.
pub async fn inspect(id: &str) -> Result<()> {
    let pool = open_schedule_pool().await?;
    let row = fetch_job(&pool, id).await?;
    let latest_run = sqlx::query(
        r#"
        SELECT id, status, started_at, completed_at, retry_count, langsmith_trace_id
        FROM job_runs
        WHERE job_id = ?
        ORDER BY started_at DESC
        LIMIT 1
        "#,
    )
    .bind(row.get::<String, _>("id"))
    .fetch_optional(&pool)
    .await?;
    let checkpoint_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow_checkpoints WHERE workflow_id = ?")
            .bind(row.get::<String, _>("workflow_id"))
            .fetch_one(&pool)
            .await
            .unwrap_or(0);

    println!(
        "Task: {} ({})",
        row.get::<String, _>("name"),
        row.get::<String, _>("id")
    );
    println!("  Workflow: {}", row.get::<String, _>("workflow_id"));
    println!("  State: {}", row.get::<String, _>("state"));
    println!("  Priority: {}", row.get::<i64, _>("priority"));
    println!("  Source: {}", row.get::<String, _>("source_kind"));
    println!(
        "  Manifest: {}",
        row.get::<Option<String>, _>("manifest_path")
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "  Notes: {}",
        row.get::<Option<String>, _>("task_notes_path")
            .unwrap_or_else(|| "-".to_string())
    );
    println!("  Trigger: {}", row.get::<String, _>("trigger_type"));
    println!(
        "  Next run: {}",
        row.get::<Option<String>, _>("next_run_at")
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "  Last run: {}",
        row.get::<Option<String>, _>("last_run_at")
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "  Owner: {}",
        row.get::<Option<String>, _>("owner")
            .unwrap_or_else(|| "-".to_string())
    );
    println!("  Tags: {}", row.get::<String, _>("tags"));
    println!(
        "  Disabled until: {}",
        row.get::<Option<String>, _>("disabled_until")
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "  Description: {}",
        row.get::<Option<String>, _>("description")
            .unwrap_or_default()
    );
    println!("  Checkpoints: {}", checkpoint_count);
    if row.get::<String, _>("trigger_type") == "event" {
        let trigger_config_raw: String = row.get("trigger_config");
        if let Ok(trigger_config) = serde_json::from_str::<Value>(&trigger_config_raw) {
            if let Some(event_name) = trigger_config.get("event_name").and_then(Value::as_str) {
                println!("  Event subscription: {}", event_name);
            }
        }
    }
    if let Some(run) = latest_run {
        println!("  Latest run:");
        println!("    Status: {}", run.get::<String, _>("status"));
        println!("    Started: {}", run.get::<String, _>("started_at"));
        println!(
            "    Completed: {}",
            run.get::<Option<String>, _>("completed_at")
                .unwrap_or_else(|| "-".to_string())
        );
        println!("    Retries: {}", run.get::<i64, _>("retry_count"));
        if let Some(trace) = run.get::<Option<String>, _>("langsmith_trace_id") {
            println!("    Trace: {}", trace);
        }
    }

    Ok(())
}

/// Pause a scheduled job.
pub async fn pause(id: &str) -> Result<()> {
    let pool = open_schedule_pool().await?;
    let job_id: String =
        sqlx::query_scalar("SELECT id FROM scheduled_jobs WHERE id = ? OR name = ?")
            .bind(id)
            .bind(id)
            .fetch_optional(&pool)
            .await?
            .with_context(|| format!("Job '{}' not found", id))?;

    let result =
        sqlx::query("UPDATE scheduled_jobs SET state = 'paused' WHERE id = ? AND state = 'active'")
            .bind(&job_id)
            .execute(&pool)
            .await?;

    if result.rows_affected() == 0 {
        println!("Job '{}' is already paused or not in active state", id);
    } else {
        publish_control_event(
            &pool,
            "control.scheduler.job_paused",
            serde_json::json!({
                "job_id": job_id,
                "requested_by": "cli",
            }),
        )
        .await;
        println!("✓ Job '{}' paused successfully", id);
    }

    Ok(())
}

/// Resume a scheduled job.
pub async fn resume(id: &str) -> Result<()> {
    let pool = open_schedule_pool().await?;
    let row = sqlx::query("SELECT id, next_run_at FROM scheduled_jobs WHERE id = ? OR name = ?")
        .bind(id)
        .bind(id)
        .fetch_optional(&pool)
        .await?
        .with_context(|| format!("Job '{}' not found", id))?;

    let job_id: String = row.get("id");
    let next_run: Option<String> = row.try_get("next_run_at").ok();
    let next_run_at =
        next_run.unwrap_or_else(|| (Utc::now() + chrono::Duration::minutes(1)).to_rfc3339());

    let result = sqlx::query(
        "UPDATE scheduled_jobs SET state = 'active', next_run_at = ?, disabled_until = NULL WHERE id = ? AND state = 'paused'"
    )
    .bind(&next_run_at)
    .bind(&job_id)
    .execute(&pool)
    .await?;

    if result.rows_affected() == 0 {
        println!("Job '{}' is already active or not in paused state", id);
    } else {
        publish_control_event(
            &pool,
            "control.scheduler.job_resumed",
            serde_json::json!({
                "job_id": job_id,
                "requested_by": "cli",
                "next_run_at": next_run_at,
            }),
        )
        .await;
        println!("✓ Job '{}' resumed successfully", id);
    }

    Ok(())
}

/// Force a task to become due immediately.
pub async fn run_now(id: &str) -> Result<()> {
    let pool = open_schedule_pool().await?;
    let result = sqlx::query(
        "UPDATE scheduled_jobs SET state = 'active', disabled_until = NULL, next_run_at = ? WHERE id = ? OR name = ?",
    )
    .bind(Utc::now().to_rfc3339())
    .bind(id)
    .bind(id)
    .execute(&pool)
    .await?;

    if result.rows_affected() == 0 {
        anyhow::bail!("Job '{}' not found", id);
    }

    publish_control_event(
        &pool,
        "control.scheduler.job_run_now",
        serde_json::json!({
            "job_id": id,
            "requested_by": "cli",
        }),
    )
    .await;
    println!("✓ Marked '{}' due to run now", id);
    Ok(())
}

/// Update task priority.
pub async fn reprioritize(id: &str, priority: i64) -> Result<()> {
    let pool = open_schedule_pool().await?;
    let result = sqlx::query("UPDATE scheduled_jobs SET priority = ? WHERE id = ? OR name = ?")
        .bind(priority)
        .bind(id)
        .bind(id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        anyhow::bail!("Job '{}' not found", id);
    }

    publish_control_event(
        &pool,
        "control.scheduler.job_reprioritized",
        serde_json::json!({
            "job_id": id,
            "priority": priority,
            "requested_by": "cli",
        }),
    )
    .await;
    println!("✓ Updated '{}' priority to {}", id, priority);
    Ok(())
}

/// Rebind a task to a different workflow target.
pub async fn rebind(id: &str, workflow: &str) -> Result<()> {
    let pool = open_schedule_pool().await?;
    let result = sqlx::query("UPDATE scheduled_jobs SET workflow_id = ? WHERE id = ? OR name = ?")
        .bind(workflow)
        .bind(id)
        .bind(id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        anyhow::bail!("Job '{}' not found", id);
    }

    publish_control_event(
        &pool,
        "control.scheduler.job_rebound",
        serde_json::json!({
            "job_id": id,
            "workflow_id": workflow,
            "requested_by": "cli",
        }),
    )
    .await;

    println!("✓ Rebound '{}' to workflow '{}'", id, workflow);
    Ok(())
}

/// Disable task until timestamp.
pub async fn disable_until(id: &str, until: &str) -> Result<()> {
    let pool = open_schedule_pool().await?;
    let until = disabled_until_utc(Some(until))?
        .with_context(|| "disable_until requires a timestamp".to_string())?;

    let result = sqlx::query(
        "UPDATE scheduled_jobs SET disabled_until = ?, state = 'active' WHERE id = ? OR name = ?",
    )
    .bind(until.to_rfc3339())
    .bind(id)
    .bind(id)
    .execute(&pool)
    .await?;

    if result.rows_affected() == 0 {
        anyhow::bail!("Job '{}' not found", id);
    }

    publish_control_event(
        &pool,
        "control.scheduler.job_disabled_until",
        serde_json::json!({
            "job_id": id,
            "disabled_until": until.to_rfc3339(),
            "requested_by": "cli",
        }),
    )
    .await;

    println!("✓ Disabled '{}' until {}", id, until.to_rfc3339());
    Ok(())
}

/// Clear disabled-until state.
pub async fn enable(id: &str) -> Result<()> {
    let pool = open_schedule_pool().await?;
    let result =
        sqlx::query("UPDATE scheduled_jobs SET disabled_until = NULL WHERE id = ? OR name = ?")
            .bind(id)
            .bind(id)
            .execute(&pool)
            .await?;

    if result.rows_affected() == 0 {
        anyhow::bail!("Job '{}' not found", id);
    }

    publish_control_event(
        &pool,
        "control.scheduler.job_enabled",
        serde_json::json!({
            "job_id": id,
            "requested_by": "cli",
        }),
    )
    .await;

    println!("✓ Cleared disabled_until for '{}'", id);
    Ok(())
}

/// List recent job attempts.
pub async fn runs(job: Option<&str>, limit: usize) -> Result<()> {
    let pool = open_schedule_pool().await?;

    let rows = if let Some(job) = job {
        sqlx::query(
            r#"
            SELECT r.id, r.job_id, j.name, r.status, r.started_at, r.completed_at,
                   r.retry_count, r.langsmith_trace_id, r.result
            FROM job_runs r
            JOIN scheduled_jobs j ON j.id = r.job_id
            WHERE r.job_id = ? OR j.name = ?
            ORDER BY r.started_at DESC
            LIMIT ?
            "#,
        )
        .bind(job)
        .bind(job)
        .bind(limit as i64)
        .fetch_all(&pool)
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT r.id, r.job_id, j.name, r.status, r.started_at, r.completed_at,
                   r.retry_count, r.langsmith_trace_id, r.result
            FROM job_runs r
            JOIN scheduled_jobs j ON j.id = r.job_id
            ORDER BY r.started_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&pool)
        .await?
    };

    if rows.is_empty() {
        println!("No job runs found.");
        return Ok(());
    }

    for row in rows {
        println!(
            "{} {} [{}] retry={} started={} completed={}",
            row.get::<String, _>("job_id"),
            row.get::<String, _>("name"),
            row.get::<String, _>("status"),
            row.get::<i64, _>("retry_count"),
            row.get::<String, _>("started_at"),
            row.get::<Option<String>, _>("completed_at")
                .unwrap_or_else(|| "-".to_string())
        );
        if let Some(trace_id) = row.get::<Option<String>, _>("langsmith_trace_id") {
            println!("  trace: {}", trace_id);
        }
    }

    Ok(())
}

/// List current dead-letter entries.
pub async fn dead_letters(job: Option<&str>, limit: usize) -> Result<()> {
    let pool = open_schedule_pool().await?;
    let rows = if let Some(job) = job {
        sqlx::query(
            r#"
            SELECT d.id, d.job_id, j.name, d.last_error, d.failed_at, d.retry_count, d.resolved
            FROM dead_letter_queue d
            JOIN scheduled_jobs j ON j.id = d.job_id
            WHERE d.job_id = ? OR j.name = ?
            ORDER BY d.failed_at DESC
            LIMIT ?
            "#,
        )
        .bind(job)
        .bind(job)
        .bind(limit as i64)
        .fetch_all(&pool)
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT d.id, d.job_id, j.name, d.last_error, d.failed_at, d.retry_count, d.resolved
            FROM dead_letter_queue d
            JOIN scheduled_jobs j ON j.id = d.job_id
            ORDER BY d.failed_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&pool)
        .await?
    };

    if rows.is_empty() {
        println!("No dead-letter entries found.");
        return Ok(());
    }

    for row in rows {
        println!(
            "{} {} [{} retries] resolved={} failed_at={}",
            row.get::<String, _>("id"),
            row.get::<String, _>("name"),
            row.get::<i64, _>("retry_count"),
            row.get::<i64, _>("resolved") == 1,
            row.get::<String, _>("failed_at")
        );
        println!("  error: {}", row.get::<String, _>("last_error"));
    }

    Ok(())
}

/// Replay a dead-letter entry by resetting the target job.
pub async fn replay_dead_letter(id: &str) -> Result<()> {
    let pool = open_schedule_pool().await?;
    let row = sqlx::query(
        r#"
        SELECT id, job_id
        FROM dead_letter_queue
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(&pool)
    .await?;

    let Some(row) = row else {
        anyhow::bail!("Dead-letter entry '{}' not found", id);
    };

    let job_id: String = row.get("job_id");
    sqlx::query(
        r#"
        UPDATE scheduled_jobs
        SET state = 'active',
            consecutive_failures = 0,
            lease_owner = NULL,
            lease_expires_at = NULL,
            next_run_at = COALESCE(next_run_at, ?)
        WHERE id = ?
        "#,
    )
    .bind((Utc::now() + chrono::Duration::minutes(1)).to_rfc3339())
    .bind(&job_id)
    .execute(&pool)
    .await?;

    sqlx::query("UPDATE dead_letter_queue SET resolved = 1, resolved_at = ? WHERE id = ?")
        .bind(Utc::now().to_rfc3339())
        .bind(id)
        .execute(&pool)
        .await?;

    publish_control_event(
        &pool,
        "control.scheduler.dead_letter_replayed",
        serde_json::json!({
            "dead_letter_id": id,
            "job_id": job_id,
            "requested_by": "cli",
        }),
    )
    .await;

    println!("✓ Replayed dead-letter entry '{}' for job '{}'", id, job_id);
    Ok(())
}

/// List recent runtime events.
pub async fn events(name: Option<&str>, limit: usize) -> Result<()> {
    let pool = open_schedule_pool().await?;
    let rows = if let Some(name) = name {
        sqlx::query(
            r#"
            SELECT id, event_name, event_type, session_id, status, created_at, processed_at
            FROM runtime_events
            WHERE event_name = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(name)
        .bind(limit as i64)
        .fetch_all(&pool)
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT id, event_name, event_type, session_id, status, created_at, processed_at
            FROM runtime_events
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&pool)
        .await?
    };

    if rows.is_empty() {
        println!("No runtime events found.");
        return Ok(());
    }

    for row in rows {
        println!(
            "{} {} [{}] status={} session={} created={}",
            row.get::<String, _>("id"),
            row.get::<String, _>("event_name"),
            row.get::<String, _>("event_type"),
            row.get::<String, _>("status"),
            row.get::<Option<String>, _>("session_id")
                .unwrap_or_else(|| "-".to_string()),
            row.get::<String, _>("created_at")
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn resolve_tasks_root_defaults_to_hidden_claw_path() {
        let cwd = current_workspace_root().unwrap();
        assert_eq!(
            resolve_tasks_root(None).unwrap(),
            cwd.join(DEFAULT_TASKS_DIR)
        );
    }

    #[test]
    fn trigger_from_inline_rejects_conflicting_flags() {
        assert!(trigger_from_inline(Some(60), Some("2026-01-01T00:00:00Z")).is_err());
    }

    #[test]
    fn notes_path_is_scoped_under_artifacts_folder() {
        let path = PathBuf::from("/tmp/work/.claw/tasks/daily.yaml");
        assert_eq!(
            notes_path_for_manifest_path(&path),
            PathBuf::from("/tmp/work/.claw/tasks/_artifacts/daily/notes.md")
        );
    }

    #[test]
    fn detect_manifest_paths_filters_non_yaml() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".claw/tasks")).unwrap();
        std::fs::write(dir.path().join(".claw/tasks/a.yaml"), "version: 1\ntask:\n  name: A\n  workflow: agent\n  trigger:\n    type: interval\n    every_seconds: 60\n").unwrap();
        std::fs::write(dir.path().join(".claw/tasks/skip.txt"), "nope").unwrap();

        let found = detect_manifest_paths(&dir.path().join(".claw/tasks"));
        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("a.yaml"));
    }
}
